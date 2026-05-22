# 056 — Phase 2 raft-rs dynamic membership proposal API

## Context
The raft-rs apply path now has a safe first real command:
- `raft_member_upsert` updates only the `raft_members` catalog metadata.
- `(cluster_id, command_id)` replay protection runs before side effects.
- `EntryConfChange` / `EntryConfChangeV2` are still gated and must not silently mutate raft membership.
- Leader scheduling remains protected by persisted `leader_fencing_token`.

## Required next work
1. Design and implement a small membership proposal surface for raft mode, guarded by real Leader + persisted fencing token.
2. Keep the API envelope contract exactly `{ code, message, data }`.
3. Add validation for proposed member changes:
   - no database foreign keys;
   - no fake leader / no tokenless proposal;
   - endpoint must be Docker bridge / K8s / LB safe (`http`/`https` absolute URL);
   - prevent removing self or reducing quorum without an explicit safe path.
4. Wire only the proposal intent first if needed; do not apply `EntryConfChange` until committed-entry decoding + ConfState persistence are implemented and tested.
5. Add tests for non-leader rejection, invalid endpoint/status rejection, duplicate proposal idempotency, and config-change entries remaining gated when unsupported.
6. Update `design/tikee-architecture-design.md`, `.memory/*`, and the roadmap checklist.
7. Run full verification and commit/push.

## Constraints
- Do not use database foreign keys.
- Do not enable fake leadership or tokenless scheduling/proposals.
- Do not use Swagger.
- Do not move Go/Python SDK work back from Phase4.
