#![allow(clippy::redundant_pub_crate)]

use async_trait::async_trait;

use crate::{error::WorkerSdkError, proto::worker::v1::DispatchTask};

/// User-provided async processor interface for future task dispatch support.
#[async_trait]
pub trait TaskProcessor: Send + Sync + 'static {
    /// Execute one task payload.
    async fn process(&self, task: TaskContext) -> Result<TaskOutcome, WorkerSdkError>;
}

/// Minimal task context placeholder reserved for Worker dispatch protocol evolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskContext {
    /// Job identifier.
    pub job_id: String,
    /// Explicit processor key/name for SDK routing.
    pub processor_name: String,
    /// Instance identifier.
    pub instance_id: String,
    /// Raw task payload.
    pub payload: Vec<u8>,
}

/// Minimal processor outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutcome {
    /// Task completed successfully.
    Succeeded,
    /// Task failed with a message safe to send back to tikee.
    Failed(String),
}

impl TaskOutcome {
    pub(crate) fn message(&self) -> Option<String> {
        match self {
            Self::Succeeded => None,
            Self::Failed(message) => Some(message.clone()),
        }
    }

    /// Stable governance failure class extracted from the failure message when available.
    #[must_use]
    pub fn failure_class(&self) -> Option<&'static str> {
        match self {
            Self::Succeeded => None,
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

pub(crate) fn task_context(task: &DispatchTask) -> TaskContext {
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
    }
}
