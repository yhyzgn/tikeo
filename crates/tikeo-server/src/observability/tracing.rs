//! OpenTelemetry tracing and local logging runtime wiring.
//!
//! tikeo uses one process-level subscriber for both local diagnostics and optional OTLP export.
//! Console logging is always enabled and defaults to `INFO`; operators can set `RUST_LOG` for
//! temporary overrides. When `observability.logging.log_dir` is configured, the same events are
//! also written to `tikeo.log` so container, bare-metal, and systemd deployments have a durable
//! troubleshooting trail.

use std::path::Path;

use anyhow::{Context, Result};
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use tikeo_config::{LoggingConfig, ObservabilityConfig, TracingConfig};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{Registry, layer::SubscriberExt, util::SubscriberInitExt};

/// Running tracing and local logging handle.
///
/// Keep the value alive for the full server lifetime. Dropping it flushes pending OTLP spans and
/// file-log buffers. Do not create multiple runtimes in the same process unless tests isolate the
/// global tracing subscriber.
#[derive(Debug)]
pub struct TracingRuntime {
    provider: Option<SdkTracerProvider>,
    file_log_guard: Option<WorkerGuard>,
}

impl TracingRuntime {
    /// Start tracing subscribers with default local logging and optional OTLP export.
    ///
    /// # Errors
    ///
    /// Returns an error when OTLP tracing is enabled but exporter configuration is invalid, or when
    /// the configured log directory cannot be created.
    pub fn start(config: &TracingConfig) -> Result<Self> {
        Self::start_with_logging(config, &LoggingConfig::default())
    }

    /// Start tracing subscribers from the complete observability configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when local file logging or OTLP exporter setup fails.
    pub fn start_observability(config: &ObservabilityConfig) -> Result<Self> {
        Self::start_with_logging(&config.tracing, &config.logging)
    }

    fn start_with_logging(tracing: &TracingConfig, logging: &LoggingConfig) -> Result<Self> {
        let env_filter =
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new(format!(
                    "tikeo={},tower_http={}",
                    normalize_log_level(&logging.level),
                    normalize_log_level(&logging.level)
                ))
            });
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .compact();
        let (file_writer, file_log_guard) = file_logging_writer(logging)?;

        if !tracing.enabled {
            if let Some(file_writer) = file_writer {
                let file_layer = tracing_subscriber::fmt::layer()
                    .with_target(true)
                    .with_ansi(false)
                    .with_writer(file_writer);
                Registry::default()
                    .with(env_filter)
                    .with(fmt_layer)
                    .with(file_layer)
                    .init();
            } else {
                Registry::default().with(env_filter).with(fmt_layer).init();
            }
            tracing::info!(level = %normalize_log_level(&logging.level), log_dir = ?logging.log_dir, "tikeo local logging initialized");
            return Ok(Self {
                provider: None,
                file_log_guard,
            });
        }

        let endpoint = tracing
            .otlp_endpoint
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .context("observability.tracing.otlp_endpoint is required when tracing is enabled")?;
        let exporter = SpanExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .with_endpoint(endpoint)
            .with_headers(
                tracing
                    .headers
                    .iter()
                    .filter_map(|name| {
                        let name = name.trim();
                        (!name.is_empty()).then(|| (name.to_owned(), "configured".to_owned()))
                    })
                    .collect(),
            )
            .build()
            .context("failed to build OTLP span exporter")?;
        let provider = SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(
                Resource::builder()
                    .with_service_name("tikeo-server")
                    .build(),
            )
            .build();
        global::set_tracer_provider(provider.clone());

        if let Some(file_writer) = file_writer {
            let tracer = provider.tracer("tikeo-server");
            let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
            let file_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_ansi(false)
                .with_writer(file_writer);
            Registry::default()
                .with(env_filter)
                .with(fmt_layer)
                .with(file_layer)
                .with(otel_layer)
                .init();
        } else {
            let tracer = provider.tracer("tikeo-server");
            let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
            Registry::default()
                .with(env_filter)
                .with(fmt_layer)
                .with(otel_layer)
                .init();
        }

        tracing::info!(level = %normalize_log_level(&logging.level), log_dir = ?logging.log_dir, otlp_endpoint_configured = true, "tikeo local logging and tracing initialized");
        Ok(Self {
            provider: Some(provider),
            file_log_guard,
        })
    }

    /// Emit a deterministic smoke span and force-flush it.
    ///
    /// # Errors
    ///
    /// Returns an error when the exporter cannot flush the span.
    pub fn emit_smoke_span(&self, name: &'static str) -> Result<()> {
        {
            let span = tracing::info_span!(
                "tikeo.otel.smoke",
                component = "otel-smoke",
                smoke_name = name
            );
            let guard = span.enter();
            tracing::info!(event = "otel_smoke", "emitting OpenTelemetry smoke span");
            drop(guard);
            drop(span);
        }
        if let Some(provider) = &self.provider {
            provider
                .force_flush()
                .context("failed to flush OTLP spans")?;
        }
        Ok(())
    }

    /// Shutdown tracing and flush remaining spans and file logs.
    ///
    /// # Errors
    ///
    /// Returns an error when the SDK reports a shutdown failure.
    pub fn shutdown(&mut self) -> Result<()> {
        if let Some(provider) = self.provider.take() {
            provider
                .shutdown()
                .context("failed to shutdown OTLP tracer provider")?;
        }
        drop(self.file_log_guard.take());
        Ok(())
    }
}

impl Drop for TracingRuntime {
    fn drop(&mut self) {
        let Some(provider) = self.provider.take() else {
            drop(self.file_log_guard.take());
            return;
        };
        if let Ok(handle) = std::thread::Builder::new()
            .name("tikeo-otel-shutdown".to_owned())
            .spawn(move || {
                if let Err(error) = provider.shutdown() {
                    tracing::warn!(%error, "failed to shutdown tracing runtime");
                }
            })
        {
            let _ = handle.join();
        }
        drop(self.file_log_guard.take());
    }
}

fn file_logging_writer(
    logging: &LoggingConfig,
) -> Result<(
    Option<tracing_appender::non_blocking::NonBlocking>,
    Option<WorkerGuard>,
)> {
    let Some(log_dir) = logging
        .log_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok((None, None));
    };
    std::fs::create_dir_all(log_dir)
        .with_context(|| format!("failed to create observability.logging.log_dir: {log_dir}"))?;
    let appender = tracing_appender::rolling::never(Path::new(log_dir), "tikeo.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    Ok((Some(writer), Some(guard)))
}

fn normalize_log_level(level: &str) -> &'static str {
    match level.trim().to_ascii_lowercase().as_str() {
        "debug" => "debug",
        "warn" | "warning" => "warn",
        "error" => "error",
        _ => "info",
    }
}
