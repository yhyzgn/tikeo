# Workers user guide

The Workers page is implemented by `web/src/pages/WorkersPage.tsx`. It displays Worker Tunnel connectivity, structured capabilities, persisted lifecycle history, and current dispatch capacity. It is the first place to verify whether `DispatchTask` messages can reach an eligible Worker.

## Source-backed data paths

The page reads `/api/v1/workers`, `/api/v1/workers/history`, and the Worker SSE stream. Worker execution itself uses the Worker Tunnel protocol documented in the protobuf reference: `WorkerTunnelService`, `OpenTunnel`, `RegisterWorker`, `Heartbeat`, `DispatchTask`, `TaskLog`, `TaskResult`, and `TaskCheckpoint`.

## Understanding Worker Tunnel state

Workers connect outbound to the Server; the Server does not require business Workers to expose inbound ports. Online status reflects the active tunnel registry, while persisted snapshots keep recent visibility after reconnects or server restarts. If a Worker disappears from the live list, inspect lifecycle events before assuming all capacity is gone.

## Capability and routing checks

Structured capabilities are the routing contract. SDK processors, script runners, labels, worker pool, namespace, app, region, or cluster must be advertised by the Worker before jobs should depend on them. A Worker should not advertise a sandbox or script runner that it cannot execute.

## Dispatch queue handoff

The page links to the dispatch queue surface for deeper scheduling triage. If Jobs show pending instances while Workers look healthy, compare job processor binding and broadcast selector requirements with the Worker table. If no Worker matches, fix Worker registration or job scope rather than retrying blindly.
