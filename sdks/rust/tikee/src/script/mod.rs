#![allow(clippy::redundant_pub_crate)]

use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tokio::{io::AsyncWriteExt, process::Command};

use crate::{error::WorkerSdkError, task::TaskOutcome};

mod container;
mod local;

pub use container::ContainerScriptRunner;
pub use local::LocalSubprocessScriptRunner;

/// Supported non-WASM dynamic script runner kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptRunnerKind {
    /// POSIX shell runner.
    Shell,
    /// Python runner.
    Python,
    /// Node.js runner.
    Node,
    /// PowerShell runner.
    PowerShell,
    /// Rhai expression/script runner.
    Rhai,
}

impl ScriptRunnerKind {
    /// Parse a wire language value into a runner kind.
    #[must_use]
    pub fn from_language(language: &str) -> Option<Self> {
        match language.trim().to_ascii_lowercase().as_str() {
            "shell" | "sh" | "bash" => Some(Self::Shell),
            "python" | "py" => Some(Self::Python),
            "node" | "nodejs" | "javascript" | "js" | "typescript" | "ts" => Some(Self::Node),
            "powershell" | "pwsh" => Some(Self::PowerShell),
            "rhai" => Some(Self::Rhai),
            _ => None,
        }
    }

    /// Stable runner name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shell => "shell",
            Self::Python => "python",
            Self::Node => "node",
            Self::PowerShell => "powershell",
            Self::Rhai => "rhai",
        }
    }
}

/// Default-deny policy snapshot for non-WASM dynamic script runners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRunnerPolicy {
    /// Maximum wall-clock runtime in milliseconds.
    pub timeout_ms: u64,
    /// Maximum memory in bytes.
    pub max_memory_bytes: u64,
    /// Maximum captured output bytes.
    pub max_output_bytes: u64,
    /// Whether network egress is allowed. Current SDK abstraction rejects it.
    pub allow_network: bool,
    /// Allowed environment variable names.
    pub env_vars: Vec<String>,
    /// Read-only filesystem paths granted to the runner. Current SDK abstraction rejects them.
    pub read_only_paths: Vec<String>,
    /// Writable filesystem paths granted to the runner. Current SDK abstraction rejects them.
    pub writable_paths: Vec<String>,
    /// Secret references granted to the runner. Current SDK abstraction rejects them.
    pub secret_refs: Vec<String>,
}

impl Default for ScriptRunnerPolicy {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            max_memory_bytes: 64 * 1024 * 1024,
            max_output_bytes: 1024 * 1024,
            allow_network: false,
            env_vars: Vec::new(),
            read_only_paths: Vec::new(),
            writable_paths: Vec::new(),
            secret_refs: Vec::new(),
        }
    }
}

impl ScriptRunnerPolicy {
    /// Validate the SDK-side policy boundary before any future local runner executes code.
    ///
    /// # Errors
    ///
    /// Returns an error for zero limits or dangerous capabilities that require future
    /// platform policy grants.
    pub fn validate_default_deny(&self) -> Result<(), WorkerSdkError> {
        if self.timeout_ms == 0 {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "script timeout must be greater than zero".to_owned(),
            ));
        }
        if self.max_memory_bytes == 0 {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "script memory limit must be greater than zero".to_owned(),
            ));
        }
        if self.max_output_bytes == 0 {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "script output limit must be greater than zero".to_owned(),
            ));
        }
        if self.allow_network {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "script network access requires a future URL policy grant".to_owned(),
            ));
        }
        if !self.read_only_paths.is_empty() || !self.writable_paths.is_empty() {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "script filesystem access requires a future filesystem policy grant".to_owned(),
            ));
        }
        if !self.secret_refs.is_empty() {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "script secret access requires a future secret policy grant".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Explicit worker-side script runner registry.
#[derive(Default)]
pub struct ScriptRunnerRegistry {
    runners: HashMap<ScriptRunnerKind, Box<dyn ScriptRunner>>,
}

impl ScriptRunnerRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or replace a runner for its language/kind.
    pub fn register<R>(&mut self, runner: R)
    where
        R: ScriptRunner,
    {
        self.runners.insert(runner.kind(), Box::new(runner));
    }

    pub(crate) fn get(&self, kind: ScriptRunnerKind) -> Option<&dyn ScriptRunner> {
        self.runners.get(&kind).map(std::convert::AsRef::as_ref)
    }
}

/// Future non-WASM dynamic script runner contract.
#[async_trait]
pub trait ScriptRunner: Send + Sync + 'static {
    /// Runner language/kind.
    fn kind(&self) -> ScriptRunnerKind;

    /// Execute a released immutable script snapshot.
    async fn run(&self, task: ScriptRunnerTask) -> Result<TaskOutcome, WorkerSdkError>;
}

/// Immutable script snapshot passed to a future non-WASM runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRunnerTask {
    /// Script id.
    pub script_id: String,
    /// Immutable script version id.
    pub version_id: String,
    /// Immutable script version number.
    pub version_number: u64,
    /// Script language.
    pub language: String,
    /// Script source content from the released version snapshot.
    pub content: String,
    /// Content SHA-256 digest.
    pub content_sha256: String,
    /// Default-deny execution policy snapshot.
    pub policy: ScriptRunnerPolicy,
}

pub(super) const fn default_script_command(
    kind: ScriptRunnerKind,
) -> (&'static str, &'static [&'static str]) {
    match kind {
        ScriptRunnerKind::Shell => ("sh", &["-s"]),
        ScriptRunnerKind::Python => ("python3", &["-"]),
        ScriptRunnerKind::Node => ("node", &["-"]),
        ScriptRunnerKind::PowerShell => {
            ("pwsh", &["-NoProfile", "-NonInteractive", "-Command", "-"])
        }
        ScriptRunnerKind::Rhai => ("rhai", &[]),
    }
}

pub(super) fn validate_script_runner_task(
    kind: ScriptRunnerKind,
    task: &ScriptRunnerTask,
) -> Result<(), WorkerSdkError> {
    task.policy.validate_default_deny()?;
    let task_kind = ScriptRunnerKind::from_language(&task.language).ok_or_else(|| {
        WorkerSdkError::UnsupportedScriptRunner(format!(
            "unsupported script language: {}",
            task.language
        ))
    })?;
    if task_kind != kind {
        return Err(WorkerSdkError::UnsupportedScriptRunner(format!(
            "{} runner cannot execute {} scripts",
            kind.as_str(),
            task.language
        )));
    }
    if task.version_id.trim().is_empty() || task.version_number == 0 {
        return Err(WorkerSdkError::UnsupportedScriptRunner(
            "script runner requires a released immutable script version snapshot".to_owned(),
        ));
    }
    if task.content_sha256.trim().is_empty() {
        return Err(WorkerSdkError::UnsupportedScriptRunner(
            "script runner requires a content sha256 digest".to_owned(),
        ));
    }
    let actual = format!("{:x}", Sha256::digest(task.content.as_bytes()));
    if !actual.eq_ignore_ascii_case(task.content_sha256.trim()) {
        return Err(WorkerSdkError::UnsupportedScriptRunner(
            "script content sha256 digest mismatch".to_owned(),
        ));
    }
    Ok(())
}

pub(super) async fn run_script_command(
    mut command: Command,
    kind: ScriptRunnerKind,
    task: ScriptRunnerTask,
) -> Result<TaskOutcome, WorkerSdkError> {
    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            WorkerSdkError::ScriptRuntimeUnavailable(format!(
                "{} runner executable not found",
                kind.as_str()
            ))
        } else {
            WorkerSdkError::ScriptExecutionFailed(format!(
                "failed to spawn {} runner: {error}",
                kind.as_str()
            ))
        }
    })?;

    let Some(mut stdin) = child.stdin.take() else {
        return Err(WorkerSdkError::ScriptExecutionFailed(
            "script runner stdin was not available".to_owned(),
        ));
    };
    let content = task.content.into_bytes();
    let writer = tokio::spawn(async move {
        stdin.write_all(&content).await?;
        stdin.shutdown().await
    });

    let timeout = Duration::from_millis(task.policy.timeout_ms);
    let output = if let Ok(result) = tokio::time::timeout(timeout, child.wait_with_output()).await {
        result.map_err(|error| {
            WorkerSdkError::ScriptExecutionFailed(format!(
                "{} runner failed: {error}",
                kind.as_str()
            ))
        })?
    } else {
        writer.abort();
        return Err(WorkerSdkError::ScriptTimeout {
            timeout_ms: task.policy.timeout_ms,
        });
    };
    writer
        .await
        .map_err(|error| {
            WorkerSdkError::ScriptExecutionFailed(format!("script stdin writer failed: {error}"))
        })?
        .map_err(|error| {
            WorkerSdkError::ScriptExecutionFailed(format!("script stdin write failed: {error}"))
        })?;

    let max_output = usize::try_from(task.policy.max_output_bytes).unwrap_or(usize::MAX);
    let output_len = output.stdout.len().saturating_add(output.stderr.len());
    if output_len > max_output {
        return Err(WorkerSdkError::ScriptOutputLimitExceeded {
            max_output_bytes: task.policy.max_output_bytes,
            actual_bytes: u64::try_from(output_len).unwrap_or(u64::MAX),
        });
    }

    if output.status.success() {
        Ok(TaskOutcome::Succeeded)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let message = if stderr.is_empty() { stdout } else { stderr };
        Ok(TaskOutcome::Failed(if message.is_empty() {
            format!(
                "{} runner exited with status {}",
                kind.as_str(),
                output.status
            )
        } else {
            message
        }))
    }
}

/// Placeholder runner used until language-specific sandbox implementations are enabled.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnsupportedScriptRunner;

#[async_trait]
impl ScriptRunner for UnsupportedScriptRunner {
    fn kind(&self) -> ScriptRunnerKind {
        ScriptRunnerKind::Shell
    }

    async fn run(&self, task: ScriptRunnerTask) -> Result<TaskOutcome, WorkerSdkError> {
        task.policy.validate_default_deny()?;
        Err(WorkerSdkError::UnsupportedScriptRunner(format!(
            "{} script runner is not enabled; use a dedicated sandbox runner before executing dynamic scripts",
            task.language
        )))
    }
}
