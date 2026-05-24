//! OpenTelemetry tracing runtime wiring.

use anyhow::{Context, Result};
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use tikee_config::TracingConfig;
use tracing_subscriber::{Registry, layer::SubscriberExt, util::SubscriberInitExt};

/// Running tracing exporter handle.
#[derive(Debug)]
pub struct TracingRuntime {
    provider: Option<SdkTracerProvider>,
}

impl TracingRuntime {
    /// Start tracing subscribers and optional OTLP export.
    ///
    /// # Errors
    ///
    /// Returns an error when OTLP tracing is enabled but exporter configuration is invalid.
    pub fn start(config: &TracingConfig) -> Result<Self> {
        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("tikee=info,tower_http=info"));
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .compact();

        if !config.enabled {
            Registry::default().with(env_filter).with(fmt_layer).init();
            return Ok(Self { provider: None });
        }

        let endpoint = config
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
                config
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
                    .with_service_name("tikee-server")
                    .build(),
            )
            .build();
        let tracer = provider.tracer("tikee-server");
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        global::set_tracer_provider(provider.clone());

        Registry::default()
            .with(env_filter)
            .with(fmt_layer)
            .with(otel_layer)
            .init();

        Ok(Self {
            provider: Some(provider),
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
                "tikee.otel.smoke",
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

    /// Shutdown tracing and flush remaining spans.
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
        Ok(())
    }
}

impl Drop for TracingRuntime {
    fn drop(&mut self) {
        let Some(provider) = self.provider.take() else {
            return;
        };
        if let Ok(handle) = std::thread::Builder::new()
            .name("tikee-otel-shutdown".to_owned())
            .spawn(move || {
                if let Err(error) = provider.shutdown() {
                    tracing::warn!(%error, "failed to shutdown tracing runtime");
                }
            })
        {
            let _ = handle.join();
        }
    }
}
