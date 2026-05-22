# 054 — Phase 2 raft-rs business apply and membership boundaries

## Context
Phase2 raft-rs safe runtime foundations now include:
- bootstrap/config/storage boundary for TiKV raft-rs (`raft` crate 0.7.x);
- no-FK metadata/member/log/snapshot persistence;
- inbound HTTP raft transport -> runtime inbox;
- outbound peer HTTP transport skeleton with optional `cluster.transport_token` / `x-tikee-raft-token`;
- Ready durability order using `advance_append` / `advance_apply_to`;
- committed `EntryNormal` apply bookkeeping that advances `raft_metadata.applied_index` monotonically;
- explicit gating for `EntryConfChange` / `EntryConfChangeV2` so membership is never silently changed;
- leader fencing token lifecycle: only real raft-rs `Leader` with term > 0 can derive a token, token is persisted before `can_schedule=true`, non-leader clears token.

## Hard safety rule
Do not weaken the no-fake-leader guarantee. Raft mode may schedule only when status has `can_schedule=true` **and** a persisted `leader_fencing_token`; tikee/dispatcher ownership gates must continue to consume that boundary.

## Completed in 054
1. Added `raft_applied_commands` durable no-FK table/entity/repository for idempotent state-machine bookkeeping.
2. Defined the first `EntryNormal` command envelope: `{ "command_id": string, "command_type": string, "payload": object }`.
3. Implemented safe `noop` apply semantics; unknown command types are recorded as `deferred_unsupported`; invalid JSON is recorded as `rejected`.
4. Apply records are idempotent by `(node_id, log_index)` and command ids are reserved by `(cluster_id, command_id)`.
5. Config-change entries remain gated.

## Continue with 055
See `.prompt/055-phase2-raft-rs-real-business-commands-and-membership.md`.

## Constraints
- API envelope remains `{ code, message, data }`.
- DB 全库严禁外键；只能软关联字段。
- No Swagger UI.
- Docker bridge / K8s / LB-safe networking only.
- Go SDK + Python SDK remain Phase4.
