# tikeo Rust Worker SDK 🦀

[🇨🇳 中文 SDK 文档](../../../docs/zh-CN/sdk.md)

Rust SDK for active outbound Tikeo Worker Tunnel connections.

## Features

- Outbound gRPC Worker Tunnel registration, heartbeat, task result, and unregister flow.
- Structured worker capabilities: SDK processors, plugin processors, script runners, and tags.
- Task-scoped logs that are sent precisely to the current job instance.
- SDK diagnostics with `SdkLogConfig`, default `INFO`, console output, and optional `tikeo-sdk.log`.
- App-scoped management client using `x-tikeo-api-key`.
- Script sandbox runners for SRT, Deno, containers, local diagnostics, and fail-closed unavailable handlers.

## Usage

```rust,no_run
use tikeo::{WorkerClient, WorkerConfig, TaskOutcome, configure_sdk_logging, SdkLogConfig};

configure_sdk_logging(SdkLogConfig::info().with_log_dir("./logs"));

let mut config = WorkerConfig::local("http://127.0.0.1:9998", "orders-rust-1");
config.namespace = "dev-alpha".into();
config.app = "orders".into();
config.add_sdk_processor("demo.echo");

let client = WorkerClient::new(config);
```

## Operational cautions

- The server assigns the authoritative `worker_id`; `client_instance_id` is only a stable hint.
- Keep SDK diagnostics at `INFO` in production and switch to `DEBUG` only while troubleshooting.
- Emit task output through `TaskContext::log_info` / `log_error`; do not capture global stdout.
- Do not advertise a script runner unless the corresponding sandbox backend is actually available.
- Docker/Podman are explicit heavy backends and are not selected by the default auto path.

## Verification

```bash
cargo test --all-features
cargo clippy --all-features -- -D warnings
```
