# 032 — Workflow shards reduce and child workflow callback

## Current context
- Dispatch queue now has real DB conditional claim semantics for pending rows.
- Single job instances create a `dispatch_queue` row at creation time.
- Worker Tunnel dispatcher claims `dispatch_queue` job rows before dispatching and marks the queue row `running` after a successful worker send.
- Workflow queued nodes are claimed before materialization; materialized workflow-node queue rows become `done`.
- Expired pending leases are cleaned each dispatcher tick.

## Next implementation goals
1. Dispatch `workflow_shards` as concrete worker tasks or shard queue rows instead of only persisting pending shard records.
2. Persist shard result output/status and retries; expose a result callback/API if worker protocol is not yet shard-aware.
3. Implement MapReduce reduce semantics: when all map shards succeed, auto-queue/advance the reduce node; if any shard fails beyond retry policy, mark map node failed and advance failure edges.
4. Map child workflow terminal status back to the parent `sub_workflow` node and auto-advance parent successors.
5. Add or extend tests for shard success/failure aggregation and child workflow callback propagation.

## Hard constraints
- No database foreign keys; all relationships remain soft-linked by IDs.
- HTTP responses must remain `{ code, message, data }`.
- Swagger UI is forbidden.
- SDK packages stay under `sdks/`.
- After changes: `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `cargo build --workspace --all-features`, Java SDK tests, web lint/typecheck/test/build when frontend or API DTOs change, update design/.memory/.prompt, commit and push.
