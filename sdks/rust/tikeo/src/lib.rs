//! Rust Worker SDK for active outbound tikeo Worker Tunnel connections.

#![forbid(unsafe_code)]

pub mod proto;

mod config;
mod error;
mod logging;
pub mod management;
mod script;
mod session;
mod task;
mod wasm;

pub use config::{PluginType, WorkerConfig};
pub use error::WorkerSdkError;
pub use logging::{SdkLogConfig, SdkLogLevel, configure_sdk_logging, install_task_log_bridge};
pub use management::{
    BroadcastSelectorRequest as ManagementBroadcastSelectorRequest,
    CreateJobRequest as ManagementCreateJobRequest, JobDefinition, JobInstance, ManagementClient,
    TriggerJobRequest as ManagementTriggerJobRequest,
};
pub use script::{
    ContainerScriptRunner, DenoScriptRunner, LocalSubprocessScriptRunner, SandboxToolResolver,
    ScriptRunner, ScriptRunnerKind, ScriptRunnerPolicy, ScriptRunnerRegistry, ScriptRunnerTask,
    SrtScriptRunner, UnsupportedScriptRunner,
};
pub use session::{WorkerClient, WorkerSession};
pub use task::{TaskContext, TaskOutcome, TaskProcessor};

#[cfg(test)]
mod tests;
