#![forbid(unsafe_code)]

use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use tikee::{
    ContainerScriptRunner, ScriptRunnerKind, ScriptRunnerRegistry, TaskContext, TaskOutcome,
    TaskProcessor, WorkerClient, WorkerConfig, WorkerSdkError,
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
    configure_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::Shell,
        "TIKEE_SHELL_IMAGE",
        "alpine:3.20",
    );
    configure_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::Python,
        "TIKEE_PYTHON_IMAGE",
        "python:3.13-alpine",
    );
    configure_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::Node,
        "TIKEE_NODE_IMAGE",
        "node:24-alpine",
    );
    configure_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::Rhai,
        "TIKEE_RHAI_IMAGE",
        "rhaiscript/rhai:latest",
    );
    for runner in runners.structured_capabilities() {
        config.add_script_runner(runner.language, runner.sandbox_backend);
    }

    configure_script_runner(
        &mut config,
        &mut runners,
        ScriptRunnerKind::PowerShell,
        "TIKEE_POWERSHELL_IMAGE",
        "mcr.microsoft.com/powershell:latest",
    );

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
    let mut session = WorkerClient::new(config).connect().await?;
    println!(
        "Rust worker connected: worker_id={}, generation={}, lease_seconds={}",
        session.worker_id(),
        session.generation(),
        session.lease_seconds()
    );

    if enabled_env("TIKEE_WORKER_HEARTBEAT_ON_START") {
        let ping = session.heartbeat().await?;
        println!("heartbeat ack sequence={}", ping.sequence);
    }

    loop {
        let outcome = session
            .process_next_with_script_runners(&NoopProcessor, &runners)
            .await?;
        println!("processed task outcome={outcome:?}");
        if oneshot {
            session.close().await?;
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn configure_script_runner(
    config: &mut WorkerConfig,
    runners: &mut ScriptRunnerRegistry,
    kind: ScriptRunnerKind,
    image_env: &str,
    default_image: &str,
) {
    let enable_key = format!("TIKEE_ENABLE_SCRIPT_{}", kind.as_str().to_ascii_uppercase());
    if !enabled_env(&enable_key) {
        return;
    }
    let image = env_or(image_env, default_image);
    runners.register(ContainerScriptRunner::new(kind, image));
    config.add_script_runner(kind.as_str(), "container");
    config.labels.insert(
        format!("script_{}_sandbox", kind.as_str()),
        "container".to_owned(),
    );
}

struct NoopProcessor;

#[async_trait]
impl TaskProcessor for NoopProcessor {
    async fn process(&self, task: TaskContext) -> Result<TaskOutcome, WorkerSdkError> {
        let outcome = match task.processor_name.as_str() {
            "" | "demo.echo" | "demo.context" | "demo.bytes" | "demo.heartbeat"
            | "billing.sql-sync" => TaskOutcome::Succeeded,
            "demo.fail" => TaskOutcome::Failed("rust demo intentional failure".to_owned()),
            other => TaskOutcome::Failed(format!("unsupported rust demo processor: {other}")),
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
