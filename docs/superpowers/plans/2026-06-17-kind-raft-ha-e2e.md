# Kind Raft HA E2E Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a reproducible single-machine Kind validation that proves Tikeo 4-Pod Raft HA semantics, non-sticky API access, Worker Tunnel registration, dispatch, and leader-pod failover.

**Architecture:** Use a local Kind cluster with a PostgreSQL Deployment, a four-replica `tikeo-server` StatefulSet, headless peer DNS, API and Worker Tunnel ClusterIP Services, and a local Node.js worker connected through `kubectl port-forward` to the Worker Tunnel Service. The script bootstraps missing CLI tools into `.dev/tools/bin`, builds a fast local server image from `target/debug/tikeo`, runs rollout/fault checks, triggers jobs before and after leader deletion, and stores JSON/log evidence under `.dev/reports`.

**Tech Stack:** Bash, Docker, Kind, kubectl, Kubernetes StatefulSet/Service/ConfigMap/Secret, PostgreSQL 16, existing Tikeo smoke helpers, Node.js/Bun worker demo.

---

## File structure

- Create `scripts/kind-raft-ha-e2e.sh`: end-to-end Kind harness with tool bootstrap, manifest generation, deployment, port-forwarding, API/worker/job validation, fault injection, and evidence collection.
- Modify `docs/docs/deployment/server-ha.md`: add operator-facing Kind 4-Pod validation steps and expected evidence.
- Modify `docs/i18n/zh-CN/docusaurus-plugin-content-docs/current/deployment/server-ha.md`: Chinese mirror of Kind validation steps.
- Modify `deploy/k8s/README.md`: link from deployment manifests to the Kind validation harness.
- Modify `README.md` / `README.zh-CN.md`: concise validation command and evidence pointer in HA deployment section.

## Tasks

### Task 1: Build Kind E2E harness

- [ ] Create `scripts/kind-raft-ha-e2e.sh` with strict bash mode.
- [ ] Implement preflight and local CLI bootstrap for `kind`, `kubectl`, and optional `helm` into `.dev/tools/bin` when commands are absent.
- [ ] Build `target/debug/tikeo` with `cargo build --bin tikeo` unless disabled, then build/load a fast Debian-based local Docker image into Kind.
- [ ] Generate PostgreSQL and four-Pod Raft manifests with stable peer DNS and external DB URL.
- [ ] Create or reuse Kind cluster, apply manifests, wait for Postgres and StatefulSet readiness.
- [ ] Start API and Worker Tunnel `kubectl port-forward` processes and clean them up unless `TIKEO_KIND_E2E_KEEP=1`.
- [ ] Bootstrap admin via existing smoke helper, seed namespace/app/worker pool, start Node.js worker demo through Worker Tunnel service.
- [ ] Verify exactly one schedulable node and four diagnostics nodes using `scripts/verify-raft-ha-rollout.sh`.
- [ ] Trigger an API job through the API Service and assert `succeeded` with worker result.
- [ ] Run `scripts/raft-ha-fault-injection-drill.sh` in apply mode to delete the current leader pod, wait for recovery, verify rollout again, and trigger a second job.
- [ ] Collect `kubectl get`, pod YAML/logs, cluster diagnostics, metrics summary, rollout reports, worker log, instance result/logs, and a final summary JSON.

### Task 2: Document exact Kind validation steps

- [ ] Add a standalone Kind section to English and Chinese Server HA docs with prerequisites, one-command run, environment knobs, expected success criteria, and cleanup/keep behavior.
- [ ] Explain what Kind validates and what it does not validate versus a real multi-node production cluster.
- [ ] Add deployment README link and README quick command.

### Task 3: Verify and finish

- [ ] Run `bash -n scripts/kind-raft-ha-e2e.sh`.
- [ ] Run the full script or, if blocked by missing Docker/Kind/network resources, capture exact blocker evidence.
- [ ] If full run passes, commit with Lore commit protocol and push.
