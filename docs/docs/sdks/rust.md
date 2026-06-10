---
title: Rust Worker SDK
description: Verified Rust SDK and Worker demo entry points.
---

# Rust Worker SDK

The Rust SDK lives under `sdks/rust/tikeo`, and the runnable worker demo lives under `examples/rust/worker-demo`.


## Install from crates.io

Replace `${TIKEO_VERSION}` with the version shown by the top README `Rust SDK` badge. Rust uses the plain version string without a leading `v`.

```bash
cargo add tikeo@${TIKEO_VERSION}
```

```toml
[dependencies]
tikeo = "${TIKEO_VERSION}"
```

## Verify the SDK

```bash
cargo fmt --manifest-path sdks/rust/tikeo/Cargo.toml -- --check
cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features
```

## Run the demo

```bash
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

The demo is expected to connect to the Worker Tunnel endpoint from local configuration when run in live mode.


## Management API create + trigger

The Rust management client is source-backed by `sdks/rust/tikeo/src/management.rs`. It is for app-scoped service credentials only: the SDK sends `x-tikeo-api-key`, normally loaded from `TIKEO_MANAGEMENT_API_KEY`, and does not reuse browser sessions, OIDC cookies, or user-scoped bearer tokens. A created API job uses `scheduleType=api`; the default trigger helper sends `triggerType=api` and `executionMode=single`.

```rust
use tikeo::{
    ManagementBroadcastSelectorRequest,
    ManagementClient,
    ManagementCreateJobRequest,
    ManagementTriggerJobRequest,
};

let endpoint = std::env::var("TIKEO_MANAGEMENT_ENDPOINT")
    .unwrap_or_else(|_| "http://127.0.0.1:9090".to_owned());
let api_key = std::env::var("TIKEO_MANAGEMENT_API_KEY")?;
let management = ManagementClient::new(endpoint, api_key, "dev-alpha", "orders");

let created = management
    .create_job(ManagementCreateJobRequest::api("rust-echo-api", "demo.echo"))
    .await?;
let instance = management
    .trigger_job(&created.id, ManagementTriggerJobRequest::api())
    .await?;

assert_eq!(instance.trigger_type, "api");
assert_eq!(instance.execution_mode, "single");
```

Broadcast is intentionally not the default. Use the explicit selector helper only when one API trigger should fan out to multiple matching workers; it serializes `broadcastSelector` with `executionMode=broadcast`.

```rust
let broadcast = ManagementTriggerJobRequest::broadcast_api(Some(
    ManagementBroadcastSelectorRequest {
        tags: Some(vec!["manual-demo".to_owned()]),
        region: Some("us-east-1".to_owned()),
        cluster: None,
        labels: Some(std::collections::HashMap::from([(
            "worker_pool".to_owned(),
            "rust-blue".to_owned(),
        )])),
    },
));
let _instance = management.trigger_job(&created.id, broadcast).await?;
```


## Source-backed reference links

Keep SDK helper docs anchored to source-derived API and protocol references:

- Create helper endpoint: [`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- Trigger helper endpoint: [`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- Instance polling endpoint: [`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- Instance log endpoint: [`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker dispatch message: [`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## Minimal worker mental model

A Rust worker owns three responsibilities: connect to the Server tunnel, advertise only the capabilities it can really execute, and return logs/results with the assignment token supplied by the Server. This keeps scheduling, audit, and stale-worker fencing aligned.

## Capability discipline

Do not advertise a processor, script backend, or plugin capability unless the worker can execute it. Unsupported runtimes should fail closed. This rule is important because the Server schedules work from capability snapshots and persisted worker session state.

## Evaluation checklist

- Run SDK tests with all enabled features.
- Run the worker demo while the Server is listening on the Worker Tunnel port.
- Confirm the Web console shows the worker session and capability snapshot.
- Trigger a job that routes to the demo processor.
- Inspect instance logs and result status.

## Production notes

For production, package workers independently from the Server image. Worker identity should be scoped through namespace, app, worker pool, labels, and structured capabilities, not ad-hoc naming conventions.

## Version and packaging notes

The public README declares the Rust runtime baseline. Keep SDK docs, demo manifests, and CI toolchain setup aligned when that baseline changes.
