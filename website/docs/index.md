---
title: What is Tikeo?
slug: /
description: Tikeo is a Rust-native platform for scheduled jobs, workflow DAGs, worker tunnels, multi-language workers, and governed scripts.
keywords: [rust scheduler, workflow orchestration, worker tunnel, distributed job scheduler]
---

# What is Tikeo?

Tikeo is a distributed task scheduling and compute orchestration platform built in Rust. It combines a Server, a Web console, outbound-only Workers, workflow DAGs, multi-language SDKs, governed scripts, RBAC, audit logs, and deployment assets into one operator-friendly project.

## Why it exists

Traditional job schedulers often assume that the central server can call executor addresses directly. That breaks down when workers live behind NAT, in private Kubernetes namespaces, across VPCs, or in another cluster. Tikeo reverses that boundary: workers dial out to the Server over a gRPC/HTTP2 Worker Tunnel, and the Server reuses that connection for dispatch, cancellation, heartbeats, logs, and results.

## First evaluation path

1. Install the local toolchain.
2. Start the Server and Web console.
3. Run a Rust, Go, or Java Worker demo.
4. Create one job or workflow.
5. Inspect instances, logs, workers, audit evidence, and metrics.

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
(cd examples/rust/worker-demo && cargo run)
```

## What is implemented today

- Rust Server and Web UI.
- Worker Tunnel and persisted worker session visibility.
- Jobs, instances, schedules, attempts, logs, workflows, scripts, alerts, RBAC, OIDC, metrics, audit, and deployment assets.
- Rust, Go, Java, Python, and Node.js SDK/demo surfaces in CI, with language-specific docs starting from verified entry points.

## Next pages

- [Installation](./getting-started/installation)
- [Quickstart](./getting-started/quickstart)
- [Worker Tunnel](./concepts/worker-tunnel)
