---
title: Worker Tunnel protobuf reference
description: Source-backed reference for the outbound Worker Tunnel gRPC/protobuf contract used by SDKs and server dispatch.
---

# Worker Tunnel protobuf reference

This reference is curated from `crates/tikeo-proto/proto/worker.proto`, the
canonical server-side protobuf contract for the Worker Tunnel. SDK packages
bundle generated or copied bindings, but protocol changes must start from this
source file. The current package is `package tikeo.worker.v1`.

The Worker Tunnel is outbound-only from the business worker process to Tikeo
Server. Workers do not expose inbound ports. Server-to-worker actions such as
dispatch are written back on the existing stream, which keeps cross-cluster,
cross-VPC, NAT, and Kubernetes namespace deployments simple.

## Service surface

```protobuf
service WorkerTunnelService {
  rpc OpenTunnel(stream WorkerMessage) returns (stream ServerMessage);
  rpc SubscribeTaskLogs(SubscribeTaskLogsRequest) returns (stream TaskLog);
}
```

`WorkerTunnelService.OpenTunnel` is the long-lived bidirectional stream used for
registration, heartbeats, dispatch, logs, task results, unregister, and
checkpoints. `SubscribeTaskLogs` exposes task log streaming for consumers that
need replay plus live log updates.

## Message direction map

| Direction | Message | Purpose |
| --- | --- | --- |
| Worker → Server | `RegisterWorker` | Sends namespace, app, cluster, region, labels, legacy capabilities, structured capabilities, and optional election settings. |
| Worker → Server | `Heartbeat` | Renews the lease with `worker_id`, generation, sequence, and fencing token. |
| Worker → Server | `TaskLog` | Streams task log lines with `instance_id`, level, message, sequence, and `assignment_token`. |
| Worker → Server | `TaskResult` | Completes an assigned task with success flag, message, and `assignment_token`. |
| Worker → Server | `TaskCheckpoint` | Persists resumable progress for long-running work using `checkpoint_json`. |
| Worker → Server | `UnregisterWorker` | Gracefully closes the authoritative worker session. |
| Server → Worker | `WorkerRegistered` | Returns the server-assigned `worker_id`, lease seconds, generation, and fencing token. |
| Server → Worker | `Ping` | Keeps the stream active and measures liveness. |
| Server → Worker | `DispatchTask` | Sends work to a selected worker over the existing outbound tunnel. |

## RegisterWorker

`RegisterWorker` carries the worker's logical scope. The optional
`client_instance_id` is only a client-side stable hint; Tikeo assigns the
authoritative `worker_id` in `WorkerRegistered`. New routing should prefer
`structured_capabilities` over legacy string capabilities. Worker cluster
election uses `WorkerClusterElection` with an optional stable domain and
deterministic priority.

## WorkerRegistered and Heartbeat

`WorkerRegistered` returns the authority that the worker must echo in future
messages: `worker_id`, `generation`, and `fencing_token`. `Heartbeat` includes
the same identity data plus a monotonic sequence. These fields let the server
reject stale incarnations after reconnects, replacement sessions, or lease
expiry.

## DispatchTask

`DispatchTask` is the key Server → Worker command. It contains:

- `instance_id` and `job_id` for the scheduled execution record.
- `payload` bytes for processor input.
- `processor_name`, the explicit SDK routing key used by Java, Rust, Go,
  Python, Node.js, and future SDK adapters.
- `processor_binding` for dynamic script or WASM execution metadata.
- `assignment_token`, the server-issued authority that must be echoed by logs,
  checkpoints, and results.

SDK docs should link processor helper behavior to this message because workers
route incoming tasks by `processor_name` and prove assignment ownership with
`assignment_token`.

## TaskLog, TaskResult, and TaskCheckpoint

`TaskLog` and `TaskResult` are worker-authored evidence. `TaskLog` persists
operator-visible progress and is later readable through
`/api/v1/instances/{instance}/logs`. `TaskResult` moves the instance to a
terminal success or failure state. Both include `assignment_token`; workers must
not fabricate a result for work they were not assigned.

`TaskCheckpoint` supports resumable long-running tasks by attaching ordered
`checkpoint_json` snapshots to an instance. It uses the same worker identity and
assignment-token boundary as logs and results.

## Dynamic processor bindings

`TaskProcessorBinding` can contain `ScriptProcessorBinding` or
`WasmProcessorBinding`. These bindings are immutable execution snapshots:
script/module bytes, version identifiers, SHA-256 integrity fields, runtime
limits, network/file/env grants, and sandbox backend metadata. The Server
distributes the snapshot over the tunnel; the worker enforces execution policy.
The Server still does not run user code.

## Operational invariants

- Worker processes initiate `OpenTunnel`; do not expose business-worker inbound
  ports.
- The server-assigned `worker_id` is authoritative; `client_instance_id` is only
  a hint.
- Capabilities must describe real executable support. Do not advertise script
  or plugin runners that fail open or are missing runtime tools.
- Logs, checkpoints, and results must carry the `assignment_token` from
  `DispatchTask`.
- HTTP management APIs observe and trigger work, but actual execution evidence
  flows through `WorkerTunnelService`.
