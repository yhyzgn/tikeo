#![forbid(unsafe_code)]

use scheduler_worker_sdk::WorkerConfig;

#[tokio::main]
async fn main() {
    let endpoint = std::env::var("SCHEDULER_WORKER_ENDPOINT")
        .unwrap_or_else(|_| "http://0.0.0.0:9998".to_owned());
    let worker_id = std::env::var("SCHEDULER_WORKER_ID")
        .unwrap_or_else(|_| "rust-demo-worker".to_owned());
    let config = WorkerConfig::local(endpoint, worker_id);
    println!(
        "Rust worker demo configured: worker_id={}, endpoint={}",
        config.worker_id, config.endpoint
    );
    println!("Start scheduler server and replace this dry run with WorkerClient::connect() when needed.");
}
