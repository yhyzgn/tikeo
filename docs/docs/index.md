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
3. Run a Rust, Go, Java, Python, or Node.js Worker demo.
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

## Product strengths to evaluate

Tikeo is strongest when the environment is more complex than a single trusted executor process. Evaluate it for cross-network workers, audited execution history, governed scripts, multi-language worker teams, workflow topology, and deployment paths that need Kubernetes or VM/systemd parity.

## Comparison framing

The right comparison is not only feature count. Ask whether the scheduler can operate when workers cannot receive inbound traffic, whether stale workers are fenced, whether script execution is governed outside the Server process, whether Web and API evidence agree, and whether deployment assets are production-shaped rather than screenshots.

## Documentation scope

This first docs site focuses on accurate evaluation. It does not claim public hosted deployment, search indexing, full API generation, or complete translated content beyond the pages present in the repository. Later phases should generate API and configuration references from source artifacts.

## Maintainer promise

If a page gives a command, the repository should contain the corresponding code, test, demo, or verification path. If a feature is planned, the page should call it planned instead of presenting it as complete.
