#![forbid(unsafe_code)]

use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use tikee::{
    ContainerScriptRunner, ScriptRunnerKind, ScriptRunnerRegistry, TaskContext, TaskOutcome,
    TaskProcessor, UnsupportedScriptRunner, WorkerClient, WorkerConfig, WorkerSdkError,
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
    configure_default_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::Shell,
        "TIKEE_ENABLE_SCRIPT_SHELL",
        &sandbox_backend,
        "TIKEE_SHELL_IMAGE",
        "alpine:3.20",
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::Python,
        "TIKEE_ENABLE_SCRIPT_PYTHON",
        &sandbox_backend,
        "TIKEE_PYTHON_IMAGE",
        "python:3.13-alpine",
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::Js,
        "TIKEE_ENABLE_SCRIPT_JAVASCRIPT",
        &sandbox_backend,
        "TIKEE_JAVASCRIPT_IMAGE",
        "denoland/deno:alpine",
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::Ts,
        "TIKEE_ENABLE_SCRIPT_TYPESCRIPT",
        &sandbox_backend,
        "TIKEE_TYPESCRIPT_IMAGE",
        "denoland/deno:alpine",
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::PowerShell,
        "TIKEE_ENABLE_SCRIPT_POWERSHELL",
        &sandbox_backend,
        "TIKEE_POWERSHELL_IMAGE",
        "mcr.microsoft.com/powershell:latest",
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::Php,
        "TIKEE_ENABLE_SCRIPT_PHP",
        &sandbox_backend,
        "TIKEE_PHP_IMAGE",
        "php:8.4-cli-alpine",
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::Groovy,
        "TIKEE_ENABLE_SCRIPT_GROOVY",
        &sandbox_backend,
        "TIKEE_GROOVY_IMAGE",
        "groovy:4-jdk21-alpine",
    );
    configure_default_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::Rhai,
        "TIKEE_ENABLE_SCRIPT_RHAI",
        &sandbox_backend,
        "TIKEE_RHAI_IMAGE",
        "rhaiscript/rhai:latest",
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

fn configure_default_script_runner(
    config: &mut WorkerConfig,
    runners: &mut ScriptRunnerRegistry,
    kind: ScriptRunnerKind,
    enable_env: &str,
    sandbox_backend: &str,
    image_env: &str,
    default_image: &str,
) {
    if disabled_env(enable_env) {
        return;
    }
    register_script_runner(
        config,
        runners,
        kind,
        sandbox_backend,
        image_env,
        default_image,
    );
}

fn register_script_runner(
    config: &mut WorkerConfig,
    runners: &mut ScriptRunnerRegistry,
    kind: ScriptRunnerKind,
    sandbox_backend: &str,
    image_env: &str,
    default_image: &str,
) {
    let image = env_or(image_env, default_image);
    let backend = resolve_sandbox_backend(kind, sandbox_backend);
    if backend == "docker" || backend == "podman" {
        runners.register(ContainerScriptRunner::with_runtime(
            kind,
            backend.clone(),
            image,
            std::iter::empty::<String>(),
        ));
    } else {
        runners.register(UnsupportedScriptRunner::new(
            kind,
            format!(
                "{backend} backend is declared for Java parity but no Rust runner is configured"
            ),
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

struct DemoProcessor;

#[async_trait]
impl TaskProcessor for DemoProcessor {
    async fn process(&self, task: TaskContext) -> Result<TaskOutcome, WorkerSdkError> {
        println!(
            "[rust-worker] processor={} instance={} payload_bytes={}",
            task.processor_name,
            task.instance_id,
            task.payload.len()
        );
        let outcome = match task.processor_name.as_str() {
            "" | "demo.echo" => {
                println!(
                    "[demo.echo] payload='{}'",
                    String::from_utf8_lossy(&task.payload)
                );
                TaskOutcome::Success("rust demo echo processed".to_owned())
            }
            "demo.context" => {
                println!(
                    "[demo.context] jobId={} instanceId={}",
                    task.job_id, task.instance_id
                );
                TaskOutcome::Success(format!(
                    "rust demo context processed instance={}",
                    task.instance_id
                ))
            }
            "demo.bytes" => {
                println!(
                    "[demo.bytes] payload='{}' length={}",
                    String::from_utf8_lossy(&task.payload),
                    task.payload.len()
                );
                TaskOutcome::Success(format!(
                    "rust demo bytes processed payload_bytes={}",
                    task.payload.len()
                ))
            }
            "demo.heartbeat" => {
                println!(
                    "[demo.heartbeat] tick jobId={} instanceId={}",
                    task.job_id, task.instance_id
                );
                TaskOutcome::Success("rust demo heartbeat processed".to_owned())
            }
            "billing.sql-sync" => {
                println!(
                    "[billing.sql-sync] plugin SQL processor received payload='{}'",
                    String::from_utf8_lossy(&task.payload)
                );
                TaskOutcome::Success("rust demo sql plugin processed".to_owned())
            }
            "demo.fail" => {
                eprintln!(
                    "[demo.fail] intentional failure payload='{}'",
                    String::from_utf8_lossy(&task.payload)
                );
                TaskOutcome::Failed("rust demo intentional failure".to_owned())
            }
            other => {
                eprintln!("[rust-worker] unsupported processor={other}");
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
