#![allow(clippy::redundant_pub_crate)]

use std::sync::Arc;

use async_trait::async_trait;

use crate::{error::WorkerSdkError, proto::worker::v1::DispatchTask};

/// User-provided async processor interface for Worker task dispatch.
#[async_trait]
pub trait TaskProcessor: Send + Sync + 'static {
    /// Execute one task payload.
    async fn process(&self, task: TaskContext) -> Result<TaskOutcome, WorkerSdkError>;
}

/// Task context received from the Worker dispatch protocol.
#[derive(Clone)]
pub struct TaskContext {
    /// Job identifier.
    pub job_id: String,
    /// Explicit processor key/name for SDK routing.
    pub processor_name: String,
    /// Instance identifier.
    pub instance_id: String,
    /// Raw task payload.
    pub payload: Vec<u8>,
    /// Task-scoped logger that emits precise instance logs.
    pub logger: TaskLogger,
}

/// Task-scoped logger callback.
pub type TaskLogger = Arc<dyn Fn(&str, String) + Send + Sync>;

impl TaskContext {
    /// Emit one info log line for this task instance.
    pub fn log_info(&self, message: impl Into<String>) {
        (self.logger)("info", message.into());
    }

    /// Emit one error log line for this task instance.
    pub fn log_error(&self, message: impl Into<String>) {
        (self.logger)("error", message.into());
    }
}

/// Processor outcome reported to the Worker Tunnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutcome {
    /// Task completed successfully without an additional operator-facing message.
    Succeeded,
    /// Task completed successfully with an operator-facing message safe to send back to tikeo.
    Success(String),
    /// Task failed with a message safe to send back to tikeo.
    Failed(String),
}

impl TaskOutcome {
    pub(crate) fn message(&self) -> Option<String> {
        match self {
            Self::Succeeded => None,
            Self::Success(message) | Self::Failed(message) => Some(message.clone()),
        }
    }

    /// Whether this outcome should be reported as a successful task result.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Success(_))
    }

    /// Stable governance failure class extracted from the failure message when available.
    #[must_use]
    pub fn failure_class(&self) -> Option<&'static str> {
        match self {
            Self::Succeeded | Self::Success(_) => None,
            Self::Failed(message) => classify_failure_message(message),
        }
    }
}

fn classify_failure_message(message: &str) -> Option<&'static str> {
    let lower = message.to_ascii_lowercase();
    if lower.contains("not registered") || lower.contains("not enabled") {
        Some("script_missing_worker_runner")
    } else if lower.contains("policy")
        || lower.contains("network access")
        || lower.contains("filesystem access")
        || lower.contains("secret access")
        || lower.contains("must be greater than zero")
    {
        Some("script_policy_rejected")
    } else if lower.contains("digest mismatch") {
        Some("script_digest_mismatch")
    } else if lower.contains("timed out") {
        Some("script_timeout")
    } else if lower.contains("output exceeded") {
        Some("script_output_limit")
    } else if lower.contains("runtime unavailable") || lower.contains("executable not found") {
        Some("script_runtime_unavailable")
    } else {
        None
    }
}

pub(crate) fn task_context(task: &DispatchTask, logger: TaskLogger) -> TaskContext {
    let processor_name = if task.processor_name.is_empty() {
        task.job_id.clone()
    } else {
        task.processor_name.clone()
    };
    TaskContext {
        job_id: task.job_id.clone(),
        processor_name,
        instance_id: task.instance_id.clone(),
        payload: task.payload.clone(),
        logger,
    }
}
