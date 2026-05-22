# 054 — Phase 2 raft-rs business apply and membership boundaries

## Context
Phase2 raft-rs safe runtime foundations now include:
- bootstrap/config/storage boundary for TiKV raft-rs (`raft` crate 0.7.x);
- no-FK metadata/member/log/snapshot persistence;
- inbound HTTP raft transport -> runtime inbox;
- outbound peer HTTP transport skeleton with optional `cluster.transport_token` / `x-scheduler-raft-token`;
- Ready durability order using `advance_append` / `advance_apply_to`;
- committed `EntryNormal` apply bookkeeping that advances `raft_metadata.applied_index` monotonically;
- explicit gating for `EntryConfChange` / `EntryConfChangeV2` so membership is never silently changed;
- leader fencing token lifecycle: only real raft-rs `Leader` with term > 0 can derive a token, token is persisted before `can_schedule=true`, non-leader clears token.

## Hard safety rule
Do not weaken the no-fake-leader guarantee. Raft mode may schedule only when status has `can_schedule=true` **and** a persisted `leader_fencing_token`; scheduler/dispatcher ownership gates must continue to consume that boundary.

## Required next work
1. Define the first business state-machine command envelope for raft `EntryNormal` payloads.
2. Implement a small, idempotent apply path for one safe command type, or explicitly document why business apply remains deferred.
3. Add a durable applied-command/audit shape if needed, still with no database foreign keys.
4. Design dynamic membership/config-change handling without silently applying `EntryConfChange` entries; update docs and tests.
5. Add integration tests around replay/idempotency and token-gated scheduling/dispatch ownership.
6. Run full verification, commit with Lore trailers, and push.

## Constraints
- API envelope remains `{ code, message, data }`.
- DB 全库严禁外键；只能软关联字段。
- No Swagger UI.
- Docker bridge / K8s / LB-safe networking only.
- Go SDK + Python SDK remain Phase4.
