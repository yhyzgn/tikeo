# 053 — Phase 2 raft-rs Ready apply bookkeeping and leader fencing

## Context
Completed raft-rs Phase2 safe slices:
- TiKV raft-rs (`raft` crate 0.7.x) bootstrap with stable string node id -> non-zero raft `u64` id mapping;
- no-FK raft metadata/member/log/snapshot persistence foundations;
- inbound `/api/v1/raft/append-entries` validation -> `eraftpb::Message` conversion -> runtime inbox submission;
- 100ms `RawNode::tick()` runtime loop;
- Ready persistence order skeleton: HardState -> log entries -> snapshot -> outbound messages -> `advance()`;
- outbound HTTP skeleton: Ready messages serialize back to the existing wire DTO, target configured peer endpoints via `/api/v1/raft/append-entries`, and optionally carry `x-tikee-raft-token` from `cluster.transport_token`.

## Hard safety rule
Do **not** enable raft-mode scheduling from role alone. `ClusterRole::Leader` is not enough. `can_schedule=true` is allowed only after a leader fencing token is generated, persisted in `raft_metadata.leader_fencing_token`, and consumed by tikee/dispatcher ownership gates.

## Completed in 053
1. Implemented Ready committed-entry apply bookkeeping with `advance_append` / `advance_apply_to`.
2. `EntryNormal` committed entries now monotonically update `raft_metadata.applied_index`.
3. `EntryConfChange` and `EntryConfChangeV2` are explicitly gated and never silently applied.
4. Added a leader fencing-token lifecycle: only real raft-rs `Leader` with term > 0 derives a token, persists it first, then reports `can_schedule=true`; non-leaders clear the token.
5. Added repository/runtime tests for applied-index monotonicity, config-change gating, and token derivation.

## Continue with 054
See `.prompt/054-phase2-raft-rs-business-apply-membership.md` for business apply and membership boundaries.

## Constraints
- DB全库严禁外键；只能软关联字段。
- API envelope remains `{ code, message, data }`.
- No Swagger UI.
- Docker bridge / K8s / LB-safe networking only.
- Go SDK + Python SDK remain Phase4.
