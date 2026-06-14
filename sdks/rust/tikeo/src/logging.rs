//! SDK diagnostic logging and task-scoped logging bridge for Rust Worker clients.
//!
//! SDK diagnostics describe connection, registration, heartbeat, and runtime lifecycle events and
//! can be written to the console plus an optional file directory. Task logs remain task-scoped:
//! [`TaskContext::log_info`](crate::TaskContext::log_info) and
//! [`TaskContext::log_error`](crate::TaskContext::log_error) are the low-level fallback, while
//! [`tracing::info!`] and [`tracing::error!`] emitted during processor execution are captured only
//! when a tikeo task scope is active.
//!
//! # Usage
//!
//! ```no_run
//! use tikeo::{SdkLogConfig, configure_sdk_logging, install_task_log_bridge};
//!
//! configure_sdk_logging(SdkLogConfig::info().with_log_dir("./logs"));
//! let _installed = install_task_log_bridge();
//! ```
//!
//! # Operational cautions
//!
//! Keep the default `INFO` level for production workers. Enable `DEBUG` only while diagnosing Worker
//! Tunnel connectivity or sandbox tool resolution because it may include endpoint and processor
//! metadata. Do not log secrets or raw task payloads from SDK diagnostics.

use std::{
    fs::{File, OpenOptions, create_dir_all},
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use tracing::{Event, Level, Subscriber, field};
use tracing_log::LogTracer;
use tracing_subscriber::{Layer, Registry, layer::Context, prelude::*};

use crate::task::TaskLogger;

/// Minimum severity for SDK diagnostic output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SdkLogLevel {
    /// Verbose diagnostics for short-lived troubleshooting sessions.
    Debug,
    /// Normal lifecycle diagnostics. This is the default.
    Info,
    /// Recoverable problems that deserve operator attention.
    Warning,
    /// Failures that interrupted a connection, task report, or runtime action.
    Error,
}

impl SdkLogLevel {
    /// Parse a log level name. Unknown values fall back to [`SdkLogLevel::Info`].
    #[must_use]
    pub fn parse(value: impl AsRef<str>) -> Self {
        match value.as_ref().trim().to_ascii_lowercase().as_str() {
            "debug" => Self::Debug,
            "warn" | "warning" => Self::Warning,
            "error" => Self::Error,
            _ => Self::Info,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// SDK diagnostic logging configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkLogConfig {
    /// Minimum level emitted when writing SDK diagnostics.
    pub level: SdkLogLevel,
    /// Optional directory that receives `tikeo-sdk.log` in addition to console output.
    pub log_dir: Option<PathBuf>,
}

impl SdkLogConfig {
    /// Return an `INFO` level console-only logging configuration.
    #[must_use]
    pub const fn info() -> Self {
        Self {
            level: SdkLogLevel::Info,
            log_dir: None,
        }
    }

    /// Return configuration from `TIKEO_SDK_LOG_LEVEL` and `TIKEO_SDK_LOG_DIR`.
    #[must_use]
    pub fn from_env() -> Self {
        let level =
            std::env::var("TIKEO_SDK_LOG_LEVEL").map_or(SdkLogLevel::Info, SdkLogLevel::parse);
        let log_dir = std::env::var("TIKEO_SDK_LOG_DIR")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        Self { level, log_dir }
    }

    /// Enable file output under the provided directory.
    #[must_use]
    pub fn with_log_dir(mut self, log_dir: impl AsRef<Path>) -> Self {
        self.log_dir = Some(log_dir.as_ref().to_path_buf());
        self
    }
}

impl Default for SdkLogConfig {
    fn default() -> Self {
        Self::info()
    }
}

struct SdkLogger {
    config: SdkLogConfig,
    file: Option<File>,
}

impl SdkLogger {
    fn new(config: SdkLogConfig) -> Self {
        let file = config.log_dir.as_ref().and_then(|dir| {
            if create_dir_all(dir).is_err() {
                return None;
            }
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("tikeo-sdk.log"))
                .ok()
        });
        Self { config, file }
    }

    fn log(&mut self, level: SdkLogLevel, message: &str) {
        if level < self.config.level {
            return;
        }
        let line = format!("[tikeo-sdk] {} {}", level.as_str(), message);
        if level == SdkLogLevel::Error {
            eprintln!("{line}");
        } else {
            println!("{line}");
        }
        if let Some(file) = &mut self.file {
            let _ = writeln!(file, "{line}");
        }
    }
}

static SDK_LOGGER: OnceLock<Mutex<SdkLogger>> = OnceLock::new();
static TASK_LOG_BRIDGE: OnceLock<bool> = OnceLock::new();

tokio::task_local! {
    static TASK_LOG_SCOPE: TaskLogger;
}

/// Configure process-level SDK diagnostics.
///
/// Calling this function more than once is safe; later calls are ignored so libraries do not fight
/// application-owned logging configuration.
///
/// # Usage
///
/// Call this early in the worker process, before opening the Worker Tunnel.
///
/// # Operational cautions
///
/// This logger is for SDK diagnostics only. Use [`TaskContext::log_info`](crate::TaskContext::log_info),
/// [`TaskContext::log_error`](crate::TaskContext::log_error), or task-scoped `tracing` events for task
/// instance logs.
#[must_use]
pub fn configure_sdk_logging(config: SdkLogConfig) -> bool {
    SDK_LOGGER.set(Mutex::new(SdkLogger::new(config))).is_ok()
}

/// Install the global bridge that forwards task-scoped [`tracing`] and [`log`] events into tikeo task logs.
///
/// The bridge is intentionally scope-gated: events are forwarded only while the SDK is executing a
/// processor inside a tikeo task scope. Process-level logs outside that scope are ignored by this
/// bridge and are never attached to a task instance. Calling this more than once is safe; only the
/// first call installs the global subscriber/log adapter.
///
/// Returns `true` when this call installed the bridge and `false` when it was already installed or
/// another global tracing subscriber/log adapter was already configured by the application.
#[must_use]
pub fn install_task_log_bridge() -> bool {
    *TASK_LOG_BRIDGE.get_or_init(|| {
        let subscriber = Registry::default().with(TaskLogLayer);
        let tracing_installed = tracing::subscriber::set_global_default(subscriber).is_ok();
        let log_installed = LogTracer::init().is_ok();
        tracing_installed || log_installed
    })
}

pub(crate) async fn in_task_log_scope<F>(logger: TaskLogger, future: F) -> F::Output
where
    F: std::future::Future,
{
    TASK_LOG_SCOPE.scope(logger, future).await
}

pub fn sdk_log(level: SdkLogLevel, message: impl AsRef<str>) {
    let logger = SDK_LOGGER.get_or_init(|| Mutex::new(SdkLogger::new(SdkLogConfig::from_env())));
    if let Ok(mut guard) = logger.lock() {
        guard.log(level, message.as_ref());
    }
}

struct TaskLogLayer;

impl<S> Layer<S> for TaskLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let Some(level) = task_log_level(metadata.level()) else {
            return;
        };
        let _ = TASK_LOG_SCOPE.try_with(|logger| {
            let mut visitor = MessageVisitor::default();
            event.record(&mut visitor);
            let message = visitor.finish();
            if !message.is_empty() {
                (logger)(level, message);
            }
        });
    }
}

fn task_log_level(level: &Level) -> Option<&'static str> {
    match *level {
        Level::ERROR => Some("error"),
        Level::WARN | Level::INFO => Some("info"),
        Level::DEBUG | Level::TRACE => None,
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
    fields: Vec<String>,
}

impl MessageVisitor {
    fn finish(self) -> String {
        match (self.message, self.fields.is_empty()) {
            (Some(message), true) => message,
            (Some(message), false) => format!("{} {}", message, self.fields.join(" ")),
            (None, true) => String::new(),
            (None, false) => self.fields.join(" "),
        }
    }

    fn push_field(&mut self, field: &field::Field, value: String) {
        if field.name() == "message" {
            self.message = Some(value);
        } else {
            self.fields.push(format!("{}={}", field.name(), value));
        }
    }
}

impl field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &field::Field, value: &dyn std::fmt::Debug) {
        self.push_field(field, format!("{value:?}"));
    }

    fn record_str(&mut self, field: &field::Field, value: &str) {
        self.push_field(field, value.to_owned());
    }

    fn record_error(&mut self, field: &field::Field, value: &(dyn std::error::Error + 'static)) {
        self.push_field(field, value.to_string());
    }
}
