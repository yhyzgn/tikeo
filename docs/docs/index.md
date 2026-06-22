---
title: What is Tikeo?
slug: /
description: Tikeo is a Rust-native platform for scheduled jobs, workflow DAGs, outbound Worker Tunnel execution, multi-language SDKs, governed scripts, RBAC, audit, and deployment automation.
keywords: [rust scheduler, workflow orchestration, worker tunnel, distributed job scheduler, tikeo]
---

# What is Tikeo?

Tikeo is a Rust-native orchestration control plane for teams that need more than a timer. It combines scheduled jobs, API-triggered jobs, workflow DAGs, outbound-only Workers, SDK processors, governed scripts, Notification Center delivery, alerting boundaries, RBAC, audit evidence, Web operations, Docker/Helm/Terraform deployment assets, and operator-verified SDK examples into one project.

The README is intentionally short: it explains why the project exists and how to evaluate it at a glance. This documentation site is the operating manual. A reader outcome for this site is concrete: after following the relevant pages, you should be able to install the toolchain, start the Server, bootstrap the first Owner, create namespace/app scope, create an app-scoped SDK API key, connect a Worker through the Worker Tunnel, create and trigger a job from an SDK, inspect instances/logs/audit evidence, deploy the Server/Web pair with Compose or Helm, and know which defaults changed when you moved from local SQLite to production PostgreSQL/MySQL.

## Documentation map

Read these pages in order when you are new to the repository:

| Stage | Page | What it gives you |
| --- | --- | --- |
| 1 | [Installation](./getting-started/installation) | Toolchain matrix, version baselines, repository surfaces, build/test commands, first-time bootstrap prerequisites. |
| 2 | [Quickstart](./getting-started/quickstart) | A local Server + Web + Worker + SDK Management API path with explicit acceptance evidence. |
| 3 | [Configuration reference](./reference/configuration) | Complete default-value table, env override names, examples, TLS/mTLS, OIDC, logging, OTel, cluster caveats, Worker SDK defaults. |
| 4 | [Worker Tunnel](./concepts/worker-tunnel) | Why Workers dial out, what registration carries, and what must never become a business Worker inbound Service. |
| 5 | SDK pages | Dependency coordinates, WorkerConfig defaults, minimal Worker examples, Management client credentials, live verification runbooks. |
| 6 | Deployment pages | Single binary, Docker Compose, Kubernetes/Helm, [Server HA and Raft FSOD Cluster](./deployment/server-ha), controller-specific ingress guidance, and smoke scripts. |
| 7 | Reference pages | Implementation-derived Management OpenAPI, Notification Center, and Worker Tunnel protobuf reference. |
| 8 | [Product readiness acceptance checklist](./development/product-readiness-acceptance) | Cross-feature release/handoff gates for Notification Center, legacy migration CLI, and Raft FSOD Server HA. |
| 9 | [v0.3.9 release acceptance packet](./development/release-acceptance-packet-v0.3.9) | Concrete release/handoff evidence: assets, CI, Kind HA metrics, cross-language Worker soak, and remaining production checks. |

If you only want a proof that the whole local path still works, run the Management trigger smoke from the quickstart. If you are writing a production runbook, use the configuration and deployment references first, then select one SDK page for the Worker language used by your service team.

## Architecture boundary

Tikeo has one central boundary that all docs must preserve: the Server schedules, governs, persists, audits, and dispatches; Workers execute. The Server does not run user business code. Business Workers do not need inbound ports. A Worker registers outbound to the gRPC/HTTP2 Worker Tunnel, sends a structured capability snapshot, receives the authoritative `worker_id`, heartbeats with lease/fencing metadata, receives `DispatchTask`, emits `TaskLog`, and returns `TaskResult` with the assignment token supplied by the Server.

This boundary matters because it changes how you deploy and troubleshoot the system. If a Worker runs in a private namespace, customer VPC, NAT, another cloud, or a VM behind a firewall, you still only need it to reach the Worker Tunnel endpoint. You should never expose a random business Worker HTTP server just so the scheduler can call it. The Helm chart therefore installs Server and Web only; business Workers are separate Deployments, DaemonSets, sidecars, VM services, or embedded SDK clients that dial out.

## Product surfaces implemented in this repository

The repository contains production-shaped surfaces, not just screenshots:

- Rust workspace crates for configuration, storage, server, protocol, and WASM boundaries.
- A single `tikeo` binary with `serve --config <path>` and `TIKEO_CONFIG` support.
- Config examples for local SQLite, container SQLite, PostgreSQL/CockroachDB, MySQL, and raft-shape metadata.
- Web console in `web/`, built with React, TypeScript, Vite, Ant Design, and Bun. Current operations surfaces include Notification Center channels/policies/messages/delivery queue and alert-event review.
- Docs site in `docs/`, built as a standalone Docusaurus module and Docker image.
- Worker SDKs for Rust, Go, Java/Spring Boot, Python, and Node.js.
- Runnable Worker demos under `examples/<language>/worker-demo` with structured processor capabilities.
- Management SDK helper surfaces for creating API jobs and triggering them with app-scoped `x-tikeo-api-key` credentials.
- Deployment assets for Docker Compose, Helm, Kubernetes YAML, systemd, Terraform, GitOps manifest diff, and smoke scripts.
- Contract tests that keep docs, workflows, source size, runtime versions, deployment artifacts, Notification Center indexes, and management trigger flows from drifting.

When a page states a command, it should point to one of those surfaces. When a feature is not yet ready for production use, the page should say so explicitly; for example, experimental surfaces must be labeled as such and must not be described as production runbooks.

## Reader outcome

This site is written for four operators:

1. **Platform evaluator**: needs to decide if Tikeo can replace a legacy scheduler where Workers cannot expose inbound ports.
2. **Application engineer**: needs to add an SDK dependency, declare processors, connect to the Worker Tunnel, and trigger jobs from app-scoped credentials.
3. **SRE/platform operator**: needs to deploy Server/Web, configure storage, TLS/mTLS, logging, OTel, ingress/gateway, backups, rollbacks, and smoke checks.
4. **Maintainer**: needs to run tests, keep docs operator-verified, avoid invented endpoints, and update the related operator documentation after work.

The docs therefore prefer tables, defaults, copy-paste commands, expected observations, and failure triage over marketing paragraphs.

## Evidence-first evaluation

A valid local evaluation should produce evidence from every layer:

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
cd web && bun install --frozen-lockfile && bun run typecheck && bun run build
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```

The smoke is stronger than a screenshot because it starts an isolated Server, writes an isolated SQLite config and DB under `.dev/reports/management-trigger-e2e-*`, bootstraps scope, creates a service account and SDK API key, starts the Node.js Worker demo with `TIKEO_WORKER_CONNECT=1`, creates and triggers a job through the SDK Management client, then records instance result/log evidence.

## Implementation anchors

This site should not invent API names, package coordinates, config keys, or deployment flags. The main sources are:

- `crates/tikeo-config/src/lib.rs` for server config shape, defaults, and `TIKEO__...` environment override behavior.
- `crates/tikeo-server/src/http/router.rs`, `openapi.rs`, and route files for Management API routes.
- `crates/tikeo-proto/proto/worker.proto` for Worker Tunnel protocol.
- `sdks/*` and `examples/*` for Worker and Management client examples.
- `deploy/*`, `docker-compose*.yml`, `Dockerfile`, `web/Dockerfile`, and `docs/Dockerfile` for deployment surfaces.
- `.github/tests/*` and smoke scripts for verification.

Notification Center and Alerts pages are additionally backed by `design/notification-center-alerting-plan.md`, `crates/tikeo-server/src/notification.rs`, `crates/tikeo-server/src/http/routes/notifications.rs`, `crates/tikeo-storage/src/repository/notification.rs`, `crates/tikeo-config/src/lib.rs`, and `web/src/pages/NotificationCenterPage.tsx`. If those sources disagree with the docs, fix the docs or code and add a test. Do not paper over drift with vague wording.

## What to do next

- New local evaluator: start with [Installation](./getting-started/installation), then [Quickstart](./getting-started/quickstart).
- SDK adopter: read [Configuration reference](./reference/configuration) first, then the language SDK page.
- Kubernetes operator: read [Kubernetes and Helm](./deployment/kubernetes), [Server HA and Raft FSOD Cluster](./deployment/server-ha), and [Kubernetes controller runbook](./deployment/kubernetes-controller-runbook).
- Notification operator: read [Notifications](./user-guide/notifications), [Alerts](./user-guide/alerts), and [Notification Center reference](./reference/notification-center) to keep outbound delivery separate from incident semantics.
- Troubleshooter: use [Troubleshooting](./reference/troubleshooting), the smoke report directory, and the operator-verified references.

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
