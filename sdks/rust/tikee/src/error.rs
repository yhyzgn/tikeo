use thiserror::Error;
use tonic::Status;

/// Worker SDK errors.
#[derive(Debug, Error)]
pub enum WorkerSdkError {
    /// gRPC transport error.
    #[error("worker tunnel transport error: {0}")]
    Transport(#[from] tonic::transport::Error),
    /// gRPC status error.
    #[error("worker tunnel status error: {0}")]
    Status(#[from] Status),
    /// The tunnel closed before the expected response arrived.
    #[error("worker tunnel closed")]
    TunnelClosed,
    /// Tikee returned an unexpected server message.
    #[error("unexpected worker tunnel server message")]
    UnexpectedMessage,
    /// A dynamic script runtime executable is unavailable on this worker.
    #[error("script runtime unavailable: {0}")]
    ScriptRuntimeUnavailable(String),
    /// Dynamic script execution failed before a task result could be produced.
    #[error("script execution failed: {0}")]
    ScriptExecutionFailed(String),
    /// Dynamic script exceeded its wall-clock timeout.
    #[error("script timed out after {timeout_ms}ms")]
    ScriptTimeout {
        /// Configured timeout in milliseconds.
        timeout_ms: u64,
    },
    /// Dynamic script exceeded its captured output limit.
    #[error("script output exceeded {max_output_bytes} bytes: {actual_bytes} bytes")]
    ScriptOutputLimitExceeded {
        /// Configured captured output limit.
        max_output_bytes: u64,
        /// Actual captured stdout+stderr bytes.
        actual_bytes: u64,
    },
    /// A dynamic script runner was requested before a safe sandbox implementation exists.
    #[error("unsupported script runner: {0}")]
    UnsupportedScriptRunner(String),
}
