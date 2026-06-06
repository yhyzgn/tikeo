#![forbid(unsafe_code)]

use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use tikee::{
    ContainerScriptRunner, DenoScriptRunner, SandboxToolResolver, ScriptRunnerKind,
    ScriptRunnerRegistry, SrtScriptRunner, TaskContext, TaskOutcome, TaskProcessor,
    UnsupportedScriptRunner, WorkerClient, WorkerConfig, WorkerSdkError,
};

#[tokio::main]
async fn main() -> Result<(), WorkerSdkError> {
    let endpoint = std::env::var("TIKEE_WORKER_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:9998".to_owned());
    let client_instance_id = std::env::var("TIKEE_WORKER_INSTANCE_ID")
        .or_else(|_| std::env::var("TIKEE_WORKER_CLIENT_INSTANCE_ID"))
        .unwrap_or_else(|_| "rust-worker-demo-local".to_owned());
    let mut config = WorkerConfig::local(endpoint, client_instance_id);
    config.namespace = env_or("TIKEE_WORKER_NAMESPACE", "dev-alpha");
    config.app = env_or("TIKEE_WORKER_APP", "orders");
    config.cluster = env_or("TIKEE_WORKER_CLUSTER", "local");
    config.region = env_or("TIKEE_WORKER_REGION", "local");
    config.capabilities = csv_env("TIKEE_WORKER_CAPABILITIES");
    config.labels = labels_env("TIKEE_WORKER_LABELS");
    config.add_tag("rust");
    config.add_tag("manual-demo");
    for processor in csv_env_or(
        "TIKEE_WORKER_SDK_PROCESSORS",
        "demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail",
    ) {
        config.add_sdk_processor(processor);
    }
    config.labels.insert(
        "worker_pool".to_owned(),
        env_or("TIKEE_WORKER_POOL", "rust-blue"),
    );
    if enabled_by_default("TIKEE_ENABLE_PLUGIN_SQL") {
        config.add_plugin_processor(
            env_or("TIKEE_PLUGIN_SQL_TYPE", "sql"),
            env_or("TIKEE_PLUGIN_SQL_PROCESSOR", "billing.sql-sync"),
        );
        config
            .labels
            .insert("plugin_sql".to_owned(), "enabled".to_owned());
    }

    let mut runners = ScriptRunnerRegistry::new();
    let sandbox_backend = env_or("TIKEE_WORKER_SCRIPT_SANDBOX", "auto");
    let mut sandbox_tools = SandboxToolResolver::default();
    if let Ok(state_dir) = std::env::var("TIKEE_WORKER_STATE_DIR") {
        sandbox_tools.state_dir = Some(std::path::PathBuf::from(state_dir));
    }
    sandbox_tools.auto_install = !disabled_env("TIKEE_SANDBOX_AUTO_INSTALL");
    configure_default_script_runner(
        &mut config,
        &mut runners,
        &sandbox_tools,
        ScriptRunnerConfig {
            kind: ScriptRunnerKind::Shell,
            enable_env: "TIKEE_ENABLE_SCRIPT_SHELL",
            sandbox_backend: &sandbox_backend,
            image_env: "TIKEE_SHELL_IMAGE",
            default_image: "alpine:3.20",
        },
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        &sandbox_tools,
        ScriptRunnerConfig {
            kind: ScriptRunnerKind::Python,
            enable_env: "TIKEE_ENABLE_SCRIPT_PYTHON",
            sandbox_backend: &sandbox_backend,
            image_env: "TIKEE_PYTHON_IMAGE",
            default_image: "python:3.13-alpine",
        },
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        &sandbox_tools,
        ScriptRunnerConfig {
            kind: ScriptRunnerKind::Js,
            enable_env: "TIKEE_ENABLE_SCRIPT_JAVASCRIPT",
            sandbox_backend: &sandbox_backend,
            image_env: "TIKEE_JAVASCRIPT_IMAGE",
            default_image: "denoland/deno:alpine",
        },
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        &sandbox_tools,
        ScriptRunnerConfig {
            kind: ScriptRunnerKind::Ts,
            enable_env: "TIKEE_ENABLE_SCRIPT_TYPESCRIPT",
            sandbox_backend: &sandbox_backend,
            image_env: "TIKEE_TYPESCRIPT_IMAGE",
            default_image: "denoland/deno:alpine",
        },
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        &sandbox_tools,
        ScriptRunnerConfig {
            kind: ScriptRunnerKind::PowerShell,
            enable_env: "TIKEE_ENABLE_SCRIPT_POWERSHELL",
            sandbox_backend: &sandbox_backend,
            image_env: "TIKEE_POWERSHELL_IMAGE",
            default_image: "mcr.microsoft.com/powershell:latest",
        },
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        &sandbox_tools,
        ScriptRunnerConfig {
            kind: ScriptRunnerKind::Php,
            enable_env: "TIKEE_ENABLE_SCRIPT_PHP",
            sandbox_backend: &sandbox_backend,
            image_env: "TIKEE_PHP_IMAGE",
            default_image: "php:8.4-cli-alpine",
        },
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        &sandbox_tools,
        ScriptRunnerConfig {
            kind: ScriptRunnerKind::Groovy,
            enable_env: "TIKEE_ENABLE_SCRIPT_GROOVY",
            sandbox_backend: &sandbox_backend,
            image_env: "TIKEE_GROOVY_IMAGE",
            default_image: "groovy:4-jdk21-alpine",
        },
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        &sandbox_tools,
        ScriptRunnerConfig {
            kind: ScriptRunnerKind::Rhai,
            enable_env: "TIKEE_ENABLE_SCRIPT_RHAI",
            sandbox_backend: &sandbox_backend,
            image_env: "TIKEE_RHAI_IMAGE",
            default_image: "rhaiscript/rhai:latest",
        },
    );
    for runner in runners.structured_capabilities() {
        config.add_script_runner(runner.language, runner.sandbox_backend);
    }

    println!(
        "Rust worker demo configured: client_instance_id={}, endpoint={}, namespace={}, app={}, cluster={}, region={}, structured_capabilities={:?}, legacy_capabilities={:?}, labels={:?}",
        config.client_instance_id,
        config.endpoint,
        config.namespace,
        config.app,
        config.cluster,
        config.region,
        config.structured_capabilities,
        config.capabilities,
        config.labels
    );

    if dry_run_enabled() {
        println!(
            "Dry run only. Set TIKEE_WORKER_DRY_RUN=0 or omit it to open a live Worker Tunnel; set TIKEE_ENABLE_SCRIPT_<LANG>=1 to advertise script runners."
        );
        return Ok(());
    }

    let oneshot = enabled_env("TIKEE_WORKER_ONESHOT");
    let client = WorkerClient::new(config);
    loop {
        if run_worker_session(client.clone(), &runners, oneshot).await? {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn run_worker_session(
    client: WorkerClient,
    runners: &ScriptRunnerRegistry,
    oneshot: bool,
) -> Result<bool, WorkerSdkError> {
    let mut session = match client.connect().await {
        Ok(session) => session,
        Err(error) => {
            eprintln!("Rust worker connect failed, retrying: {error}");
            return Ok(false);
        }
    };
    println!(
        "Rust worker connected: worker_id={}, generation={}, lease_seconds={}",
        session.worker_id(),
        session.generation(),
        session.lease_seconds()
    );

    if enabled_env("TIKEE_WORKER_HEARTBEAT_ON_START") {
        match session.heartbeat().await {
            Ok(ping) => println!("heartbeat ack sequence={}", ping.sequence),
            Err(error) => {
                eprintln!("heartbeat-on-start failed, reconnecting: {error}");
                let _ = session.close().await;
                return Ok(false);
            }
        }
    }

    loop {
        match session
            .process_next_with_script_runners(&DemoProcessor, runners)
            .await
        {
            Ok(outcome) => {
                println!("processed task outcome={outcome:?}");
                if oneshot {
                    session.close().await?;
                    return Ok(true);
                }
            }
            Err(error) => {
                eprintln!("worker tunnel ended, reconnecting: {error}");
                let _ = session.close().await;
                return Ok(false);
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

struct ScriptRunnerConfig<'a> {
    kind: ScriptRunnerKind,
    enable_env: &'a str,
    sandbox_backend: &'a str,
    image_env: &'a str,
    default_image: &'a str,
}

fn configure_default_script_runner(
    config: &mut WorkerConfig,
    runners: &mut ScriptRunnerRegistry,
    sandbox_tools: &SandboxToolResolver,
    runner_config: ScriptRunnerConfig<'_>,
) {
    if disabled_env(runner_config.enable_env) {
        return;
    }
    register_script_runner(config, runners, sandbox_tools, runner_config);
}

fn register_script_runner(
    config: &mut WorkerConfig,
    runners: &mut ScriptRunnerRegistry,
    sandbox_tools: &SandboxToolResolver,
    runner_config: ScriptRunnerConfig<'_>,
) {
    let kind = runner_config.kind;
    let image = env_or(runner_config.image_env, runner_config.default_image);
    let backend = resolve_sandbox_backend(kind, runner_config.sandbox_backend);
    if backend == "srt" {
        if let (Some(srt), Some(rg)) =
            (sandbox_tools.resolve_srt(), sandbox_tools.resolve_ripgrep())
        {
            if let Some(interpreter) = resolve_srt_interpreter(kind, sandbox_tools) {
                let extra_path = sandbox_tool_path_entries(&srt, &rg, &interpreter, sandbox_tools);
                runners.register(SrtScriptRunner::new(kind, srt, interpreter, extra_path));
            } else {
                runners.register(UnsupportedScriptRunner::new(
                    kind,
                    format!(
                        "{} SRT interpreter is unavailable; install it or use an explicit container backend",
                        kind.as_str()
                    ),
                ));
            }
        } else {
            runners.register(UnsupportedScriptRunner::new(
                kind,
                "SRT sandbox runtime or ripgrep dependency is unavailable",
            ));
        }
    } else if backend == "deno" || backend == "v8" {
        if let Some(deno) = sandbox_tools.resolve_deno() {
            runners.register(DenoScriptRunner::new(kind, deno));
        } else {
            runners.register(UnsupportedScriptRunner::new(
                kind,
                "Deno sandbox runtime is unavailable",
            ));
        }
    } else if backend == "docker" || backend == "podman" {
        runners.register(ContainerScriptRunner::with_runtime(
            kind,
            backend.clone(),
            image,
            std::iter::empty::<String>(),
        ));
    } else {
        runners.register(UnsupportedScriptRunner::new(
            kind,
            format!("{backend} backend is unavailable in this Rust demo worker"),
        ));
    }
    if runners
        .structured_capabilities()
        .iter()
        .any(|runner| runner.language == kind.as_str())
    {
        config
            .labels
            .insert(format!("script_{}_sandbox", kind.as_str()), backend);
    }
}

fn resolve_sandbox_backend(kind: ScriptRunnerKind, requested: &str) -> String {
    let normalized = requested.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "auto" => match kind {
            ScriptRunnerKind::Js | ScriptRunnerKind::Ts => "deno".to_owned(),
            _ => "srt".to_owned(),
        },
        "container" => "docker".to_owned(),
        "docker" | "podman" | "srt" | "deno" | "v8" | "wasmtime" | "wasmedge" | "custom" => {
            normalized
        }
        other => other.to_owned(),
    }
}

fn sandbox_tool_path_entries(
    srt: &std::path::Path,
    rg: &std::path::Path,
    interpreter: &std::path::Path,
    sandbox_tools: &SandboxToolResolver,
) -> Vec<std::path::PathBuf> {
    let mut entries = Vec::new();
    for command in [
        Some(srt.to_path_buf()),
        Some(rg.to_path_buf()),
        Some(interpreter.to_path_buf()),
        sandbox_tools.resolve_node(),
        sandbox_tools.resolve_npm(),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(parent) = command.parent()
            && !entries
                .iter()
                .any(|entry: &std::path::PathBuf| entry == parent)
        {
            entries.push(parent.to_path_buf());
        }
    }
    entries
}

fn resolve_srt_interpreter(
    kind: ScriptRunnerKind,
    sandbox_tools: &SandboxToolResolver,
) -> Option<std::path::PathBuf> {
    match kind {
        ScriptRunnerKind::Rhai => sandbox_tools.resolve_rhai(),
        ScriptRunnerKind::Shell => sandbox_tools.resolve_interpreter("sh"),
        ScriptRunnerKind::Python => sandbox_tools.resolve_interpreter("python3"),
        ScriptRunnerKind::PowerShell => sandbox_tools.resolve_powershell(),
        ScriptRunnerKind::Php => sandbox_tools.resolve_interpreter("php"),
        ScriptRunnerKind::Groovy => sandbox_tools.resolve_interpreter("groovy"),
        ScriptRunnerKind::Js | ScriptRunnerKind::Ts => sandbox_tools.resolve_deno(),
    }
}

struct DemoProcessor;

#[async_trait]
impl TaskProcessor for DemoProcessor {
    async fn process(&self, task: TaskContext) -> Result<TaskOutcome, WorkerSdkError> {
        task.log_info(format!(
            "[rust-worker] processor={} instance={} payload_bytes={}",
            task.processor_name,
            task.instance_id,
            task.payload.len()
        ));
        let outcome = match task.processor_name.as_str() {
            "" | "demo.echo" => {
                task.log_info(format!(
                    "[demo.echo] payload='{}'",
                    String::from_utf8_lossy(&task.payload)
                ));
                TaskOutcome::Success("rust demo echo processed".to_owned())
            }
            "demo.context" => {
                task.log_info(format!(
                    "[demo.context] jobId={} instanceId={}",
                    task.job_id, task.instance_id
                ));
                TaskOutcome::Success(format!(
                    "rust demo context processed instance={}",
                    task.instance_id
                ))
            }
            "demo.bytes" => {
                task.log_info(format!(
                    "[demo.bytes] payload='{}' length={}",
                    String::from_utf8_lossy(&task.payload),
                    task.payload.len()
                ));
                TaskOutcome::Success(format!(
                    "rust demo bytes processed payload_bytes={}",
                    task.payload.len()
                ))
            }
            "demo.heartbeat" => {
                task.log_info(format!(
                    "[demo.heartbeat] tick jobId={} instanceId={}",
                    task.job_id, task.instance_id
                ));
                TaskOutcome::Success("rust demo heartbeat processed".to_owned())
            }
            "billing.sql-sync" => {
                task.log_info(format!(
                    "[billing.sql-sync] plugin SQL processor received payload='{}'",
                    String::from_utf8_lossy(&task.payload)
                ));
                TaskOutcome::Success("rust demo sql plugin processed".to_owned())
            }
            "demo.fail" => {
                task.log_error(format!(
                    "[demo.fail] intentional failure payload='{}'",
                    String::from_utf8_lossy(&task.payload)
                ));
                TaskOutcome::Failed("rust demo intentional failure".to_owned())
            }
            other => {
                task.log_error(format!("[rust-worker] unsupported processor={other}"));
                TaskOutcome::Failed(format!("unsupported rust demo processor: {other}"))
            }
        };
        Ok(outcome)
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

fn dry_run_enabled() -> bool {
    enabled_env("TIKEE_WORKER_DRY_RUN") || disabled_env("TIKEE_WORKER_CONNECT")
}

fn enabled_by_default(key: &str) -> bool {
    !disabled_env(key)
}

fn enabled_env(key: &str) -> bool {
    matches!(
        std::env::var(key)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn disabled_env(key: &str) -> bool {
    matches!(
        std::env::var(key)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "0" | "false" | "no" | "off"
    )
}

fn csv_env(key: &str) -> Vec<String> {
    std::env::var(key)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn csv_env_or(key: &str, fallback: &str) -> Vec<String> {
    let value = std::env::var(key).unwrap_or_else(|_| fallback.to_owned());
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn labels_env(key: &str) -> HashMap<String, String> {
    std::env::var(key)
        .unwrap_or_default()
        .split(',')
        .filter_map(|entry| entry.split_once('='))
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .filter(|(key, value)| !key.is_empty() && !value.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_sandbox_backend_matches_java_lightweight_defaults() {
        assert_eq!(
            resolve_sandbox_backend(ScriptRunnerKind::Python, "auto"),
            "srt"
        );
        assert_eq!(
            resolve_sandbox_backend(ScriptRunnerKind::Js, "auto"),
            "deno"
        );
        assert_eq!(resolve_sandbox_backend(ScriptRunnerKind::Ts, ""), "deno");
    }

    #[test]
    fn srt_path_entries_include_runtime_dependencies() {
        let temp_root = std::env::temp_dir().join(format!(
            "tikee-rust-worker-demo-path-test-{}",
            std::process::id()
        ));
        let srt_dir = temp_root.join("srt-bin");
        let rg_dir = temp_root.join("rg-bin");
        std::fs::create_dir_all(&srt_dir).expect("create fake srt dir");
        std::fs::create_dir_all(&rg_dir).expect("create fake rg dir");

        let entries = sandbox_tool_path_entries(
            &srt_dir.join("srt"),
            &rg_dir.join("rg"),
            &srt_dir.join("sh"),
            &SandboxToolResolver::default(),
        );

        assert!(
            entries.iter().any(|entry| entry == &srt_dir),
            "SRT launcher directory must be injected into sanitized PATH: {entries:?}"
        );
        assert!(
            entries.iter().any(|entry| entry == &rg_dir),
            "ripgrep directory must be injected into sanitized PATH: {entries:?}"
        );

        for optional_command in ["node", "npm"] {
            if let Some(command) = find_command_on_path(optional_command)
                && let Some(parent) = command.parent()
            {
                assert!(
                    entries.iter().any(|entry| entry == parent),
                    "{optional_command} directory must be preserved for npm-installed SRT launchers: {entries:?}"
                );
            }
        }

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn missing_srt_interpreter_is_not_resolved() {
        assert!(
            SandboxToolResolver::default()
                .resolve_interpreter("definitely-missing-tikee-interpreter")
                .is_none()
        );
    }

    #[test]
    fn powershell_uses_managed_sandbox_tool_resolver() {
        let source = std::fs::read_to_string(file!()).expect("read rust demo source");
        assert!(
            source.contains("ScriptRunnerKind::PowerShell => sandbox_tools.resolve_powershell()")
        );
        assert!(!source.contains(
            "ScriptRunnerKind::PowerShell => sandbox_tools.resolve_interpreter(\"pwsh\")"
        ));
    }

    fn find_command_on_path(binary: &str) -> Option<std::path::PathBuf> {
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .map(|entry| entry.join(binary))
            .find(|candidate| candidate.exists())
    }
}
