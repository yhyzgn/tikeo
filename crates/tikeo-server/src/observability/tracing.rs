//! OpenTelemetry tracing and high-throughput local/remote logging runtime wiring.
//!
//! tikeo uses one process-level subscriber for local diagnostics, optional durable files,
//! optional ELK/log-collector forwarding, and optional OTLP trace export. All non-console sinks
//! are non-blocking: application threads enqueue formatted log lines and dedicated workers flush
//! them to disk or collectors.

use std::{
    collections::BTreeMap,
    fmt,
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
use serde_json::{Map, Value};
use tikeo_config::{ElkLogConfig, FileLogSinkConfig, LoggingConfig, TikeoConfig, TracingConfig};
use tracing::{Event, Subscriber, debug, error, field::Visit, trace, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    Layer, Registry,
    filter::{EnvFilter, LevelFilter},
    fmt::{FmtContext, FormatEvent, FormatFields, format::Writer},
    layer::SubscriberExt,
    registry::LookupSpan,
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

        if logging.channels.console.enabled {
            layers.push(
                tracing_subscriber::fmt::layer()
                    .event_format(DirectTextFormatter)
                    .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
                    .with_filter(level_filter(&logging.channels.console.level))
                    .boxed(),
            );
        }

        if logging.channels.file.enabled {
            let (writer, guard) = file_logging_writer(&logging.channels.file, "tikeo.log")?;
            file_log_guards.push(guard);
            layers.push(
                tracing_subscriber::fmt::layer()
                    .event_format(DirectJsonFormatter)
                    .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
                    .with_ansi(false)
                    .with_writer(writer)
                    .with_filter(level_filter(&logging.channels.file.level))
                    .boxed(),
            );
        }

        if logging.channels.error_file.enabled {
            let (writer, guard) =
                file_logging_writer(&logging.channels.error_file, "tikeo-error.log")?;
            file_log_guards.push(guard);
            layers.push(
                tracing_subscriber::fmt::layer()
                    .event_format(DirectJsonFormatter)
                    .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
                    .with_ansi(false)
                    .with_writer(writer)
                    .with_filter(level_filter(&logging.channels.error_file.level))
                    .boxed(),
            );
        }

        let elk_guard = if logging.channels.elk.enabled {
            let forwarder = ElkForwarder::start(&logging.channels.elk);
            layers.push(
                tracing_subscriber::fmt::layer()
                    .event_format(ElkJsonFormatter::new())
                    .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
                    .with_ansi(false)
                    .with_writer(forwarder.writer())
                    .with_filter(level_filter(&logging.channels.elk.level))
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
            console_level = %normalize_log_level(&logging.channels.console.level),
            file_level = %normalize_log_level(&logging.channels.file.level),
            error_file_level = %normalize_log_level(&logging.channels.error_file.level),
            elk_level = %normalize_log_level(&logging.channels.elk.level),
            "tikeo logging filters configured"
        );
        trace!(
            file_raw_path = %logging.channels.file.path,
            error_file_raw_path = %logging.channels.error_file.path,
            elk_servers = %logging.channels.elk.servers,
            "tikeo logging sink raw configuration loaded"
        );
        if !logging.channels.console.enabled
            && !logging.channels.file.enabled
            && !logging.channels.error_file.enabled
            && !logging.channels.elk.enabled
        {
            warn!(
                "all logging sinks are disabled; runtime diagnostics will be unavailable unless RUST_LOG subscriber is installed externally"
            );
        }

        tracing::info!(
            root_level = %normalize_log_level(&logging.root.level),
            console_enabled = logging.channels.console.enabled,
            console_level = %normalize_log_level(&logging.channels.console.level),
            file_enabled = logging.channels.file.enabled,
            file_level = %normalize_log_level(&logging.channels.file.level),
            file_path = %logging.channels.file.path,
            error_file_enabled = logging.channels.error_file.enabled,
            error_file_level = %normalize_log_level(&logging.channels.error_file.level),
            error_file_path = %logging.channels.error_file.path,
            elk_enabled = logging.channels.elk.enabled,
            elk_topic = %logging.channels.elk.topic,
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

#[derive(Debug, Clone, Copy)]
struct DirectTextFormatter;

impl<S, N> FormatEvent<S, N> for DirectTextFormatter
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let captured = CapturedFields::from_event(event);
        let metadata = event.metadata();
        let category = LogCategory::from_target(metadata.target());
        write!(
            writer,
            "\u{001b}[2m{}\u{001b}[0m {} {} \u{001b}[2m{}\u{001b}[0m {}",
            current_datetime(),
            ansi_level(*metadata.level()),
            category.ansi_label(),
            metadata.target(),
            captured.message_with_fields()
        )?;
        writeln!(writer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogCategory {
    Http,
    Sql,
    App,
}

impl LogCategory {
    fn from_target(target: &str) -> Self {
        if target == "sqlx::query" || target.starts_with("sea_orm") {
            Self::Sql
        } else if target.contains("http") || target.contains("tower_http") {
            Self::Http
        } else {
            Self::App
        }
    }

    const fn ansi_label(self) -> &'static str {
        match self {
            Self::Http => "\u{001b}[36;1m[HTTP]\u{001b}[0m",
            Self::Sql => "\u{001b}[35;1m[SQL ]\u{001b}[0m",
            Self::App => "\u{001b}[37;1m[APP ]\u{001b}[0m",
        }
    }
}

const fn ansi_level(level: tracing::Level) -> &'static str {
    match level {
        tracing::Level::TRACE => "\u{001b}[35mTRACE\u{001b}[0m",
        tracing::Level::DEBUG => "\u{001b}[34mDEBUG\u{001b}[0m",
        tracing::Level::INFO => "\u{001b}[32mINFO \u{001b}[0m",
        tracing::Level::WARN => "\u{001b}[33mWARN \u{001b}[0m",
        tracing::Level::ERROR => "\u{001b}[31;1mERROR\u{001b}[0m",
    }
}

#[derive(Debug, Clone, Copy)]
struct DirectJsonFormatter;

impl<S, N> FormatEvent<S, N> for DirectJsonFormatter
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let captured = CapturedFields::from_event(event);
        let metadata = event.metadata();
        let mut object = Map::new();
        object.insert("timestamp".to_owned(), Value::String(current_datetime()));
        object.insert(
            "level".to_owned(),
            Value::String(metadata.level().to_string()),
        );
        object.insert(
            "target".to_owned(),
            Value::String(metadata.target().to_owned()),
        );
        object.insert("message".to_owned(), Value::String(captured.message()));
        let fields = captured.fields_json();
        if !fields.is_empty() {
            object.insert("fields".to_owned(), Value::Object(fields));
        }
        write_json_line(&mut writer, &Value::Object(object))
    }
}

#[derive(Debug, Clone)]
struct ElkJsonFormatter {
    hostname: Option<String>,
}

impl ElkJsonFormatter {
    fn new() -> Self {
        Self {
            hostname: local_hostname(),
        }
    }
}

impl<S, N> FormatEvent<S, N> for ElkJsonFormatter
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let captured = CapturedFields::from_event(event);
        let metadata = event.metadata();
        let mut object = Map::new();
        object.insert("app".to_owned(), Value::String("tikeo-server".to_owned()));
        object.insert("ip".to_owned(), Value::Null);
        object.insert(
            "hostname".to_owned(),
            self.hostname.clone().map_or(Value::Null, Value::String),
        );
        object.insert(
            "class".to_owned(),
            Value::String(metadata.target().to_owned()),
        );
        object.insert(
            "file".to_owned(),
            Value::String(metadata.file().unwrap_or_default().to_owned()),
        );
        object.insert(
            "method".to_owned(),
            Value::String(metadata.module_path().unwrap_or_default().to_owned()),
        );
        object.insert(
            "line".to_owned(),
            Value::String(
                metadata
                    .line()
                    .map_or_else(String::new, |line| line.to_string()),
            ),
        );
        object.insert("datetime".to_owned(), Value::String(current_datetime()));
        object.insert("thread".to_owned(), Value::String(current_thread_name()));
        object.insert(
            "level".to_owned(),
            Value::String(metadata.level().to_string()),
        );
        object.insert("trace_id".to_owned(), Value::String(captured.trace_id()));
        object.insert(
            "msg".to_owned(),
            Value::String(captured.message_with_fields()),
        );
        object.insert("exception".to_owned(), Value::String(captured.exception()));
        write_json_line(&mut writer, &Value::Object(object))
    }
}

#[derive(Debug, Default)]
struct CapturedFields {
    values: BTreeMap<String, Value>,
    message: Option<String>,
    summary: Option<String>,
    db_statement: Option<String>,
    trace_id: Option<String>,
    exception: Option<String>,
}

impl CapturedFields {
    fn from_event(event: &Event<'_>) -> Self {
        let mut captured = Self::default();
        event.record(&mut captured);
        captured
    }

    fn message(&self) -> String {
        self.db_statement
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .or(self.message.as_deref())
            .or(self.summary.as_deref())
            .unwrap_or_default()
            .to_owned()
    }

    fn message_with_fields(&self) -> String {
        let mut parts = Vec::new();
        let message = self.message();
        if !message.is_empty() {
            parts.push(message);
        }
        parts.extend(
            self.loggable_fields()
                .map(|(key, value)| format!("{key}={}", json_value_to_log_text(value))),
        );
        parts.join(" | ")
    }

    fn fields_json(&self) -> Map<String, Value> {
        self.loggable_fields()
            .map(|(key, value)| (key.to_owned(), value.clone()))
            .collect()
    }

    fn loggable_fields(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.values.iter().filter_map(|(key, value)| {
            if matches!(key.as_str(), "message" | "summary" | "db.statement") {
                None
            } else {
                Some((key.as_str(), value))
            }
        })
    }

    fn trace_id(&self) -> String {
        self.trace_id.clone().unwrap_or_default()
    }

    fn exception(&self) -> String {
        self.exception.clone().unwrap_or_default()
    }

    fn record_text(&mut self, field: &tracing::field::Field, value: String) {
        let name = field.name();
        match name {
            "message" => self.message = Some(value.clone()),
            "summary" => self.summary = Some(value.clone()),
            "db.statement" => self.db_statement = Some(value.clone()),
            "trace_id" => self.trace_id = Some(value.clone()),
            "error" | "exception" => self.exception = Some(value.clone()),
            _ => {}
        }
        self.values.insert(name.to_owned(), Value::String(value));
    }
}

impl Visit for CapturedFields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.record_text(field, clean_debug_value(&format!("{value:?}")));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.record_text(field, value.to_owned());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.values
            .insert(field.name().to_owned(), Value::Bool(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.values
            .insert(field.name().to_owned(), Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.values
            .insert(field.name().to_owned(), Value::Number(value.into()));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        let value = serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number);
        self.values.insert(field.name().to_owned(), value);
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.record_text(field, value.to_string());
    }
}

fn clean_debug_value(value: &str) -> String {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        serde_json::from_str::<String>(value).unwrap_or_else(|_| value.to_owned())
    } else {
        value.to_owned()
    }
}

fn json_value_to_log_text(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}

fn write_json_line(writer: &mut Writer<'_>, value: &Value) -> fmt::Result {
    serde_json::to_writer(JsonFmtWriter(writer), value).map_err(|_| fmt::Error)?;
    writeln!(writer)
}

struct JsonFmtWriter<'writer, 'fmt>(&'writer mut Writer<'fmt>);

impl io::Write for JsonFmtWriter<'_, '_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let value = std::str::from_utf8(buf).map_err(io::Error::other)?;
        self.0
            .write_str(value)
            .map_err(|_| io::Error::other("format writer failed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn current_datetime() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

fn current_thread_name() -> String {
    let thread = thread::current();
    thread
        .name()
        .map_or_else(|| format!("{:?}", thread.id()), ToOwned::to_owned)
}

fn local_hostname() -> Option<String> {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        })
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
                append_elk_frame(&mut batch, &line);
                batch_line_count = batch_line_count.saturating_add(1);
                for line in receiver.try_iter().take(ELK_BATCH_MAX_LINES - 1) {
                    if !line.is_empty() {
                        append_elk_frame(&mut batch, &line);
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
            append_elk_frame(&mut batch, &line);
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

fn append_elk_frame(batch: &mut Vec<u8>, line: &[u8]) {
    let payload = String::from_utf8_lossy(line).trim().to_owned();
    if !payload.is_empty() {
        batch.extend_from_slice(payload.as_bytes());
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
