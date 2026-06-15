# Server Raft HA Rollout Checklist

**Date:** 2026-06-15  
**Scope:** Tikeo Server multi-pod Kubernetes HA, Raft active-passive scheduling ownership, Worker Tunnel leader behavior, and future shard ownership.  
**Rule:** Change a row status to `✅` only after the verification command or artifact is recorded in this file.

## Architecture decision

Tikeo Server HA should keep the original Raft/fencing design. Do not introduce Redis/Dragonfly distributed locks for core scheduler ownership. The production path is:

1. P0: Raft active-passive Server HA — multiple server pods form one Raft group; exactly one Leader owns schedule/dispatch/retry loops through `canSchedule=true` and a persisted `leaderFencingToken`.
2. P1: Worker Tunnel leader behavior — workers must have a safe way to be reachable by the dispatch owner during leader election/failover.
3. P2: Raft shard ownership only if measured scheduling bottlenecks require multi-active scheduling; shard ownership must still use Raft/fencing rather than external locks.

## P0 — Raft active-passive Server HA delivery

| ID | Item | Acceptance criteria | Status | Evidence |
|---|---|---|---|---|
| P0-1 | Rollout checklist | This file records P0/P1/P2, acceptance criteria, and evidence rule. | ✅ | Created in this change; local content check passed. |
| P0-2 | Helm values/schema/example | Chart exposes `server.cluster.mode=raft`, replicas, peer DNS, headless service, and transport token Secret values. | ✅ | `deploy/helm/tikeo/values.yaml`, `values.schema.json`, and `examples/values-raft-ha.yaml`; `python3 .github/tests/helm_raft_ha_contract_test.py` passed. |
| P0-3 | StatefulSet rendering | Raft mode renders a StatefulSet with stable pod names; standalone remains Deployment. | ✅ | Helm contract test passed; `helm template` standalone and raft overlays rendered successfully. |
| P0-4 | Headless peer service | Raft mode renders a headless Service used for peer endpoints. | ✅ | Contract test checks `tikeo-server-headless`, `clusterIP: None`, and `publishNotReadyAddresses: true`. |
| P0-5 | Stable Raft identity/config | Each pod starts with stable `cluster.node_id` and peer endpoints for all replicas. | ✅ | `TIKEO__CLUSTER__NODE_ID` uses `metadata.name`; ConfigMap renders peers `tikeo-server-0..2.tikeo-server-headless:9090`; contract test passed. |
| P0-6 | Transport token Secret | Raft internal HTTP transport token is sourced from Kubernetes Secret, not committed raw values. | ✅ | `TIKEO__CLUSTER__TRANSPORT_TOKEN` uses `secretKeyRef`; contract test passed. |
| P0-7 | Raw K8s/operator example | Non-Helm example exists for a 3-node Raft HA deployment. | ✅ | Added `deploy/k8s/tikeo-raft-ha.yaml`; contract test checks StatefulSet/headless/Secret refs. |
| P0-8 | Contract tests | Automated tests prove standalone and raft Helm render semantics. | ✅ | Added `.github/tests/helm_raft_ha_contract_test.py`; `3 tests OK`. |
| P0-9 | Docs updated | README, Helm README, and docs site explain active-passive Raft HA and no Redis lock. | ✅ | Updated README, Helm README, English/Chinese Kubernetes docs, and English/Chinese configuration references. |
| P0-10 | Verification run | Helm lint/template, deploy bootstrap, docs contracts, and targeted Raft tests complete. | ✅ | See verification log entries from 2026-06-15. |

## P1 — Worker Tunnel behavior under Server HA

| ID | Item | Acceptance criteria | Status | Evidence |
|---|---|---|---|---|
| P1-1 | Choose leader strategy | Document whether workers connect to leader, reconnect on leader change, or use follower proxy. | ✅ | Chosen strategy: Raft followers reject new Worker Tunnel registration with `FailedPrecondition`; workers reconnect through the Service until they reach the scheduling Leader. This keeps live worker streams local to the dispatch owner and avoids follower-held workers. |
| P1-2 | Implement strategy | Dispatch owner can reach eligible workers after leader election/failover. | ✅ | `WorkerTunnelRuntime` now carries cluster coordinator; `handle_register` rejects Raft non-owner nodes before registry mutation. Verified by `register_message_is_rejected_on_raft_follower`. |
| P1-3 | Verify failover | E2E covers leader failover + worker reconnect/dispatch with no duplicate/lost dispatch. | ✅ | `TIKEO_RAFT_WORKER_E2E_KEEP=0 scripts/raft-worker-failover-e2e.sh` passed; report `.dev/reports/raft-worker-failover-20260615t080318z-781199/raft-worker-failover-20260615t080318z-781199.json`. |

## P2 — Raft shard ownership evaluation

| ID | Item | Acceptance criteria | Status | Evidence |
|---|---|---|---|---|
| P2-1 | Shard design | Define shard key, assignment command, fencing token, rebalance/failover, and non-goals. | ✅ | Design constraints recorded below and executable decision model added in `cluster::shard_ownership`; implementation remains gated on measured bottleneck evidence. |
| P2-2 | Tests first | Add failing tests for shard ownership decisions before implementation. | ✅ | Added `cluster::shard_ownership` tests for stable shard mapping, balanced owner spread, owner-only fencing token, and stale epoch rejection; `cargo test -p tikeo-server cluster::shard_ownership --all-features` passed (4 tests). |
| P2-3 | Implement only if needed | Implement shard ownership only if P0/P1 single owner becomes a measured bottleneck. | ✅ | No measured bottleneck exists after P1 E2E; runtime multi-active scheduling remains intentionally disabled. Added pure Raft/fencing decision model only; no Redis/Dragonfly locks and no scheduler/dispatcher runtime wiring. |

## Verification log

- 2026-06-15: Created checklist and plan. P0-1 marked `✅` because the file exists and contains P0/P1/P2 with the evidence rule.

- 2026-06-15: P0 verification passed:
  - `python3 .github/tests/helm_raft_ha_contract_test.py` — 3 tests OK.
  - `.dev/tools/helm lint deploy/helm/tikeo` — 1 chart linted, 0 failed.
  - `.dev/tools/helm template tikeo deploy/helm/tikeo --namespace tikeo` — rendered standalone Deployment successfully.
  - `.dev/tools/helm template tikeo deploy/helm/tikeo --namespace tikeo -f deploy/helm/tikeo/examples/values-raft-ha.yaml` — rendered Raft StatefulSet successfully.
  - `scripts/verify-deploy-bootstrap.sh` — deployment bootstrap templates verified.
  - `python3 -m unittest discover -s .github/tests -p 'docs_site_contract_test.py' -k kubernetes_controller_runbook` — 1 test OK.
  - `cargo test -p tikeo-server raft --all-features` — 30 passed.

- 2026-06-15: P1 targeted verification passed:
  - `cargo test -p tikeo-server register_message_is_rejected_on_raft_follower --all-features` — follower registration rejection test OK.
  - `cargo test -p tikeo-server tunnel::service --all-features` — 8 passed.
  - `cargo test -p tikeo-server raft --all-features` — 31 passed.
  - `python3 .github/tests/helm_raft_ha_contract_test.py` — 3 tests OK.
  - `scripts/verify-deploy-bootstrap.sh` — deployment bootstrap templates verified.


- 2026-06-15: P1 real failover E2E passed:
  - `TIKEO_RAFT_WORKER_E2E_KEEP=0 scripts/raft-worker-failover-e2e.sh` — PASS.
  - Latest verification after SDK reconnect cleanup: `TIKEO_RAFT_WORKER_E2E_KEEP=0 TIKEO_RAFT_WORKER_E2E_REBUILD_SERVER=0 scripts/raft-worker-failover-e2e.sh` — PASS.
  - Report: `.dev/reports/raft-worker-failover-20260615t080318z-781199/raft-worker-failover-20260615t080318z-781199.json`.
  - Covered: 3 local Raft server processes sharing Docker Postgres, TCP LB for API/Tunnel, Node worker registration on initial leader, pre-failover job success, leader kill, new leader election, worker reconnect to new leader, post-failover job success.


- 2026-06-15: Node SDK HA reconnect cleanup verification passed:
  - `(cd sdks/nodejs/tikeo && bun test)` — 19 tests passed, including failed-registration stream/client close before retry.

- 2026-06-15: P2 shard ownership evaluation completed without enabling runtime multi-active scheduling:
  - `cargo test -p tikeo-server cluster::shard_ownership --all-features` — 4 tests passed.
  - Added pure decision model in `crates/tikeo-server/src/cluster/shard_ownership.rs` for deterministic shard keys, Raft-applied assignment epochs, owner fencing tokens, and stale-token rejection.
  - Gate decision: P0/P1 single-owner path passed real failover E2E, so no measured bottleneck justifies scheduler/dispatcher shard ownership runtime wiring yet.

## P1 chosen Worker Tunnel strategy

The first production-safe Server HA strategy is **leader-local Worker Tunnel registration**:

1. In `standalone`, workers register normally.
2. In `raft`, a node accepts new Worker Tunnel registration only when local cluster status has `canSchedule=true`.
3. Raft followers/candidates/unknown nodes reject registration with gRPC `FailedPrecondition` before mutating the in-memory `WorkerRegistry`.
4. Workers should use reconnect/backoff against the Worker Tunnel Service or Gateway endpoint. Once Kubernetes routes them to the current scheduling Leader, registration succeeds and the Leader owns both dispatch and the live outbound stream.
5. This deliberately avoids follower-held worker streams until a future proxy or shared tunnel registry is explicitly designed and tested.

P1 environment verification is complete: `scripts/raft-worker-failover-e2e.sh` starts a 3-node Raft cluster, connects a real Node SDK worker through a TCP Service/LB proxy, kills the leader, observes worker reconnect to the new leader, and verifies pre/post-failover job success.

## P2 shard ownership design constraints

Initial P2 design constraints, not yet implementation:

- Shard key should be deterministic from durable task scope, for example `hash(namespace/app/job_id) % shard_count`; workflow/internal queue items must use the parent workflow/job scope consistently.
- Shard ownership assignment must be a Raft-applied command with a term/epoch and per-shard fencing token.
- A server may run schedule/dispatch loops only for shards it owns and only while its shard fencing token is current.
- Rebalance must be explicit and observable; failed owners release through Raft re-assignment, not Redis/Dragonfly locks.
- P2 is not required unless P0/P1 single Leader scheduling is measured as a bottleneck.
