#![forbid(unsafe_code)]

use std::collections::HashMap;

use tikee::WorkerConfig;

#[tokio::main]
async fn main() {
    let endpoint = std::env::var("TIKEE_WORKER_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:9998".to_owned());
    let client_instance_id = std::env::var("TIKEE_WORKER_INSTANCE_ID")
        .unwrap_or_else(|_| "rust-demo-instance".to_owned());
    let mut config = WorkerConfig::local(endpoint, client_instance_id);
    config.namespace = env_or("TIKEE_WORKER_NAMESPACE", "default");
    config.app = env_or("TIKEE_WORKER_APP", "default");
    config.cluster = env_or("TIKEE_WORKER_CLUSTER", "local");
    config.region = env_or("TIKEE_WORKER_REGION", "local");
    config.capabilities = csv_env("TIKEE_WORKER_CAPABILITIES");
    config.labels = labels_env("TIKEE_WORKER_LABELS");
    if let Ok(worker_pool) = std::env::var("TIKEE_WORKER_POOL") {
        config.labels.insert("worker_pool".to_owned(), worker_pool);
    }
    println!(
        "Rust worker demo configured: client_instance_id={}, endpoint={}, namespace={}, app={}, cluster={}, region={}, capabilities={:?}, labels={:?}",
        config.client_instance_id,
        config.endpoint,
        config.namespace,
        config.app,
        config.cluster,
        config.region,
        config.capabilities,
        config.labels
    );
    println!(
        "Start tikee server and replace this dry run with WorkerClient::connect() when needed."
    );
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
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

fn labels_env(key: &str) -> HashMap<String, String> {
    std::env::var(key)
        .unwrap_or_default()
        .split(',')
        .filter_map(|entry| entry.split_once('='))
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .filter(|(key, value)| !key.is_empty() && !value.is_empty())
        .collect()
}
