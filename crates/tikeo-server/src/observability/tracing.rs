//! OpenTelemetry tracing and high-throughput local/remote logging runtime wiring.
//!
//! tikeo uses one process-level subscriber for local diagnostics, optional durable files,
//! optional ELK/log-collector forwarding, and optional OTLP trace export. All non-console sinks
//! are non-blocking: application threads enqueue formatted log lines and dedicated workers flush
//! them to disk or collectors.

use std::{
    io::{self, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{Context, Result};
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use serde_json::json;
use tikeo_config::{ElkLogConfig, FileLogSinkConfig, LoggingConfig, TikeoConfig, TracingConfig};
use tracing::{debug, error, trace, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    Layer, Registry,
    filter::{EnvFilter, LevelFilter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

const ELK_QUEUE_CAPACITY: usize = 65_536;
const ELK_BATCH_MAX_LINES: usize = 512;
const ELK_RECONNECT_BACKOFF: Duration = Duration::from_secs(1);
const ELK_RECV_TIMEOUT: Duration = Duration::from_millis(200);

/// Running tracing and logging handle.
///
/// Keep the value alive for the full server lifetime. Dropping it flushes pending OTLP spans,
/// file-log buffers, and stops ELK forwarding threads.
#[derive(Debug)]
pub struct TracingRuntime {
    provider: Option<SdkTracerProvider>,
    file_log_guards: Vec<WorkerGuard>,
    elk_guard: Option<ElkForwarderGuard>,
}

impl TracingRuntime {
    /// Start tracing subscribers with default local logging and optional OTLP export.
    ///
    /// # Errors
    ///
    /// Returns an error when OTLP tracing is enabled but exporter configuration is invalid.
    pub fn start(config: &TracingConfig) -> Result<Self> {
        Self::start_with_logging(config, &LoggingConfig::default())
    }

    /// Start tracing subscribers from the complete process configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when local file logging or OTLP exporter setup fails.
    pub fn start_from_config(config: &TikeoConfig) -> Result<Self> {
        Self::start_with_logging(&config.observability.tracing, &config.observability.logging)
    }

    /// Start tracing subscribers from explicit tracing and logging configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when local file logging or OTLP exporter setup fails.
    pub fn start_with_logging(tracing: &TracingConfig, logging: &LoggingConfig) -> Result<Self> {
        let env_filter = env_filter(logging);
        let mut file_log_guards = Vec::new();
        let mut layers = Vec::new();

        if logging.console.enabled {
            layers.push(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .compact()
                    .with_filter(level_filter(&logging.console.level))
                    .boxed(),
            );
        }

        if logging.file.enabled {
            let (writer, guard) = file_logging_writer(&logging.file, "tikeo.log")?;
            file_log_guards.push(guard);
            layers.push(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_current_span(false)
                    .with_span_list(false)
                    .with_ansi(false)
                    .with_writer(writer)
                    .with_filter(level_filter(&logging.file.level))
                    .boxed(),
            );
        }

        if logging.error_file.enabled {
            let (writer, guard) = file_logging_writer(&logging.error_file, "tikeo-error.log")?;
            file_log_guards.push(guard);
            layers.push(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_current_span(false)
                    .with_span_list(false)
                    .with_ansi(false)
                    .with_writer(writer)
                    .with_filter(level_filter(&logging.error_file.level))
                    .boxed(),
            );
        }

        let elk_guard = if logging.elk.enabled {
            let forwarder = ElkForwarder::start(&logging.elk);
            layers.push(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_current_span(false)
                    .with_span_list(false)
                    .with_ansi(false)
                    .with_writer(forwarder.writer())
                    .with_filter(level_filter(&logging.elk.level))
                    .boxed(),
            );
            Some(forwarder)
        } else {
            None
        };

        let provider = if tracing.enabled {
            Some(build_tracer_provider(tracing)?)
        } else {
            None
        };
        if let Some(provider) = &provider {
            let tracer = provider.tracer("tikeo-server");
            layers.push(tracing_opentelemetry::layer().with_tracer(tracer).boxed());
        }

        Registry::default().with(env_filter).with(layers).init();

        debug!(
            root_filter = %root_filter(&logging.root.level),
            console_level = %normalize_log_level(&logging.console.level),
            file_level = %normalize_log_level(&logging.file.level),
            error_file_level = %normalize_log_level(&logging.error_file.level),
            elk_level = %normalize_log_level(&logging.elk.level),
            "tikeo logging filters configured"
        );
        trace!(
            file_raw_path = %logging.file.path,
            error_file_raw_path = %logging.error_file.path,
            elk_servers = %logging.elk.servers,
            "tikeo logging sink raw configuration loaded"
        );
        if !logging.console.enabled
            && !logging.file.enabled
            && !logging.error_file.enabled
            && !logging.elk.enabled
        {
            warn!(
                "all logging sinks are disabled; runtime diagnostics will be unavailable unless RUST_LOG subscriber is installed externally"
            );
        }

        tracing::info!(
            root_level = %normalize_log_level(&logging.root.level),
            console_enabled = logging.console.enabled,
            console_level = %normalize_log_level(&logging.console.level),
            file_enabled = logging.file.enabled,
            file_level = %normalize_log_level(&logging.file.level),
            file_path = %logging.file.path,
            error_file_enabled = logging.error_file.enabled,
            error_file_level = %normalize_log_level(&logging.error_file.level),
            error_file_path = %logging.error_file.path,
            elk_enabled = logging.elk.enabled,
            elk_topic = %logging.elk.topic,
            otlp_endpoint_configured = provider.is_some(),
            "tikeo logging runtime initialized"
        );

        Ok(Self {
            provider,
            file_log_guards,
            elk_guard,
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
        drop(self.elk_guard.take());
        self.file_log_guards.clear();
        Ok(())
    }
}

impl Drop for TracingRuntime {
    fn drop(&mut self) {
        let provider = self.provider.take();
        drop(self.elk_guard.take());
        self.file_log_guards.clear();
        if let Some(provider) = provider
            && let Ok(handle) = thread::Builder::new()
                .name("tikeo-otel-shutdown".to_owned())
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

fn env_filter(logging: &LoggingConfig) -> EnvFilter {
    let default_filter = root_filter(&logging.root.level);
    let env_filter = std::env::var("RUST_LOG")
        .ok()
        .map_or(default_filter, |value| scoped_rust_log_filter(&value));
    EnvFilter::try_new(&env_filter)
        .unwrap_or_else(|_| EnvFilter::new(root_filter(&logging.root.level)))
}

fn scoped_rust_log_filter(rust_log: &str) -> String {
    let mut filter = rust_log.trim().to_owned();
    if filter.is_empty() {
        return root_filter("info");
    }
    if !env_flag_enabled("TIKEO_SQL_VERBOSE_VALUES", false) {
        filter.push_str(",sea_orm=off");
    }
    filter
}

fn build_tracer_provider(tracing: &TracingConfig) -> Result<SdkTracerProvider> {
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
    Ok(provider)
}

fn root_filter(level: &str) -> String {
    let level = normalize_log_level(level);
    let sea_orm_level = if env_flag_enabled("TIKEO_SQL_VERBOSE_VALUES", false) {
        level
    } else {
        "off"
    };
    format!(
        "tikeo={level},tikeo_server={level},tikeo_storage={level},tikeo_config={level},sqlx={level},sea_orm={sea_orm_level},tower_http={level},tokio={level},hyper={level},tonic={level}"
    )
}

fn env_flag_enabled(name: &str, default: bool) -> bool {
    std::env::var(name).map_or(default, |value| {
        let value = value.trim();
        !(value == "0" || value.eq_ignore_ascii_case("false") || value.eq_ignore_ascii_case("off"))
    })
}

fn file_logging_writer(
    config: &FileLogSinkConfig,
    default_file_name: &str,
) -> Result<(
    tracing_appender::non_blocking::NonBlocking,
    tracing_appender::non_blocking::WorkerGuard,
)> {
    let (directory, file_name) = log_directory_and_file(&config.path, default_file_name);
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("failed to create log directory: {}", directory.display()))?;
    debug!(directory = %directory.display(), file_name = %file_name, "configured non-blocking file log sink");
    let appender = tracing_appender::rolling::never(directory, file_name);
    Ok(tracing_appender::non_blocking(appender))
}

fn log_directory_and_file(path: &str, default_file_name: &str) -> (PathBuf, String) {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return (PathBuf::from("/logs"), default_file_name.to_owned());
    }
    let path = Path::new(trimmed);
    if path.extension().is_some() {
        let directory = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(default_file_name)
            .to_owned();
        (directory, file_name)
    } else {
        (path.to_path_buf(), default_file_name.to_owned())
    }
}

fn normalize_log_level(level: &str) -> &'static str {
    match level.trim().to_ascii_lowercase().as_str() {
        "trace" => "trace",
        "debug" => "debug",
        "warn" | "warning" => "warn",
        "error" => "error",
        _ => "info",
    }
}

fn level_filter(level: &str) -> LevelFilter {
    match normalize_log_level(level) {
        "trace" => LevelFilter::TRACE,
        "debug" => LevelFilter::DEBUG,
        "warn" => LevelFilter::WARN,
        "error" => LevelFilter::ERROR,
        _ => LevelFilter::INFO,
    }
}

#[derive(Debug)]
struct ElkForwarderGuard {
    stop: Arc<AtomicBool>,
    sender: SyncSender<Vec<u8>>,
    handle: Option<JoinHandle<()>>,
}

impl ElkForwarderGuard {
    fn writer(&self) -> ElkMakeWriter {
        ElkMakeWriter {
            sender: self.sender.clone(),
        }
    }
}

impl Drop for ElkForwarderGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.sender.try_send(Vec::new());
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

struct ElkForwarder;

impl ElkForwarder {
    fn start(config: &ElkLogConfig) -> ElkForwarderGuard {
        let (sender, receiver) = mpsc::sync_channel(ELK_QUEUE_CAPACITY);
        let stop = Arc::new(AtomicBool::new(false));
        let worker = ElkWorkerConfig::from(config);
        let worker_stop = Arc::clone(&stop);
        let handle = match thread::Builder::new()
            .name("tikeo-elk-log-forwarder".to_owned())
            .spawn(move || run_elk_forwarder(&receiver, &worker_stop, &worker))
        {
            Ok(handle) => Some(handle),
            Err(error) => {
                error!(%error, "failed to start elk log forwarder thread");
                None
            }
        };
        ElkForwarderGuard {
            stop,
            sender,
            handle,
        }
    }
}

#[derive(Debug, Clone)]
struct ElkWorkerConfig {
    servers: Vec<String>,
    topic: String,
    sasl_enabled: bool,
    sasl_username: String,
}

impl From<&ElkLogConfig> for ElkWorkerConfig {
    fn from(config: &ElkLogConfig) -> Self {
        Self {
            servers: config
                .servers
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            topic: config.topic.clone(),
            sasl_enabled: config.sasl.enabled,
            sasl_username: config.sasl.username.clone(),
        }
    }
}

fn run_elk_forwarder(receiver: &Receiver<Vec<u8>>, stop: &AtomicBool, config: &ElkWorkerConfig) {
    if config.servers.is_empty() {
        warn!("elk log forwarding is enabled but no servers are configured");
        return;
    }
    debug!(server_count = config.servers.len(), topic = %config.topic, sasl_enabled = config.sasl_enabled, "starting elk log forwarder worker");
    let mut server_index = 0_usize;
    let mut stream = None;
    let mut batch = Vec::with_capacity(ELK_BATCH_MAX_LINES * 256);
    let mut batch_line_count = 0_usize;
    while !stop.load(Ordering::Relaxed) {
        match receiver.recv_timeout(ELK_RECV_TIMEOUT) {
            Ok(line) if line.is_empty() && stop.load(Ordering::Relaxed) => break,
            Ok(line) if !line.is_empty() => {
                append_elk_frame(&mut batch, &line, config);
                batch_line_count = batch_line_count.saturating_add(1);
                for line in receiver.try_iter().take(ELK_BATCH_MAX_LINES - 1) {
                    if !line.is_empty() {
                        append_elk_frame(&mut batch, &line, config);
                        batch_line_count = batch_line_count.saturating_add(1);
                    }
                }
                if flush_elk_batch(&mut stream, &mut server_index, &config.servers, &batch) {
                    trace!(
                        line_count = batch_line_count,
                        byte_count = batch.len(),
                        "flushed elk log batch"
                    );
                } else {
                    warn!(
                        line_count = batch_line_count,
                        server_count = config.servers.len(),
                        "failed to flush elk log batch; backing off"
                    );
                    thread::sleep(ELK_RECONNECT_BACKOFF);
                }
                batch.clear();
                batch_line_count = 0;
            }
            Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    while let Ok(line) = receiver.try_recv() {
        if !line.is_empty() {
            append_elk_frame(&mut batch, &line, config);
            batch_line_count = batch_line_count.saturating_add(1);
        }
        if batch.len() >= ELK_BATCH_MAX_LINES * 256 {
            let _ = flush_elk_batch(&mut stream, &mut server_index, &config.servers, &batch);
            batch.clear();
            batch_line_count = 0;
        }
    }
    if !batch.is_empty() {
        if flush_elk_batch(&mut stream, &mut server_index, &config.servers, &batch) {
            debug!(
                line_count = batch_line_count,
                byte_count = batch.len(),
                "flushed final elk log batch during shutdown"
            );
        } else {
            error!(
                line_count = batch_line_count,
                byte_count = batch.len(),
                "failed to flush final elk log batch during shutdown"
            );
        }
    }
    debug!("elk log forwarder worker stopped");
}

fn append_elk_frame(batch: &mut Vec<u8>, line: &[u8], config: &ElkWorkerConfig) {
    let payload = String::from_utf8_lossy(line).trim().to_owned();
    let frame = json!({
        "topic": config.topic,
        "saslEnabled": config.sasl_enabled,
        "saslUsername": config.sasl_username,
        "payload": payload,
    });
    if serde_json::to_writer(&mut *batch, &frame).is_ok() {
        batch.push(b'\n');
    }
}

fn flush_elk_batch(
    stream: &mut Option<TcpStream>,
    server_index: &mut usize,
    servers: &[String],
    batch: &[u8],
) -> bool {
    if batch.is_empty() {
        return true;
    }
    for _ in 0..=servers.len() {
        if stream.is_none() {
            let server = &servers[*server_index % servers.len()];
            *server_index = server_index.saturating_add(1);
            match TcpStream::connect(server) {
                Ok(next) => {
                    trace!(%server, "connected elk log forwarder stream");
                    let _ = next.set_nodelay(true);
                    let _ = next.set_write_timeout(Some(Duration::from_secs(2)));
                    *stream = Some(next);
                }
                Err(error) => {
                    debug!(%server, %error, "failed to connect elk log forwarder stream");
                    continue;
                }
            }
        }
        if let Some(current) = stream.as_mut() {
            if let Err(error) = current.write_all(batch) {
                debug!(%error, "elk log forwarder stream write failed");
                *stream = None;
            } else {
                return true;
            }
        }
    }
    false
}

#[derive(Clone, Debug)]
struct ElkMakeWriter {
    sender: SyncSender<Vec<u8>>,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for ElkMakeWriter {
    type Writer = ElkWriter;

    fn make_writer(&'a self) -> Self::Writer {
        ElkWriter {
            sender: self.sender.clone(),
        }
    }
}

#[derive(Debug)]
struct ElkWriter {
    sender: SyncSender<Vec<u8>>,
}

impl Write for ElkWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !buf.is_empty() {
            match self.sender.try_send(buf.to_vec()) {
                Ok(()) => {}
                Err(TrySendError::Full(_)) => trace!("elk log queue full; dropping log frame"),
                Err(TrySendError::Disconnected(_)) => {
                    trace!("elk log queue disconnected; dropping log frame");
                }
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
