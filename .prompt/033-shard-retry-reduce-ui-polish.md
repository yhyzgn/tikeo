# 033 — Shard retry, reduce inputs, and UI polish

## Current context
- `workflow_shards` has `job_instance_id` soft linkage.
- Map/map_reduce materialization creates shard rows, per-shard job_instance rows, and dispatch_queue rows.
- Worker TaskResult for a shard job_instance completes the shard and lets shard aggregation advance the map node only when aggregate status is terminal.
- `POST /api/v1/workflow-shards/{id}/complete` persists shard status/output and audits the operation.
- Child workflow materialization initializes child nodes/queues; child terminal status propagates to the parent `sub_workflow` node.

## Next implementation goals
1. Add shard retry policy fields or config interpretation: max attempts, retry backoff, failed shard retry API.
2. Feed shard outputs into a reduce node context/input model instead of only advancing the next node.
3. Improve Workflows UI shard panel: show `job_instance_id`, output, terminal status color, and a manual shard completion/retry action for dev/testing.
4. Add tests for failed shard propagation and child workflow failure propagation.
5. Consider job log SSE follow endpoint if instance logs remain pull-only.

## Hard constraints
- No DB foreign keys; use soft IDs only.
- HTTP envelope remains `{ code, message, data }`.
- Swagger UI is forbidden.
- SDK packages stay under `sdks/`.
- After changes: fmt, clippy, tests, build, relevant web checks, update design/.memory/.prompt, commit and push.
