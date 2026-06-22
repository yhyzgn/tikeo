# Kind Raft FSOD HA Validation Plan

> Run date target: 2026-06-22. Scope: single-machine, multi-node Kind validation that approximates production Kubernetes scheduling with required pod anti-affinity and topology spread.

## Goal

Validate that a four-pod Tikeo Server Raft FSOD cluster can keep scheduling and dispatching tasks when the old owner/gateway fails, while preserving epoch fencing and durable Worker Outbox reroute semantics. This plan intentionally uses **Kind worker nodes + required Pod Anti-Affinity** so that each Server pod is scheduled onto a different Kubernetes node, approximating production failure domains on one developer machine.

## Environment and topology

- Kubernetes runtime: Kind cluster `${TIKEO_KIND_CLUSTER_NAME:-tikeo-raft-ha}`.
- Node layout: one control-plane node plus `${TIKEO_KIND_WORKER_NODES:-4}` worker nodes.
- Server workload: `StatefulSet/tikeo-server`, default 4 replicas.
- Placement rule: required `podAntiAffinity` and `topologySpreadConstraints` over `kubernetes.io/hostname`.
- Storage: Postgres inside the Kind namespace for deterministic local evidence.
- Worker: Node.js demo worker outside the cluster, connected through the Worker Tunnel service/port-forward to one selected gateway pod.

## Checklist

| Status | ID | Test item | Scope | Expected output | Actual output | Script / evidence |
| --- | --- | --- | --- | --- | --- | --- |
| ⬜ | T00 | Preflight and tool install | docker, curl, jq, python3, bun, kind, kubectl | Missing kind/kubectl are installed under `.dev/tools/bin`; cluster API reachable | Filled by report generator | `scripts/kind-raft-ha-e2e.sh`, `kind-create-cluster.log`, `kubectl-cluster-info.txt` |
| ⬜ | T01 | Multi-node Kind topology | 1 control-plane + 4 worker nodes | `kindWorkerNodes >= serverReplicas`; nodes visible through kubectl | Filled by report generator | `kind-config.yaml`, `kind-nodes-initial.txt` |
| ⬜ | T02 | Server pod anti-affinity and topology spread | StatefulSet scheduling | 4 Server pods placed on 4 distinct Kind worker nodes; `antiAffinitySatisfied=true` | Filled by report generator | `server-pod-placement-initial-summary.json`, `server-pod-placement-after-gateway-poweroff-summary.json` |
| ⬜ | T03 | Raft bootstrap and schedulable owner | cluster diagnostics | exactly one or more observed schedulable owner views; leader/owner has fencing token; rollout gate passes | Filled by report generator | `cluster-diagnostics-initial.json`, `rollout-before-failover.json` |
| ⬜ | T04 | API pod and Worker gateway separation | Service/API vs Worker Tunnel | API requests enter one non-leader pod; Worker Tunnel is pinned to a different non-leader pod | Filled by report generator | `pod-selection.txt`, `workers-online.json` |
| ⬜ | T05 | Pre-failover dispatch | SDK/API key + Worker execution | API-triggered `demo.echo` job succeeds before chaos | Filled by report generator | `instance-result-before-failover.json`, `instance-logs-before-failover.json` |
| ⬜ | T06 | Old Owner Full GC / zombie recovery approximation | Epoch fencing under old-owner failure | Old owner is removed/recreated; new epoch/owner evidence rejects stale fencing; post-failover dispatch succeeds | Filled by report generator | `fault-drill.log`, `failover-summary.txt`, `db-evidence-before-failover.json`, `db-evidence-after-failover.json` |
| ⬜ | C01 | Chaos drill: old Owner zombie simulation | leader pod delete + rollout recovery | leader changes or term/epoch advances; stale owner cannot continue scheduling; rollout gate passes | Filled by report generator | `fault-drill/*`, `cluster-diagnostics-after-fault-drill.json` |
| ⬜ | T07 | Web/API load balancing round-robin/service rotation | in-cluster repeated ClusterIP calls | 48 service requests hit multiple Server pods; coverage/skew metrics recorded | Filled by report generator | `service-probe-initial-summary.json`, `service-probe-after-failover-summary.json`, `chart-service-lb.svg` |
| ⬜ | T08 | Gateway disconnect + Worker reconnect reroute | Outbox state machine | Slow `demo.sleep` dispatch survives old gateway force delete; Worker reconnects to a new gateway; row reroutes to newer generation | Filled by report generator | `gateway-poweroff-delete.log`, `gateway-reroute-summary.json`, `db-evidence-after-gateway-reroute.json` |
| ⬜ | C02 | Chaos drill: gateway node instant poweroff approximation | `kubectl delete pod --force --grace-period=0` | old gateway disappears without graceful drain; surviving Server performs global Outbox reroute | Filled by report generator | `gateway-poweroff-delete.log`, `gateway-reroute-pods.txt` |
| ⬜ | T09 | Post-chaos dispatch | final API dispatch | `demo.echo` job succeeds after leader and gateway chaos | Filled by report generator | `instance-result-after-failover.json`, `db-evidence-after-failover.json` |
| ⬜ | T10 | Evidence bundle and report | automated report generation | markdown report, JSON summary, CSV metrics, SVG charts generated under report directory | Filled by report generator | `kind-ha-validation-report.md`, `kind-ha-validation-summary.json`, `kind-ha-metrics.csv`, `chart-*.svg` |

## Acceptance criteria

- All checklist items T00-T10 and chaos drills C01-C02 are recorded as passed or have an explicit blocker with raw evidence.
- Server pod spread satisfies `uniqueNodes == serverReplicas == 4` at initial rollout and after gateway force-delete recovery.
- Service probe records at least 48 requests and reaches at least half of Server pods; ideally all 4 pods.
- Gateway reroute evidence shows `oldGateway != newGateway` and at least one Outbox row moved to the new gateway with `gatewayGeneration >= 2`.
- Epoch fencing evidence includes a targeted unit test for stale token rejection plus Kind failover DB/API evidence.
- Final report includes numeric indices: HA confidence, epoch fencing, Outbox reroute, service load-balancing, anti-affinity, and evidence completeness.

## Execution command

```bash
TIKEO_KIND_E2E_KEEP=0 \
TIKEO_KIND_E2E_REBUILD_SERVER=1 \
TIKEO_KIND_WORKER_NODES=4 \
scripts/kind-raft-ha-e2e.sh
```
