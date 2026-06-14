---
title: Rust Worker SDK
description: Rust SDK dependency coordinates, WorkerConfig defaults, minimal Worker, Management API helpers, and live verification runbook.
---

# Rust Worker SDK

The Rust SDK is the closest language surface to the Server protocol. It lives in `sdks/rust/tikeo` and re-exports `WorkerConfig`, `WorkerClient`, `TaskProcessor`, `TaskContext`, `TaskOutcome`, script runners, WASM helpers, and Management client types from `sdks/rust/tikeo/src/lib.rs`. The runnable demo lives in `examples/rust/worker-demo`.


Shared SDK/API contract: see [SDK and API integration guide](../integrations/sdk-and-api) for common concepts, unified configuration parameters, Management API semantics, Worker connection parameters, trigger types, errors/retries, and the language difference table. This language page stays focused on installation, minimal Worker code, exception behavior, and Management client syntax.

## Dependency coordinates

Source package metadata is in `sdks/rust/tikeo/Cargo.toml`:

| Field | Value |
| --- | --- |
| Crate name | `tikeo` |
| Version in repo | `0.2.0` |
| Rust edition | `2024` |
| Rust baseline | `1.95` |
| Optional feature | `wasm` enables `wasmtime` |
| Important runtime deps | `tonic`, `prost`, `tokio`, `reqwest`, `serde`, `sha2`, `tracing` |

Install from crates.io when a release is published:

```bash
cargo add tikeo@${TIKEO_VERSION}
```

Repository-local examples use the checked-out SDK. Verify it directly:

```bash
cargo fmt --manifest-path sdks/rust/tikeo/Cargo.toml -- --check
cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features
```

## WorkerConfig defaults

`sdks/rust/tikeo/src/config.rs` defines `WorkerConfig::local(endpoint, client_instance_id)`.

| Field | Default from helper | Notes |
| --- | --- | --- |
| `endpoint` | caller-provided | Demos use `http://127.0.0.1:9998`. |
| `client_instance_id` | caller-provided | Stable client hint; Server assigns authoritative `worker_id`. |
| `namespace` | `default` | Demo overrides to `dev-alpha`. |
| `app` | `default` | Demo overrides to `orders`. |
| `cluster` | `local` | Worker cluster metadata. |
| `region` | `local` | Worker region metadata. |
| `capabilities` | `[]` | Legacy metadata only. |
| `structured_capabilities` | empty `WorkerCapabilities` | Routing uses this. |
| `labels` | `{}` | Demo adds `worker_pool`. |
| `election.enabled` | `true` in register message | Sent as `WorkerClusterElection`. |
| `election.domain` | empty | Blank means namespace/app/cluster/region domain. |
| `election.priority` | `100` | Lower wins. |

Structured helpers include `add_tag`, `add_sdk_processor`, `add_script_runner`, and `add_plugin_processor`. These deduplicate and ignore blank values.

## Minimal Worker

```rust
use async_trait::async_trait;
use tikeo::{install_task_log_bridge, TaskContext, TaskOutcome, TaskProcessor, WorkerClient, WorkerConfig, WorkerSdkError};

struct Echo;

#[async_trait]
impl TaskProcessor for Echo {
    async fn process(&self, task: TaskContext) -> Result<TaskOutcome, WorkerSdkError> {
        tracing::info!(processor = %task.processor_name, instance = %task.instance_id, "rust echo processor");
        Ok(TaskOutcome::Success("rust echo processed".to_owned()))
    }
}

#[tokio::main]
async fn main() -> Result<(), WorkerSdkError> {
    let _ = install_task_log_bridge(); // captures tracing/log events only inside active task scope
    let mut config = WorkerConfig::local("http://127.0.0.1:9998", "rust-worker-1");
    config.namespace = "sdk-smoke".to_owned();
    config.app = "management".to_owned();
    config.add_sdk_processor("demo.echo");
    config.labels.insert("worker_pool".to_owned(), "rust-blue".to_owned());

    let client = WorkerClient::new(config);
    let mut session = client.connect().await?;
    loop {
        session.process_next(&Echo).await?;
    }
}
```

Use ordinary `tracing::info!/warn!/error!` in processors after installing the bridge. The bridge uses Tokio task-local scope, so non-task tracing events are ignored by instance logging. `TaskContext::log_info/log_error` remains a fallback.

Use `process_next_with_script_runners` only when you have registered real script runners. The SDK sends logs/results with the assignment token received from `DispatchTask`; do not invent your own token.

## Demo environment variables

`examples/rust/worker-demo/src/main.rs` documents the live demo shape:

| Variable | Default | Meaning |
| --- | --- | --- |
| `TIKEO_WORKER_ENDPOINT` | `http://127.0.0.1:9998` | Worker Tunnel endpoint. |
| `TIKEO_WORKER_INSTANCE_ID` / `TIKEO_WORKER_CLIENT_INSTANCE_ID` | `rust-worker-demo-local` | Stable client hint. |
| `TIKEO_WORKER_NAMESPACE` | `dev-alpha` | Demo namespace. |
| `TIKEO_WORKER_APP` | `orders` | Demo app. |
| `TIKEO_WORKER_CLUSTER` | `local` | Demo cluster. |
| `TIKEO_WORKER_REGION` | `local` | Demo region. |
| `TIKEO_WORKER_SDK_PROCESSORS` | `demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail,demo.exception` | Structured SDK processors. |
| `TIKEO_WORKER_POOL` | `rust-blue` | Stored as label `worker_pool`. |
| `TIKEO_WORKER_DRY_RUN` | unset | Set to `1` to avoid live tunnel. |
| `TIKEO_WORKER_ONESHOT` | unset | Exit after one processed task. |
| `TIKEO_SANDBOX_AUTO_INSTALL` | enabled unless disabled | Controls sandbox tool auto-install. |

Run:

```bash
TIKEO_WORKER_CONNECT=1 \
TIKEO_WORKER_NAMESPACE=sdk-smoke \
TIKEO_WORKER_APP=management \
TIKEO_WORKER_SDK_PROCESSORS=demo.echo \
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

## Management API create + trigger

```rust
use tikeo::{ManagementClient, ManagementCreateJobRequest, ManagementTriggerJobRequest};

let endpoint = std::env::var("TIKEO_MANAGEMENT_ENDPOINT")
    .unwrap_or_else(|_| "http://127.0.0.1:9090".to_owned());
let api_key = std::env::var("TIKEO_MANAGEMENT_API_KEY")?;
let management = ManagementClient::new(endpoint, api_key, "sdk-smoke", "management");

let created = management
    .create_job(ManagementCreateJobRequest::api("rust-echo-api", "demo.echo"))
    .await?;
let instance = management
    .trigger_job(&created.id, ManagementTriggerJobRequest::api())
    .await?;

assert_eq!(instance.trigger_type, "api");
assert_eq!(instance.execution_mode, "single");
```

Broadcast is explicit:

```rust
use tikeo::ManagementBroadcastSelectorRequest;

let selector = ManagementBroadcastSelectorRequest {
    tags: Some(vec!["manual-demo".to_owned()]),
    region: Some("local".to_owned()),
    cluster: Some("local".to_owned()),
    labels: Some(std::collections::HashMap::from([("worker_pool".to_owned(), "rust-blue".to_owned())])),
};
let request = ManagementTriggerJobRequest::broadcast_api(Some(selector));
```

## Management client credentials

All SDK Management clients use app-scoped service credentials. They send the `x-tikeo-api-key` header, normally sourced from `TIKEO_MANAGEMENT_API_KEY`. Do not confuse this key with a human bearer token from `/api/v1/auth/login`, and do not reuse browser sessions or OIDC provider tokens in SDK services.

The common create+trigger default is:

| Field | Default helper behavior |
| --- | --- |
| Job schedule | `scheduleType=api` |
| Job enabled | `true` |
| Retry policy | `enabled=true`, `maxAttempts=3`, `initialDelaySeconds=5`, `backoffMultiplier=2`, `maxDelaySeconds=60` |
| Trigger source | `triggerType=api` |
| Trigger execution mode | `executionMode=single` |
| Broadcast | Opt-in only through explicit broadcast helper and `broadcastSelector` |

## Operator-verified reference links

Keep SDK helper docs anchored to operator-verified API and protocol references:

- Create helper endpoint: [`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- Trigger helper endpoint: [`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- Instance polling endpoint: [`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- Instance log endpoint: [`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker dispatch message: [`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## Live verification runbook

1. Start the Server with `cargo run --bin tikeo -- serve --config config/dev.toml`.
2. Bootstrap an Owner or login to an existing local Owner.
3. Create namespace/app/worker pool, service account, and SDK API key as shown in the quickstart.
4. Start the language demo Worker with matching namespace/app and `TIKEO_WORKER_CONNECT=1` when the demo supports live mode.
5. Create and trigger an API job through the language Management client.
6. Inspect `/api/v1/workers`, `/api/v1/instances`, instance logs, and audit logs.
7. Preserve smoke evidence. For a maintained end-to-end proof, run `TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh`.

Expected acceptance evidence includes an online worker with the requested structured processor, an API-triggered instance with `executionMode=single`, task logs from the Worker, and a successful processor message. Missing sandbox tools or unsupported processors must fail closed and be visible in task/diagnostic logs.

## Failure and exception demos

All language demos now separate business failure from runtime exceptions. `demo.fail` returns a normal failed `TaskOutcome` and is used to verify business-rule failure handling. `demo.exception` throws, panics, raises, or returns a processor error so the SDK can capture a real runtime stack trace and send it as task logs while still reporting a failed task result. Use both processors during acceptance: the first proves expected business failure semantics; the second proves operator-visible stack traces survive the Worker Tunnel and Notification Center trace page.

## Capability discipline

The dispatch contract uses structured capabilities, not folklore or only string naming conventions. A Worker should advertise SDK processors, plugin processors, script runners, labels, and tags only when the runtime can really execute them. Do not advertise SQL, shell, Python, Node.js, WASM, SRT, Deno, Docker, or Podman support just because a package exists; advertise it after the demo or service has resolved the tool and can fail safely.

## Prerequisites

Use the setup, authentication, and access requirements described in this page before running any command. For local examples, start the Server with `config/dev.toml`, use `127.0.0.1` as the client host, and keep tokens in shell variables rather than pasted into files.

## Verify

After following the page, verify the result with the documented API, UI, build, smoke, or deployment checks. A valid verification includes the command that was run, the route or file that was inspected, and the observed status or artifact.

## Troubleshooting

When a step fails, first capture the exact command, response status, and Server log window. Then check authentication, namespace/app scope, Worker eligibility, storage readiness, and proxy behavior before changing production configuration.

## Production checklist

- [ ] Secrets are referenced through environment or platform secret mechanisms and are not written into examples.
- [ ] Commands have been adapted from local `127.0.0.1` to the real host, TLS, and authentication model.
- [ ] Rollback and evidence collection are documented for the changed surface.
- [ ] Operators can repeat the verification without private shell history or hidden state.
