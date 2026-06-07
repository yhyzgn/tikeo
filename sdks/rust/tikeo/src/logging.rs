//! SDK diagnostic logging for Rust Worker clients.
//!
//! This module is intentionally independent from task-scoped instance logs. Task output still flows
//! through [`TaskContext`](crate::TaskContext) so unrelated process logs are never attached to a job
//! instance. SDK diagnostics describe connection, registration, heartbeat, and runtime lifecycle
//! events and can be written to the console plus an optional file directory.
//!
//! # Usage
//!
//! ```no_run
//! use tikeo::{SdkLogConfig, configure_sdk_logging};
//!
//! configure_sdk_logging(SdkLogConfig::info().with_log_dir("./logs"));
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
/// This logger is for SDK diagnostics only. Use [`TaskContext::log_info`](crate::TaskContext::log_info)
/// and [`TaskContext::log_error`](crate::TaskContext::log_error) for task instance logs.
#[must_use]
pub fn configure_sdk_logging(config: SdkLogConfig) -> bool {
    SDK_LOGGER.set(Mutex::new(SdkLogger::new(config))).is_ok()
}

pub fn sdk_log(level: SdkLogLevel, message: impl AsRef<str>) {
    let logger = SDK_LOGGER.get_or_init(|| Mutex::new(SdkLogger::new(SdkLogConfig::from_env())));
    if let Ok(mut guard) = logger.lock() {
        guard.log(level, message.as_ref());
    }
}
