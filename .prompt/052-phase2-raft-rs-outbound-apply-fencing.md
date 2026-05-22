# 052 — Phase 2 raft-rs outbound transport, apply bookkeeping, and fencing prep

## Context
The project uses TiKV raft-rs (`raft` crate 0.7.x) for the tikee server cluster direction. Completed safe slices:
- stable string `node_id` -> non-zero raft `u64` mapping and `RawNode` bootstrap validation;
- no-FK durable raft metadata/member/log/snapshot records;
- `/api/v1/raft/append-entries` DTO validation and conversion into `eraftpb::Message`;
- `RaftRuntimeCoordinator` ticker loop that drives `RawNode::tick()` every 100ms;
- Ready persistence order skeleton: HardState -> log entries -> snapshot -> `advance()`;
- inbound runtime inbox: validated HTTP messages are submitted to a bounded mpsc channel and stepped by the runtime loop.

## Hard safety rule
Do **not** set `can_schedule=true`, do **not** emit `leader_fencing_token`, and do **not** let dispatch/tick ownership run from raft mode until real raft-rs leader state plus persisted fencing token are implemented and consumed by the existing dispatch gates.

## Completed in 052
1. Added an outbound peer transport skeleton for raft-rs `Ready.messages()` to configured peer endpoints over HTTP.
2. Converted outbound `eraftpb::Message` values back into the existing wire DTO shape with base64 payload encoding.
3. Added optional `cluster.transport_token` / `x-tikee-raft-token` for internal server-to-server Raft HTTP auth.
4. Added tests for outbound message serialization and endpoint path construction.

## Remaining / continue with 053
- Implement Ready committed-entry apply bookkeeping and applied-index persistence.
- Implement leader fencing-token lifecycle before any raft-mode scheduling authority.
- See `.prompt/053-phase2-raft-rs-apply-and-fencing.md`.

## Current constraints
- API responses must always use `{ code, message, data }`.
- DB全库严禁外键；只能软关联字段。
- Backend crates stay under `crates/`; backend entrypoint remains at repo root.
- Web stays under `web/` with React + Ant Design + Bun.
- Go SDK + Python SDK remain Phase4.
