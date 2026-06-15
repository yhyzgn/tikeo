---
title: Server HA and cluster modes
description: How Tikeo Server high availability works, why Raft active-passive is the production default, and when shard ownership may be added later.
keywords: [tikeo server ha, raft, active passive, kubernetes ha, worker tunnel failover, distributed scheduler]
---

# Server HA and cluster modes

Tikeo Server HA is intentionally **Raft active-passive scheduling ownership** today. You can run multiple Server pods for control-plane availability, but task ownership is not split across pods unless a future Raft shard-ownership feature is explicitly enabled and verified.

This page is the source of truth for the trade-off. Read it before increasing `server.replicas` or adding an external lock service.

## Deployment architecture

```mermaid
flowchart LR
  subgraph WorkerNetworks[Worker networks: app namespaces, VPCs, VMs, edge]
    W1[Worker SDK process]
    W2[Worker SDK process]
  end

  subgraph K8s[Kubernetes namespace: tikeo]
    WT[Worker Tunnel Service or Gateway
gRPC HTTP2]
    API[HTTP API Service
REST and SSE]
    HS[tikeo-server-headless
Raft peer DNS]

    subgraph SS[StatefulSet tikeo-server]
      S0[tikeo-server-0
Raft node]
      S1[tikeo-server-1
Raft node]
      S2[tikeo-server-2
Raft node]
    end

    Web[Tikeo Web]
  end

  DB[(External DB
PostgreSQL MySQL CockroachDB)]
  Secret[Raft transport token Secret]
  Human[Operators and SDK management clients]

  W1 -->|outbound OpenTunnel| WT
  W2 -->|outbound OpenTunnel| WT
  WT --> S0
  WT --> S1
  WT --> S2
  Human --> API
  Human --> Web
  API --> S0
  API --> S1
  API --> S2
  S0 <-->|Raft append entries| HS
  S1 <-->|Raft append entries| HS
  S2 <-->|Raft append entries| HS
  S0 --> DB
  S1 --> DB
  S2 --> DB
  Secret --> S0
  Secret --> S1
  Secret --> S2
```

The important detail in this diagram is not the number of pods. The important detail is ownership: all Server pods are live control-plane members, but exactly one Raft Leader owns scheduling at a time.

```mermaid
stateDiagram-v2
  [*] --> Standalone: cluster.mode=standalone
  Standalone --> SingleOwner: one process owns scheduling
  [*] --> RaftMode: server.cluster.mode=raft
  RaftMode --> Election: StatefulSet peers start
  Election --> LeaderActive: leader elected and fencing token persisted
  LeaderActive --> FollowerRejected: worker hits follower
  FollowerRejected --> Reconnect: FailedPrecondition
  Reconnect --> LeaderActive: SDK reconnects to leader
  LeaderActive --> Election: leader dies
```

## Cluster mode decision

```mermaid
flowchart TD
  A[Need more than one Server pod?] -->|No| B[Use standalone]
  A -->|Yes| C[Use Raft active-passive]
  C --> D[External DB plus StatefulSet plus headless peer Service]
  D --> E[One Leader schedules; followers serve API health Raft transport]
  E --> F{Measured scheduler bottleneck?}
  F -->|No| G[Keep active-passive]
  F -->|Yes| H[Design Raft shard ownership with epoch fencing]
  H --> I[Do not use Redis or Dragonfly locks for core ownership]
```

## What is implemented now

| Capability | Current behavior |
| --- | --- |
| Multi-pod Server deployment | Helm `server.cluster.mode=raft` renders a `StatefulSet` with stable pod names and a headless peer Service. |
| Consensus and ownership | Server pods form a Raft group. Only the elected Leader with a persisted fencing token reports `canSchedule=true`. |
| Scheduling loops | Only the scheduling Leader runs schedule, dispatch, retry, notification-delivery ownership, and other ownership-sensitive loops. Followers skip those loops. |
| Worker Tunnel registration | In Raft mode, followers reject new Worker Tunnel registrations with `FailedPrecondition`. Workers reconnect through the Service/LB until routed to the Leader. |
| Worker execution | Workers still run outside the Server chart and connect outbound to the Worker Tunnel. No inbound Worker Service is required. |
| External locks | Redis/Dragonfly/SQL advisory locks are not used for core scheduler ownership. |
| Multi-active scheduling | Not enabled today. A pure Raft/fencing shard-decision model exists for future gated work, but runtime shard scheduling is intentionally off until a measured bottleneck justifies it. |

## Why active-passive first

Tikeo has two independent HA questions:

1. **Control-plane availability**: can the API/cluster recover when one Server pod dies?
2. **Scheduling parallelism**: can multiple Server pods safely claim different work at the same time?

The current feature solves the first question and keeps the second deliberately conservative. This avoids the most dangerous scheduler failure modes: duplicate dispatch, split-brain ownership, stale leases, and “two pods think they own the same queue” bugs.

## Advantages

| Advantage | Why it matters |
| --- | --- |
| Strong ownership semantics | Raft term + persisted fencing token gives observable ownership evidence instead of an implicit best-effort DB lock. |
| Safer failover | A dead Leader stops owning dispatch; a new Leader must be elected and fenced before scheduling resumes. |
| Simpler operations | Operators deploy one `StatefulSet`, a headless Service, an external DB, and a transport-token Secret. No Redis/Dragonfly cluster is required just to coordinate ownership. |
| Fewer duplicate-dispatch risks | Only one Server owns schedule/dispatch loops, so queue claims do not rely on several active schedulers racing correctly. |
| Clear Worker Tunnel model | The Leader owns both dispatch and the live worker stream. Followers do not hold workers that the dispatch owner cannot reach. |
| Upgrade path remains open | If throughput later requires multi-active scheduling, shard ownership can be added inside the same Raft/fencing model. |

## Limitations and trade-offs

| Limitation | Operational meaning | Mitigation |
| --- | --- | --- |
| One active scheduler | Additional Server pods improve HA but do not linearly increase scheduling throughput. | Measure queue pressure first. Add Raft shard ownership only when the single Leader is the bottleneck. |
| Followers are not dispatch workers | Followers serve health/API/Raft transport but do not schedule or own Worker Tunnel registrations. | Route Worker Tunnel through a Service/LB and rely on SDK reconnect/backoff. |
| Failover is not instantaneous | During election, scheduling pauses until a new Leader has a persisted fencing token. | Use normal retry policies and monitor cluster status/queue age. |
| Requires stable identities | Raft pods need stable names and peer DNS; a plain `Deployment` is not enough for production multi-pod Server HA. | Use the Helm Raft overlay or raw `deploy/k8s/tikeo-raft-ha.yaml`. |
| Requires external DB | Multi-pod HA cannot rely on a single pod-local SQLite file. | Use PostgreSQL, MySQL, or CockroachDB-compatible external storage. |
| LB behavior matters for Worker Tunnel | A worker may initially hit a follower and receive `FailedPrecondition`. | SDKs should reconnect with backoff; expose Worker Tunnel through a gRPC/HTTP2-capable Service/Gateway. |

## Deployment modes

| Mode | How to run it | Use when | Do not use when |
| --- | --- | --- | --- |
| Standalone | `cluster.mode=standalone`, one Server process/pod | Local dev, demos, small single-node VM installs | You need Server pod failover |
| Raft active-passive | `server.cluster.mode=raft`, `StatefulSet`, external DB, transport token | Production Kubernetes HA and safe Server failover | You expect every pod to schedule a share of jobs |
| Future Raft shard ownership | Not currently runtime-enabled | A measured bottleneck proves the Leader cannot schedule/dispatch fast enough | You only want generic “more pods” without proving throughput pressure |
| Redis/Dragonfly lock based scheduling | Not a Tikeo core scheduler mode | N/A | Core scheduling ownership; it breaks the Raft/fencing design goal |

## Prerequisites

Before enabling Raft Server HA, prepare these production dependencies instead of only changing the replica count:

- **External database**: PostgreSQL, MySQL, or CockroachDB-compatible storage reachable by every Server pod through the same schema. Do not use pod-local SQLite for multi-pod HA.
- **Stable Server identities**: run Server as a `StatefulSet` with stable pod names and a headless peer Service. A plain Kubernetes `Deployment` does not provide the peer identity model Raft needs.
- **Raft transport Secret**: create a high-entropy `tikeo-raft-transport` Secret and mount it as `TIKEO__CLUSTER__TRANSPORT_TOKEN` for every Server pod.
- **Worker Tunnel networking**: expose Worker Tunnel through a gRPC/HTTP2-capable Service, Gateway, or ingress path. The API path may carry REST and SSE, but the Worker Tunnel stream must not be downgraded to HTTP/1.1.
- **Network-layer behavior**: configure nginx, LB, WAF, and Kubernetes ingress/gateway controllers so they do not buffer, rewrite, or prematurely close long-lived SSE and gRPC streams. Use the dedicated [SSE realtime deployment notes](./sse-realtime) for the REST/SSE API path and the Kubernetes controller runbook for ingress-specific annotations.
- **Worker reconnect policy**: use an SDK version that reconnects after `FailedPrecondition`, stream drops, or Leader failover. Followers intentionally reject Worker Tunnel registration in Raft mode.
- **Operational smoke test**: keep at least one real Worker available during rollout so failover can be tested end to end, not only by checking pod readiness.

## Verify

Use both rendered-manifest checks and runtime checks. A green rollout alone only proves pods started; it does not prove scheduling ownership is safe.

Render the Helm output first:

```bash
helm template tikeo ./deploy/helm/tikeo \
  --namespace tikeo \
  -f deploy/helm/tikeo/examples/values-external-postgres.yaml \
  -f deploy/helm/tikeo/examples/values-raft-ha.yaml \
  | grep -E 'kind: StatefulSet|tikeo-server-headless|TIKEO__CLUSTER__MODE|TIKEO__CLUSTER__TRANSPORT_TOKEN'
```

After install or upgrade:

```bash
kubectl -n tikeo rollout status statefulset/tikeo-server
kubectl -n tikeo get pods -l app.kubernetes.io/component=server -o wide
kubectl -n tikeo get svc tikeo-server-headless
```

Then verify cluster ownership from the management/API endpoint. Exactly one Server should report `canSchedule=true`; followers should be present but not scheduling:

```bash
curl -fsS "$TIKEO_SERVER_URL/api/v1/cluster" \
  -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY" \
  | jq '.data.nodes[] | {nodeId, role, canSchedule, raftTerm}'
```

Finally, verify Worker Tunnel behavior with a real Worker. In local/e2e environments, the failover smoke can be reused without rebuilding already-built binaries:

```bash
TIKEO_RAFT_WORKER_E2E_KEEP=0 \
TIKEO_RAFT_WORKER_E2E_REBUILD_SERVER=0 \
scripts/raft-worker-failover-e2e.sh
```

The expected result is: worker registers, a pre-failover job succeeds, the Leader is killed, a new Leader is elected, the worker reconnects, and a post-failover job succeeds.

## Kubernetes install summary

Use the committed overlay instead of only raising `server.replicas`:

```bash
kubectl -n tikeo create secret generic tikeo-raft-transport \
  --from-literal=transport-token="$(openssl rand -hex 32)"

helm upgrade --install tikeo ./deploy/helm/tikeo \
  --namespace tikeo \
  --create-namespace \
  -f deploy/helm/tikeo/examples/values-external-postgres.yaml \
  -f deploy/helm/tikeo/examples/values-raft-ha.yaml

kubectl -n tikeo rollout status statefulset/tikeo-server
```

Expected rendered shape:

- `StatefulSet/tikeo-server`, not `Deployment/tikeo-server`.
- `Service/tikeo-server-headless` with stable peer DNS.
- `TIKEO__CLUSTER__MODE=raft`.
- `TIKEO__CLUSTER__NODE_ID` from the pod name.
- `TIKEO__CLUSTER__TRANSPORT_TOKEN` from a Secret.
- external DB Secret for all pods.

## Worker Tunnel failover behavior

In Raft mode:

```mermaid
sequenceDiagram
  participant W as Worker SDK
  participant LB as Worker Tunnel Service or Gateway
  participant F as Follower Server
  participant L as Leader Server
  participant DB as External DB

  W->>LB: OpenTunnel RegisterWorker
  LB->>F: routed to follower
  F-->>W: FailedPrecondition
  W->>LB: reconnect with backoff
  LB->>L: routed to leader
  L->>DB: persist worker session and fencing state
  L-->>W: WorkerRegistered
  L->>W: DispatchTask over live stream
  W-->>L: TaskLog and TaskResult
  L-xL: leader dies
  W-->>LB: stream drops and reconnects
  LB->>L: new leader after election
  L-->>W: WorkerRegistered
```

1. Worker opens an outbound gRPC/HTTP2 stream to the Worker Tunnel endpoint.
2. If the stream reaches a follower, the follower rejects registration with `FailedPrecondition` before mutating its in-memory registry.
3. The Worker SDK reconnects with backoff through the Service/LB.
4. When the stream reaches the current Leader, registration succeeds.
5. The Leader dispatches tasks over its local live stream.
6. If the Leader dies, the stream drops; the worker reconnects until it reaches the new Leader.

This behavior was verified with a multi-process failover E2E: three Raft Server processes, shared PostgreSQL, TCP Service/LB proxy, Node worker registration, pre-failover job success, Leader kill, new Leader election, worker reconnect, and post-failover job success.

## When to consider shard ownership later

Do not add shard scheduling just because multiple pods exist. Add it only when production evidence shows the single scheduling Leader is the bottleneck, for example:

- queue age grows while workers are available;
- dispatch loop CPU or DB claim latency is saturated on the Leader;
- API/Web/Worker Tunnel capacity is fine, but scheduling claim throughput is not.

A future shard implementation must remain Raft/fencing based:

- deterministic shard key such as `hash(namespace/app/job_or_workflow_id) % shard_count`;
- Raft-applied assignment command with monotonic epoch;
- per-shard fencing token;
- stale-token rejection after failover/rebalance;
- observable rebalance events and rollback path.

## Troubleshooting

| Symptom | Likely cause | What to check |
| --- | --- | --- |
| More than one pod reports `canSchedule=true` | Broken fencing or mixed configuration | Stop the rollout, inspect `TIKEO__CLUSTER__MODE`, node IDs, Raft term, DB fencing rows, and make sure all pods share the same external DB. |
| No pod reports `canSchedule=true` | Raft cannot elect or persist ownership | Check headless DNS, peer addresses, transport token, external DB connectivity, and pod logs for election or persistence errors. |
| Workers keep reconnecting and never register | Worker Tunnel is routed only to followers or gRPC is broken by the network layer | Check Service/Gateway endpoints, HTTP/2 support, ingress annotations, LB health checks, and follower `FailedPrecondition` logs. |
| Jobs remain queued after failover | New Leader is not fenced or workers did not reconnect | Query `/api/v1/cluster`, inspect queue age, confirm worker sessions on the Leader, and run one management trigger smoke. |
| Works with one pod but fails with three | Local SQLite, plain Deployment, or missing headless peer Service | Confirm external DB, `StatefulSet/tikeo-server`, stable pod names, and `tikeo-server-headless`. |
| API SSE dashboards disconnect repeatedly | Proxy buffering, WAF idle timeout, or HTTP/1.1 downgrade | Apply the SSE realtime network settings and separate API/SSE behavior from Worker Tunnel gRPC checks. |

## Production checklist

- [ ] Use `standalone` for one Server only.
- [ ] Use `raft` + StatefulSet + external DB for production multi-pod Server HA.
- [ ] Do not use Redis/Dragonfly locks for core scheduler ownership.
- [ ] Confirm exactly one node reports `canSchedule=true`.
- [ ] Verify at least one real Worker connects through Worker Tunnel after failover.
- [ ] Monitor queue age before considering shard ownership.
