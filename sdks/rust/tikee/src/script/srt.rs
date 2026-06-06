use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

use async_trait::async_trait;
use tokio::process::Command;

use super::{
    ScriptRunner, ScriptRunnerKind, ScriptRunnerTask, emit_script_output,
    validate_script_runner_task,
};
use crate::{
    error::WorkerSdkError,
    script::runtime_dirs::{TaskRuntimeDirs, system_temp_file},
    task::TaskOutcome,
};

/// Anthropic Sandbox Runtime backed runner for native scripts.
#[derive(Debug, Clone)]
pub struct SrtScriptRunner {
    kind: ScriptRunnerKind,
    runtime_command: PathBuf,
    interpreter_command: PathBuf,
    extra_path_entries: Vec<PathBuf>,
}

impl SrtScriptRunner {
    /// Create a runner with the provided SRT runtime and interpreter command.
    #[must_use]
    pub fn new(
        kind: ScriptRunnerKind,
        runtime_command: impl Into<PathBuf>,
        interpreter_command: impl Into<PathBuf>,
        extra_path_entries: impl IntoIterator<Item = PathBuf>,
    ) -> Self {
        Self {
            kind,
            runtime_command: runtime_command.into(),
            interpreter_command: interpreter_command.into(),
            extra_path_entries: extra_path_entries.into_iter().collect(),
        }
    }

    fn shell_command(&self, content: &str, script_file: Option<&std::path::Path>) -> String {
        let interpreter = self.interpreter_command.to_string_lossy();
        match self.kind {
            ScriptRunnerKind::Shell | ScriptRunnerKind::Js | ScriptRunnerKind::Ts => {
                content.to_owned()
            }
            ScriptRunnerKind::Python => heredoc(&format!("{interpreter} -"), "PY", content),
            ScriptRunnerKind::PowerShell => heredoc(
                &format!(
                    "cd \"$HOME\" && {interpreter} -NoLogo -NoProfile -NonInteractive -InputFormat Text -OutputFormat Text -Command -"
                ),
                "PWSH",
                content,
            ),
            ScriptRunnerKind::Php => heredoc(&interpreter, "PHP", content),
            ScriptRunnerKind::Groovy => heredoc(&interpreter, "GROOVY", content),
            ScriptRunnerKind::Rhai => script_file.map_or_else(
                || heredoc(&interpreter, "RHAI", content),
                |path| format!("{interpreter} {}", shell_quote(&path.to_string_lossy())),
            ),
        }
    }

    fn configure_environment(
        &self,
        command: &mut Command,
        task: &ScriptRunnerTask,
        runtime_dirs: Option<&TaskRuntimeDirs>,
    ) {
        command.env_clear();
        if let Some(path) = sanitized_path(&self.extra_path_entries, std::env::var_os("PATH")) {
            command.env("PATH", path);
        }
        if let Some(runtime_dirs) = runtime_dirs {
            runtime_dirs.apply_srt_environment(command);
            if self.kind == ScriptRunnerKind::PowerShell {
                runtime_dirs.apply_powershell_environment(command);
            }
        } else if let Some(home) = std::env::var_os("HOME") {
            command.env("HOME", home);
        }
        command.env("TIKEE_SCRIPT_ID", &task.script_id);
        command.env("TIKEE_SCRIPT_VERSION_ID", &task.version_id);
        command.env(
            "TIKEE_SCRIPT_VERSION_NUMBER",
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
    }
}

fn sanitized_path(
    extra_path_entries: &[PathBuf],
    inherited_path: Option<OsString>,
) -> Option<OsString> {
    let mut path_entries = extra_path_entries.to_vec();
    if let Some(path) = inherited_path {
        path_entries.extend(std::env::split_paths(&path));
    }
    (!path_entries.is_empty())
        .then(|| std::env::join_paths(path_entries).ok())
        .flatten()
}

#[async_trait]
impl ScriptRunner for SrtScriptRunner {
    fn kind(&self) -> ScriptRunnerKind {
        self.kind
    }

    fn advertised_sandbox_backend(&self) -> Option<String> {
        Some("srt".to_owned())
    }

    async fn run(&self, task: ScriptRunnerTask) -> Result<TaskOutcome, WorkerSdkError> {
        validate_script_runner_task(self.kind, &task)?;
        if !task.policy.secret_refs.is_empty() {
            return Err(WorkerSdkError::UnsupportedScriptRunner(
                "SRT script runner cannot resolve secret refs without a worker-local secret provider".to_owned(),
            ));
        }
        let runtime_dirs =
            TaskRuntimeDirs::create(&format!("tikee-srt-{}-runtime", self.kind.as_str()))?;
        let script_file = if self.kind == ScriptRunnerKind::Rhai {
            let path = runtime_dirs.script_file("rhai");
            std::fs::write(&path, &task.content)
                .map_err(|error| WorkerSdkError::ScriptExecutionFailed(error.to_string()))?;
            Some(path)
        } else {
            None
        };
        let (settings, cleanup) =
            write_settings(&task, script_file.as_deref(), Some(&runtime_dirs))?;
        let mut command = Command::new(&self.runtime_command);
        command.args([
            "--settings".to_owned(),
            settings.to_string_lossy().to_string(),
            "-c".to_owned(),
            self.shell_command(&task.content, script_file.as_deref()),
        ]);
        command.kill_on_drop(true);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        self.configure_environment(&mut command, &task, Some(&runtime_dirs));
        command.current_dir(runtime_dirs.working_dir());
        let timeout = Duration::from_millis(task.policy.timeout_ms);
        let output = if let Ok(result) = tokio::time::timeout(timeout, command.output()).await {
            result.map_err(|error| WorkerSdkError::ScriptExecutionFailed(error.to_string()))?
        } else {
            cleanup();
            runtime_dirs.cleanup();
            if let Some(script_file) = &script_file {
                let _ = std::fs::remove_file(script_file);
            }
            return Err(WorkerSdkError::ScriptTimeout {
                timeout_ms: task.policy.timeout_ms,
            });
        };
        cleanup();
        runtime_dirs.cleanup();
        if let Some(script_file) = &script_file {
            let _ = std::fs::remove_file(script_file);
        }
        emit_script_output(&task, "info", &output.stdout);
        emit_script_output(&task, "error", &output.stderr);
        if self.kind == ScriptRunnerKind::Rhai
            && let Some(message) = rhai_diagnostic_message(&output.stdout, &output.stderr)
        {
            Ok(TaskOutcome::Failed(message))
        } else if output.status.success() {
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

fn write_settings(
    task: &ScriptRunnerTask,
    script_file: Option<&Path>,
    runtime_dirs: Option<&TaskRuntimeDirs>,
) -> Result<(PathBuf, impl FnOnce()), WorkerSdkError> {
    let mut allow_read = task.policy.read_only_paths.clone();
    let mut allow_write = task.policy.writable_paths.clone();
    if let Some(script_file) = script_file {
        allow_read.push(script_file.to_string_lossy().to_string());
    }
    if let Some(runtime_dirs) = runtime_dirs {
        allow_write.extend(runtime_dirs.allow_write_paths());
    }
    let settings = serde_json::json!({
        "network": {
            "allowUnixSocket": false,
            "allowedDomains": task.policy.allowed_network_hosts,
            "deniedDomains": []
        },
        "filesystem": {
            "allowRead": allow_read,
            "allowWrite": allow_write,
            "denyRead": sensitive_read_denies(),
            "denyWrite": []
        }
    });
    let path = system_temp_file("tikee-srt-settings", "json");
    std::fs::write(&path, settings.to_string())
        .map_err(|error| WorkerSdkError::ScriptExecutionFailed(error.to_string()))?;
    let cleanup_path = path.clone();
    Ok((path, move || {
        let _ = std::fs::remove_file(cleanup_path);
    }))
}

fn heredoc(command: &str, marker: &str, content: &str) -> String {
    let mut delimiter = marker.to_owned();
    while content.contains(&delimiter) {
        delimiter.push_str("_TIKEE");
    }
    format!("{command} <<'{delimiter}'\n{content}\n{delimiter}")
}

fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_default()
}

fn sensitive_read_denies() -> Vec<String> {
    let home = home_dir();
    if home.is_empty() {
        return Vec::new();
    }
    [
        ".ssh",
        ".gnupg",
        ".aws",
        ".kube",
        ".docker",
        ".config/tikee",
    ]
    .into_iter()
    .map(|path| format!("{home}/{path}"))
    .collect()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn rhai_diagnostic_message(stdout: &[u8], stderr: &[u8]) -> Option<String> {
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    );
    if !combined.contains("Syntax error:")
        && !combined.contains("Runtime error:")
        && !combined.contains("Parse error:")
    {
        return None;
    }
    let message = combined
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    Some(if message.is_empty() {
        "Rhai script reported an execution diagnostic".to_owned()
    } else {
        message
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitized_path_splits_inherited_path_entries() {
        let extra = PathBuf::from("/opt/tikee-srt/bin");
        let inherited = match std::env::join_paths([
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
        ]) {
            Ok(path) => path,
            Err(error) => panic!("test path must be joinable: {error}"),
        };

        let Some(path) = sanitized_path(std::slice::from_ref(&extra), Some(inherited)) else {
            panic!("path should be built");
        };
        let entries = std::env::split_paths(&path).collect::<Vec<_>>();

        assert_eq!(entries[0], extra);
        assert!(
            entries
                .iter()
                .any(|entry| entry == &PathBuf::from("/usr/local/bin"))
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry == &PathBuf::from("/usr/bin"))
        );
    }

    #[test]
    fn srt_settings_do_not_mask_managed_runtime_under_home() {
        let task = ScriptRunnerTask {
            script_id: "script_shell".to_owned(),
            version_id: "sv_1".to_owned(),
            version_number: 1,
            language: "shell".to_owned(),
            content: "echo ok".to_owned(),
            content_sha256: "unused-for-settings-test".to_owned(),
            policy: crate::script::ScriptRunnerPolicy::default(),
            log: None,
        };

        let (settings, cleanup) = write_settings(&task, None, None)
            .unwrap_or_else(|error| panic!("settings should be written: {error}"));
        let data = std::fs::read_to_string(&settings)
            .unwrap_or_else(|error| panic!("settings should be readable: {error}"));
        cleanup();
        let json: serde_json::Value = serde_json::from_str(&data)
            .unwrap_or_else(|error| panic!("settings should be valid json: {error}"));
        let deny_read = json["filesystem"]["denyRead"]
            .as_array()
            .unwrap_or_else(|| panic!("denyRead should be an array"));

        assert!(
            !deny_read
                .iter()
                .any(|entry| entry.as_str() == Some(&home_dir()))
        );
        assert!(
            deny_read
                .iter()
                .any(|entry| entry.as_str().is_some_and(|path| path.ends_with("/.ssh")))
        );
    }

    #[test]
    fn powershell_command_starts_inside_sandbox_home_and_disables_logo() {
        let runner = SrtScriptRunner::new(
            ScriptRunnerKind::PowerShell,
            "srt",
            "pwsh",
            std::iter::empty::<PathBuf>(),
        );
        let command = runner.shell_command("Write-Output 'ok'", None);

        assert!(command.starts_with("cd \"$HOME\" && pwsh -NoLogo -NoProfile -NonInteractive"));
        assert!(command.contains("-InputFormat Text -OutputFormat Text"));
    }

    #[test]
    fn powershell_settings_allow_only_task_runtime_write_dirs() {
        let task = ScriptRunnerTask {
            script_id: "script_powershell".to_owned(),
            version_id: "sv_1".to_owned(),
            version_number: 1,
            language: "powershell".to_owned(),
            content: "Write-Output 'ok'".to_owned(),
            content_sha256: "unused-for-settings-test".to_owned(),
            policy: crate::script::ScriptRunnerPolicy::default(),
            log: None,
        };
        let runtime_dirs = TaskRuntimeDirs::create("tikee-srt-powershell-runtime-test")
            .unwrap_or_else(|error| panic!("runtime dirs should be created: {error}"));
        let (settings, cleanup) = write_settings(&task, None, Some(&runtime_dirs))
            .unwrap_or_else(|error| panic!("settings should be written: {error}"));
        let data = std::fs::read_to_string(&settings)
            .unwrap_or_else(|error| panic!("settings should be readable: {error}"));
        cleanup();
        let json: serde_json::Value = serde_json::from_str(&data)
            .unwrap_or_else(|error| panic!("settings should be valid json: {error}"));
        let allow_write = json["filesystem"]["allowWrite"]
            .as_array()
            .unwrap_or_else(|| panic!("allowWrite should be an array"));

        assert!(
            allow_write
                .iter()
                .any(|entry| entry.as_str() == Some(&runtime_dirs.root().to_string_lossy()))
        );
        assert!(
            allow_write
                .iter()
                .any(|entry| entry.as_str() == Some(&runtime_dirs.home().to_string_lossy()))
        );
        assert!(
            allow_write
                .iter()
                .any(|entry| entry.as_str() == Some(&runtime_dirs.tmp().to_string_lossy()))
        );
        assert!(
            !allow_write
                .iter()
                .any(|entry| entry.as_str() == Some(&home_dir()))
        );
        runtime_dirs.cleanup();
    }

    #[test]
    fn rhai_diagnostic_output_is_treated_as_failure_message() {
        let stdout = br"                                                   ^ Syntax error: 'case' is a reserved keyword
";
        let stderr = br#"================================================
/tmp/tikee-rhai-script.rhai
================================================
1: let result = #{ language: "rhai", status: "ok", case: "manual-acceptance" };
"#;

        let Some(message) = rhai_diagnostic_message(stdout, stderr) else {
            panic!("Rhai diagnostic output should be detected");
        };

        assert!(message.contains("Syntax error"));
        assert!(message.contains("manual-acceptance"));
    }
}
