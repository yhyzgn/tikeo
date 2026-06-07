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

pub use config::WorkerConfig;
pub use error::WorkerSdkError;
pub use logging::{SdkLogConfig, SdkLogLevel, configure_sdk_logging};
pub use management::{
    CreateJobRequest as ManagementCreateJobRequest, JobDefinition, ManagementClient,
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
