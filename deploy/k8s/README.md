# Kubernetes deployment ☸️

[🇨🇳 中文部署文档](../../README.zh-CN.md#运行-tikeo-服务)

Tikeo runs the Server and Web console as separate workloads. Business Workers should run in their
own namespaces, clusters, VM groups, or embedded application processes, then initiate outbound gRPC
connections to the Worker Tunnel endpoint. Do not expose inbound Worker executor ports unless your
own application explicitly needs them.

## Deployment choices

| Path | Use it when |
| --- | --- |
| `deploy/helm/tikeo/` | Preferred production Kubernetes path. It supports standalone mode, Raft HA mode, Ingress/Gateway examples, TLS/mTLS, external DB Secrets, and release image pinning. |
| `deploy/k8s/tikeo-raft-ha.yaml` | Raw manifest reference for Raft HA without Helm. Use it as a starting point, then replace image tags, database Secret names, resources, storage, and ingress for your cluster. |
| `deploy/k8s/operator/` | CRD/GitOps drift-review workflow. It reconciles `TikeoManifest` documents through management APIs instead of bypassing RBAC/audit with direct DB writes. |

## Server HA quick reference

Multi-Pod Server HA is not a plain `replicas: N` bump. Use the Raft overlay with stable pod identity,
a headless peer Service, a shared external database, and a Kubernetes Secret for
`cluster.transport_token`. The full public runbook is
[Server HA and cluster modes](https://docs.tikeo.net/docs/deployment/server-ha).

Current FSOD semantics:

- Raft elects one fenced Leader for workflow materialization, broadcast dispatch, retry, and default
  queue ownership loops.
- Dispatch intent is persisted to `worker_dispatch_outbox` before stream delivery, so gateway or
  Worker disconnects can reroute/requeue from durable state.
- Workers may connect to any Server Pod. Sessions record `gateway_node_id`; the Leader can deliver
  locally or send an internal relay hint to the gateway that owns the stream.
- Scheduler shards are projected into `cluster_shard_ownership` under the current Leader fencing
  token. Follower shard-owner claiming is implemented for the future multi-owner path, but automatic
  balancing remains gated behind Raft-applied membership/health rebalancing.
- Redis/Dragonfly distributed locks are intentionally not part of core scheduling correctness.

Minimal raw-manifest flow:

```bash
kubectl create namespace tikeo
kubectl -n tikeo create secret generic tikeo-database \
  --from-literal=database-url='postgres://tikeo:change-me@postgres:5432/tikeo'
kubectl -n tikeo create secret generic tikeo-raft-transport \
  --from-literal=transport-token="$(openssl rand -hex 32)"
kubectl apply -f deploy/k8s/tikeo-raft-ha.yaml
kubectl -n tikeo rollout status statefulset/tikeo-server
```

## Evidence and failover drills

Use `scripts/raft-worker-failover-e2e.sh` for release validation or incident drills. The script stores
snapshots under `.dev/reports/...`, including:

- `cluster-diagnostics-*.json`
- `metrics-*.json`
- `fsod-db-*.json` with `cluster_shard_ownership`, `worker_sessions`, `worker_dispatch_outbox`, and
  `dispatch_queue`

Set `TIKEO_RAFT_WORKER_E2E_REPORT_DIR=/path/to/report` to keep evidence for review.

## CRD/operator

- `deploy/k8s/crd/tikeo-manifest-crd.yaml` defines the namespaced `TikeoManifest` CRD.
- `deploy/k8s/operator/` reconciles desired manifests through the GitOps diff endpoint.
- `applyMode=diffOnly` is the safe default; typed CRUD APIs remain the mutation path so RBAC, audit,
  approval, and validation are not bypassed.
