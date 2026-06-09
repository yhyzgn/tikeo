---
title: Rust Worker SDK
description: Verified Rust SDK and Worker demo entry points.
---

# Rust Worker SDK

The Rust SDK lives under `sdks/rust/tikeo`, and the runnable worker demo lives under `examples/rust/worker-demo`.


## Install from crates.io

Replace `<TIKEO_VERSION>` with the version shown by the top README `Rust SDK` badge. Rust uses the plain version string without a leading `v`.

```bash
cargo add tikeo@<TIKEO_VERSION>
```

```toml
[dependencies]
tikeo = "<TIKEO_VERSION>"
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
