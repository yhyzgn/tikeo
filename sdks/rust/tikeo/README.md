# tikeo Rust Worker SDK 🦀

[🇨🇳 中文 SDK 文档](../../../README.zh-CN.md#行为一致的-sdk)

Rust SDK for active outbound Tikeo Worker Tunnel connections.

## Runtime requirements

- Rust 1.95+ is required (`rust-version = "1.95"`).
- The crate uses Rust 2024 edition and forbids unsafe code.

## Features

- Outbound gRPC Worker Tunnel registration, heartbeat, task result, and unregister flow.
- Structured worker capabilities: normal processors, plugin processors, script runners, and tags.
- Task-scoped `tracing` / `log` bridge that sends processor events precisely to the current job instance.
- SDK diagnostics with `SdkLogConfig`, default `INFO`, console output, and optional `tikeo-sdk.log`.
- App-scoped management client using `x-tikeo-api-key`.
- Script sandbox runners for SRT, Deno, containers, local diagnostics, and fail-closed unavailable handlers.

## Usage

```rust,no_run
use tikeo::{WorkerClient, WorkerConfig, TaskOutcome, configure_sdk_logging, install_task_log_bridge, SdkLogConfig};

configure_sdk_logging(SdkLogConfig::info().with_log_dir("./logs"));
let _ = install_task_log_bridge();

let mut config = WorkerConfig::local("http://127.0.0.1:9998", "orders-rust-1");
config.namespace = "dev-alpha".into();
config.app = "orders".into();
config.add_normal_processor("demo.echo", "Echo payload demo processor");

let client = WorkerClient::new(config);
```

## Operational cautions

- Sandbox auto-install is background prewarm only: SDK startup never waits for downloads; missing tools stay unadvertised and fail closed until available.
- Set `TIKEO_SANDBOX_STRICT_ISOLATION=1` when strict sandbox isolation is required; this skips host `PATH` tools/interpreters and uses only sandbox-tools cache binaries.
- The server assigns the authoritative `worker_id`; `client_instance_id` is only a stable hint.
- Keep SDK diagnostics at `INFO` in production and switch to `DEBUG` only while troubleshooting.
- Prefer `tracing::info!/warn!/error!` in processors after `install_task_log_bridge()`; `TaskContext::log_info` / `log_error` remains a direct fallback. Do not capture global stdout.
- Do not advertise a script runner unless the corresponding sandbox backend is actually available.
- Docker/Podman are explicit heavy backends and are not selected by the default auto path.

## Verification

```bash
cargo test --all-features
cargo clippy --all-features -- -D warnings
```
