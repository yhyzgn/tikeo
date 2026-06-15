# Server Raft HA Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver Tikeo Server Kubernetes multi-pod HA as Raft active-passive ownership first, then close Worker Tunnel leader routing, then evaluate Raft shard ownership without introducing Redis/Dragonfly distributed locks.

**Architecture:** P0 makes the deployable Server cluster a StatefulSet-backed Raft group with stable pod identity, headless peer DNS, external DB, internal transport token, and verification that only one pod can report `canSchedule=true`. P1 makes Worker Tunnel behavior explicit under leader failover so task dispatch has a safe owner. P2 keeps multi-active scaling in the Raft/fencing model by planning shard ownership instead of external locks.

**Tech Stack:** Helm v3, Kubernetes StatefulSet/headless Service/Secret, Tikeo Rust server Raft coordinator, Docusaurus docs, Python contract tests, shell deployment smoke scripts.

---

## Status checklist

### P0 — Raft active-passive Server HA delivery

- [x] Document the current gap and target semantics in `design/reports/server-raft-ha-rollout-checklist.md`.
- [x] Add Helm values/schema/examples for `server.cluster.mode=raft`, stable replicas, headless service, peer generation, and transport token Secret.
- [x] Render Server as StatefulSet when Raft is enabled; keep Deployment for standalone/dev.
- [x] Generate per-pod config with stable `cluster.node_id` from pod ordinal and peer endpoints from headless Service DNS.
- [x] Add/update raw K8s manifests or examples for a 3-node Raft install with external DB and transport token.
- [x] Add contract tests proving Helm renders StatefulSet/headless service/raft env/config and standalone still renders Deployment.
- [x] Update README/docs/Helm README to say production multi-pod server HA uses Raft StatefulSet and only one scheduling leader.
- [x] Run verification: Helm lint/template, deploy bootstrap script, docs contract tests, and targeted Rust Raft tests.

### P1 — Worker Tunnel behavior under Server HA

- [x] Define the chosen Worker Tunnel leader strategy in the checklist after P0 evidence is available.
- [x] Implement worker connection behavior so dispatch owner can reach eligible workers after leader election/failover.
- [x] Add e2e/contract verification for leader failover plus worker reconnect/dispatch.

### P2 — Raft shard ownership evaluation

- [x] Write the shard ownership design: shard key, assignment command, fencing token, failover, and non-goals.
- [x] Add tests for shard ownership decisions before any implementation.
- [x] Implement only if scheduler/dispatcher owner is a measured bottleneck. No runtime multi-active implementation was enabled because P1 E2E passed and no bottleneck was measured; pure Raft/fencing decision model added for future gated work.

## File map

- `deploy/helm/tikeo/values.yaml` — default values; add cluster/raft settings without changing standalone default.
- `deploy/helm/tikeo/values.schema.json` — validate new cluster settings.
- `deploy/helm/tikeo/templates/_helpers.tpl` — helper names for server/headless services and generated peer endpoints.
- `deploy/helm/tikeo/templates/configmap.yaml` — generated Tikeo config including `[cluster]` and peers.
- `deploy/helm/tikeo/templates/server.yaml` — switch between Deployment and StatefulSet, add headless Service and env/Secret references.
- `deploy/helm/tikeo/examples/values-raft-ha.yaml` — production-shaped Raft HA overlay.
- `deploy/k8s/tikeo-raft-ha.yaml` — raw manifest example for operators not using Helm.
- `scripts/verify-deploy-bootstrap.sh` — assert new templates/examples are present.
- `.github/tests/helm_raft_ha_contract_test.py` — render chart and check key K8s semantics.
- `deploy/helm/tikeo/README.md`, `docs/docs/deployment/kubernetes.md`, `docs/i18n/zh-CN/.../deployment/kubernetes.md`, `docs/docs/reference/configuration.md`, `docs/i18n/zh-CN/.../reference/configuration.md`, `README.md` — operator docs.
- `design/reports/server-raft-ha-rollout-checklist.md` — source-of-truth progress checklist updated from `[ ]` to `✅` only after verification.

## Task 1: Create source-of-truth rollout checklist

- [ ] Create `design/reports/server-raft-ha-rollout-checklist.md` with P0/P1/P2 items, acceptance criteria, and a rule that status changes to `✅` only after command evidence is appended.
- [ ] Verify the file contains all three phases and current P0 items.

## Task 2: Add Helm Raft HA values and tests

- [ ] Add a failing contract test `.github/tests/helm_raft_ha_contract_test.py` that runs `.dev/tools/helm template` twice:
  - default values must render `kind: Deployment` for `tikeo-server` and no `tikeo-server-headless` service.
  - `deploy/helm/tikeo/examples/values-raft-ha.yaml` must render `kind: StatefulSet`, `serviceName: tikeo-server-headless`, a headless Service with `clusterIP: None`, `replicas: 3`, `TIKEO__CLUSTER__NODE_ID` from pod metadata or ordinal wrapper, `TIKEO__CLUSTER__MODE=raft`, and `TIKEO__CLUSTER__TRANSPORT_TOKEN` from a Secret.
- [ ] Run the test and confirm it fails before implementation.
- [ ] Add values/schema/example fields for `server.cluster.mode`, `server.cluster.transportTokenExistingSecret`, `server.cluster.transportTokenSecretKey`, `server.cluster.peerServiceName`, and safe raft defaults.
- [ ] Implement Helm template changes.
- [ ] Run the contract test and confirm it passes.

## Task 3: Update deployment verification and docs

- [ ] Update `scripts/verify-deploy-bootstrap.sh` to require the Raft HA example and documentation tokens.
- [ ] Update Helm README, Kubernetes docs in English and Chinese, and configuration reference docs to remove stale contradiction and describe Raft as the production multi-pod Server HA path with active-passive scheduling owner.
- [ ] Update README Kubernetes/cluster wording if it currently overclaims or omits the required Raft StatefulSet path.
- [ ] Run docs/deploy contract tests.

## Task 4: Run P0 verification and mark completed items

- [ ] Run `.dev/tools/helm lint deploy/helm/tikeo`.
- [ ] Run `.dev/tools/helm template tikeo deploy/helm/tikeo --namespace tikeo`.
- [ ] Run `.dev/tools/helm template tikeo deploy/helm/tikeo --namespace tikeo -f deploy/helm/tikeo/examples/values-raft-ha.yaml`.
- [ ] Run `python3 -m unittest .github/tests/helm_raft_ha_contract_test.py`.
- [ ] Run `scripts/verify-deploy-bootstrap.sh`.
- [ ] Run targeted Raft server tests or record why environment blocks them.
- [ ] Append evidence to `design/reports/server-raft-ha-rollout-checklist.md` and change only verified P0 rows to `✅`.

## Task 5: P1/P2 follow-up handoff

- [ ] Add concrete P1 implementation notes for Worker Tunnel leader strategy based on the P0 rendered deployment.
- [x] Add P2 shard ownership design constraints and explicit non-goal: no Redis/Dragonfly lock for core scheduler ownership.
- [ ] Leave unchecked items for work not implemented in this session.
