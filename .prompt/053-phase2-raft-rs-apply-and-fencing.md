# 053 — Phase 2 raft-rs Ready apply bookkeeping and leader fencing

## Context
Completed raft-rs Phase2 safe slices:
- TiKV raft-rs (`raft` crate 0.7.x) bootstrap with stable string node id -> non-zero raft `u64` id mapping;
- no-FK raft metadata/member/log/snapshot persistence foundations;
- inbound `/api/v1/raft/append-entries` validation -> `eraftpb::Message` conversion -> runtime inbox submission;
- 100ms `RawNode::tick()` runtime loop;
- Ready persistence order skeleton: HardState -> log entries -> snapshot -> outbound messages -> `advance()`;
- outbound HTTP skeleton: Ready messages serialize back to the existing wire DTO, target configured peer endpoints via `/api/v1/raft/append-entries`, and optionally carry `x-scheduler-raft-token` from `cluster.transport_token`.

## Hard safety rule
Do **not** enable raft-mode scheduling from role alone. `ClusterRole::Leader` is not enough. `can_schedule=true` is allowed only after a leader fencing token is generated, persisted in `raft_metadata.leader_fencing_token`, and consumed by scheduler/dispatcher ownership gates.

## Required next work
1. Implement Ready committed-entry apply bookkeeping:
   - iterate `ready.committed_entries()` after durability requirements are met;
   - update `raft_metadata.applied_index` safely;
   - reject/apply config-change entries deliberately (no silent membership changes yet).
2. Decide and implement the first fencing-token lifecycle:
   - derive only from real raft-rs leader status/term/node id;
   - persist token before reporting `can_schedule=true`;
   - clear token when not leader.
3. Add tests for applied index persistence, config-change gating, token persistence/clearing, and dispatcher/scheduler gates still refusing unfenced raft nodes.
4. Update design/.memory/roadmap and this prompt chain.
5. Run full verification, commit with Lore trailers, and push.

## Constraints
- DB全库严禁外键；只能软关联字段。
- API envelope remains `{ code, message, data }`.
- No Swagger UI.
- Docker bridge / K8s / LB-safe networking only.
- Go SDK + Python SDK remain Phase4.
