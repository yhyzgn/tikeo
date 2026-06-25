use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use tokio::process::Command;

use super::{
    ScriptRunner, ScriptRunnerKind, ScriptRunnerTask, default_script_command, run_script_command,
    validate_script_runner_task,
};
use crate::{error::WorkerSdkError, task::TaskOutcome};

/// Opt-in sandboxed container runner for non-WASM dynamic scripts.
///
/// This runner invokes a local container runtime CLI (Docker-compatible by default) from the
/// Worker process and passes script content through stdin. It is designed as a safer boundary
/// than direct host subprocess execution: network is disabled unless a future host-filtering
/// sandbox is supplied, the container root filesystem is read-only, file grants become explicit
/// bind mounts, and only explicitly whitelisted env vars are injected into the container.
/// Workers must still opt in by registering this runner and advertising the matching
/// structured `script_runners.language` capability.
#[derive(Debug, Clone)]
pub struct ContainerScriptRunner {
    kind: ScriptRunnerKind,
    runtime_command: PathBuf,
    image: String,
    runtime_args: Vec<String>,
}

impl ContainerScriptRunner {
    /// Create a Docker-compatible runner using the provided image.
    #[must_use]
    /// New.
    pub fn new(kind: ScriptRunnerKind, image: impl Into<String>) -> Self {
        Self::with_runtime(kind, "docker", image, std::iter::empty::<String>())
    }

    /// Create a runner with an explicit container runtime command and extra runtime args.
    #[must_use]
    /// With runtime.
    pub fn with_runtime(
        kind: ScriptRunnerKind,
        runtime_command: impl Into<PathBuf>,
        image: impl Into<String>,
        runtime_args: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            kind,
            runtime_command: runtime_command.into(),
            image: image.into(),
            runtime_args: runtime_args.into_iter().collect(),
        }
    }

    /// Docker args.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub(crate) fn docker_args(
        &self,
        task: &ScriptRunnerTask,
    ) -> Result<Vec<String>, WorkerSdkError> {
        validate_script_runner_task(self.kind, task)?;
        Self::validate_supported_capabilities(task)?;
        if self.image.trim().is_empty() {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "container script runner requires an image".to_owned(),
            ));
        }

        let mut args = vec![
            "run".to_owned(),
            "--rm".to_owned(),
            "-i".to_owned(),
            "--network=none".to_owned(),
            "--read-only".to_owned(),
            "--tmpfs".to_owned(),
            "/tmp:rw,noexec,nosuid,size=16m".to_owned(),
            format!("--memory={}", task.policy.max_memory_bytes),
            "--env".to_owned(),
            format!("TIKEO_SCRIPT_ID={}", task.script_id),
            "--env".to_owned(),
            format!("TIKEO_SCRIPT_VERSION_ID={}", task.version_id),
            "--env".to_owned(),
            format!("TIKEO_SCRIPT_VERSION_NUMBER={}", task.version_number),
        ];
        args.extend(self.runtime_args.iter().cloned());
        add_file_mounts(&mut args, &task.policy.read_only_paths, true)?;
        add_file_mounts(&mut args, &task.policy.writable_paths, false)?;
        for name in &task.policy.env_vars {
            if let Ok(value) = std::env::var(name) {
                args.push("--env".to_owned());
                args.push(format!("{name}={value}"));
            }
        }
        args.push(self.image.clone());
        let (script_command, script_args) = default_script_command(self.kind);
        args.push(script_command.to_owned());
        args.extend(script_args.iter().map(|arg| (*arg).to_owned()));
        Ok(args)
    }

    fn validate_supported_capabilities(task: &ScriptRunnerTask) -> Result<(), WorkerSdkError> {
        if task.policy.allow_network || !task.policy.allowed_network_hosts.is_empty() {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "container script runner cannot safely enforce host-level network grants with Docker CLI alone".to_owned(),
            ));
        }
        if !task.policy.secret_refs.is_empty() {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "container script runner cannot resolve script secret refs without a worker-local secret provider".to_owned(),
            ));
        }
        Ok(())
    }
}

fn add_file_mounts(
    args: &mut Vec<String>,
    paths: &[String],
    read_only: bool,
) -> Result<(), WorkerSdkError> {
    for path in paths {
        let path = validate_mount_path(path)?;
        let mode = if read_only { ",readonly" } else { "" };
        args.push("--mount".to_owned());
        args.push(format!(
            "type=bind,src={path},dst={path}{mode}",
            path = path.display()
        ));
    }
    Ok(())
}

fn validate_mount_path(path: &str) -> Result<PathBuf, WorkerSdkError> {
    let trimmed = path.trim();
    let candidate = Path::new(trimmed);
    if trimmed.is_empty()
        || trimmed != path
        || !candidate.is_absolute()
        || candidate.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::CurDir | Component::Prefix(_)
            )
        })
    {
        return Err(WorkerSdkError::UnsupportedScriptRunner(format!(
            "script file grant path must be clean and absolute: {path}"
        )));
    }
    Ok(candidate.to_path_buf())
}

#[async_trait]
impl ScriptRunner for ContainerScriptRunner {
    fn kind(&self) -> ScriptRunnerKind {
        self.kind
    }

    fn advertised_sandbox_backend(&self) -> Option<String> {
        let executable = self
            .runtime_command
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| self.runtime_command.to_str().unwrap_or("docker"));
        Some(executable.to_owned())
    }

    async fn run(&self, task: ScriptRunnerTask) -> Result<TaskOutcome, WorkerSdkError> {
        let args = self.docker_args(&task)?;
        let mut command = Command::new(&self.runtime_command);
        command.args(args);
        command.kill_on_drop(true);
        command.stdin(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        run_script_command(command, self.kind, task).await
    }
}
