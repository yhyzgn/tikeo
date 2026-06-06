//! Rust Worker SDK for active outbound tikeo Worker Tunnel connections.

#![forbid(unsafe_code)]

pub mod proto;

mod config;
mod error;
pub mod management;
mod script;
mod session;
mod task;
mod wasm;

pub use config::WorkerConfig;
pub use error::WorkerSdkError;
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
