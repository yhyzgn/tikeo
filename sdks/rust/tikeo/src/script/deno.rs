use std::{path::PathBuf, time::Duration};

use async_trait::async_trait;
use tokio::{io::AsyncWriteExt, process::Command};

use super::{
    ScriptRunner, ScriptRunnerKind, ScriptRunnerTask, emit_script_output,
    validate_script_runner_task,
};
use crate::{error::WorkerSdkError, script::runtime_dirs::TaskRuntimeDirs, task::TaskOutcome};

/// Deno-backed JavaScript/TypeScript sandbox runner.
#[derive(Debug, Clone)]
pub struct DenoScriptRunner {
    kind: ScriptRunnerKind,
    runtime_command: PathBuf,
}

impl DenoScriptRunner {
    /// Create a Deno runner for JavaScript or TypeScript.
    #[must_use]
    /// New.
    pub fn new(kind: ScriptRunnerKind, runtime_command: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            runtime_command: runtime_command.into(),
        }
    }
}

#[async_trait]
impl ScriptRunner for DenoScriptRunner {
    fn kind(&self) -> ScriptRunnerKind {
        self.kind
    }

    fn advertised_sandbox_backend(&self) -> Option<String> {
        Some("deno".to_owned())
    }

    async fn run(&self, task: ScriptRunnerTask) -> Result<TaskOutcome, WorkerSdkError> {
        validate_script_runner_task(self.kind, &task)?;
        if self.kind != ScriptRunnerKind::Js && self.kind != ScriptRunnerKind::Ts {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "Deno runner supports JavaScript and TypeScript only".to_owned(),
            ));
        }
        if !task.policy.secret_refs.is_empty() {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "Deno script runner cannot resolve secret refs without a worker-local secret provider".to_owned(),
            ));
        }
        let runtime_dirs =
            TaskRuntimeDirs::create(&format!("tikeo-deno-{}-runtime", self.kind.as_str()))?;
        let mut args = vec!["run".to_owned(), "--no-prompt".to_owned()];
        if task.policy.allow_network {
            args.push("--allow-net".to_owned());
        } else if !task.policy.allowed_network_hosts.is_empty() {
            args.push(format!(
                "--allow-net={}",
                task.policy.allowed_network_hosts.join(",")
            ));
        }
        if !task.policy.env_vars.is_empty() {
            args.push(format!("--allow-env={}", task.policy.env_vars.join(",")));
        }
        if !task.policy.read_only_paths.is_empty() {
            args.push(format!(
                "--allow-read={}",
                task.policy.read_only_paths.join(",")
            ));
        }
        let mut writable_paths = task.policy.writable_paths.clone();
        writable_paths.extend(runtime_dirs.allow_write_paths());
        if !writable_paths.is_empty() {
            args.push(format!("--allow-write={}", writable_paths.join(",")));
        }
        args.push("-".to_owned());
        let mut command = Command::new(&self.runtime_command);
        command.args(args);
        command.kill_on_drop(true);
        command.stdin(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        command.env_clear();
        runtime_dirs.apply_deno_environment(&mut command);
        command.current_dir(runtime_dirs.working_dir());
        command.env("TIKEO_SCRIPT_ID", &task.script_id);
        command.env("TIKEO_SCRIPT_VERSION_ID", &task.version_id);
        command.env(
            "TIKEO_SCRIPT_VERSION_NUMBER",
            task.version_number.to_string(),
        );
        for name in &task.policy.env_vars {
            if TaskRuntimeDirs::is_managed_environment_name(name) {
                continue;
            }
            if let Ok(value) = std::env::var(name) {
                command.env(name, value);
            }
        }
        let mut child = command
            .spawn()
            .map_err(|error| WorkerSdkError::ScriptExecutionFailed(error.to_string()))?;
        let Some(mut stdin) = child.stdin.take() else {
            return Err(WorkerSdkError::ScriptExecutionFailed(
                "Deno stdin was not available".to_owned(),
            ));
        };
        let content = task.content.clone().into_bytes();
        let writer = tokio::spawn(async move {
            stdin.write_all(&content).await?;
            stdin.shutdown().await
        });
        let timeout = Duration::from_millis(task.policy.timeout_ms);
        let output =
            if let Ok(result) = tokio::time::timeout(timeout, child.wait_with_output()).await {
                result.map_err(|error| WorkerSdkError::ScriptExecutionFailed(error.to_string()))?
            } else {
                writer.abort();
                runtime_dirs.cleanup();
                return Err(WorkerSdkError::ScriptTimeout {
                    timeout_ms: task.policy.timeout_ms,
                });
            };
        runtime_dirs.cleanup();
        emit_script_output(&task, "info", &output.stdout);
        emit_script_output(&task, "error", &output.stderr);
        writer
            .await
            .map_err(|error| WorkerSdkError::ScriptExecutionFailed(error.to_string()))?
            .map_err(|error| WorkerSdkError::ScriptExecutionFailed(error.to_string()))?;
        if output.status.success() {
            Ok(TaskOutcome::Succeeded)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            Ok(TaskOutcome::Failed(if stderr.is_empty() {
                stdout
            } else {
                stderr
            }))
        }
    }
}
