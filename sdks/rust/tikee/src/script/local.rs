use std::{path::PathBuf, time::Duration};

use async_trait::async_trait;
use tokio::{io::AsyncWriteExt, process::Command};

use super::{
    ScriptRunner, ScriptRunnerKind, ScriptRunnerTask, default_script_command,
    validate_script_runner_task,
};
use crate::{error::WorkerSdkError, task::TaskOutcome};

/// Development-only local subprocess runner for non-WASM dynamic scripts.
///
/// This runner is intentionally small and default-deny, but it is not a production sandbox
/// boundary: the child process still runs on the Worker host. Use it only in SDK tests or
/// isolated development diagnostics. Production script-capable Workers must register a
/// sandbox runner such as [`ContainerScriptRunner`](super::ContainerScriptRunner) or a
/// stronger runtime.
#[derive(Debug, Clone)]
pub struct LocalSubprocessScriptRunner {
    kind: ScriptRunnerKind,
    command: PathBuf,
    args: Vec<String>,
}

impl LocalSubprocessScriptRunner {
    /// Create a runner using the default executable for the language.
    #[must_use]
    pub fn new(kind: ScriptRunnerKind) -> Self {
        let (command, args) = default_script_command(kind);
        Self::with_command(kind, command, args.iter().map(|arg| (*arg).to_owned()))
    }

    /// Create a runner with an explicit executable and argument vector.
    #[must_use]
    pub fn with_command(
        kind: ScriptRunnerKind,
        command: impl Into<PathBuf>,
        args: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            kind,
            command: command.into(),
            args: args.into_iter().collect(),
        }
    }

    fn validate_task(&self, task: &ScriptRunnerTask) -> Result<(), WorkerSdkError> {
        validate_script_runner_task(self.kind, task)
    }
}

#[async_trait]
impl ScriptRunner for LocalSubprocessScriptRunner {
    fn kind(&self) -> ScriptRunnerKind {
        self.kind
    }

    fn advertised_sandbox_backend(&self) -> Option<String> {
        Some("custom".to_owned())
    }

    async fn run(&self, task: ScriptRunnerTask) -> Result<TaskOutcome, WorkerSdkError> {
        self.validate_task(&task)?;

        let mut command = Command::new(&self.command);
        command.args(&self.args);
        command.kill_on_drop(true);
        command.stdin(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        command.env_clear();
        command.env("TIKEE_SCRIPT_ID", &task.script_id);
        command.env("TIKEE_SCRIPT_VERSION_ID", &task.version_id);
        command.env(
            "TIKEE_SCRIPT_VERSION_NUMBER",
            task.version_number.to_string(),
        );
        for name in &task.policy.env_vars {
            if let Ok(value) = std::env::var(name) {
                command.env(name, value);
            }
        }

        let mut child = command.spawn().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                WorkerSdkError::ScriptRuntimeUnavailable(format!(
                    "{} runner executable not found: {}",
                    self.kind.as_str(),
                    self.command.display()
                ))
            } else {
                WorkerSdkError::ScriptExecutionFailed(format!(
                    "failed to spawn {} runner: {error}",
                    self.kind.as_str()
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
        let output =
            if let Ok(result) = tokio::time::timeout(timeout, child.wait_with_output()).await {
                result.map_err(|error| {
                    WorkerSdkError::ScriptExecutionFailed(format!(
                        "{} runner failed: {error}",
                        self.kind.as_str()
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
                WorkerSdkError::ScriptExecutionFailed(format!(
                    "script stdin writer failed: {error}"
                ))
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
                    self.kind.as_str(),
                    output.status
                )
            } else {
                message
            }))
        }
    }
}
