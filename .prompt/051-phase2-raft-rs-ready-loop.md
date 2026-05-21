# 051 — Phase 2 raft-rs Ready loop and leader fencing

## Context
Current done: `RaftRuntimeCoordinator` starts for `mode=raft` with storage, drives `RawNode::tick()` every 100ms, persists Ready HardState/log/snapshot in order, then advances Ready. It still does not campaign, does not wire outbound transport, and keeps scheduler ownership fenced (`can_schedule=false`, `leader_fencing_token=null`).

The project now uses TiKV raft-rs (`raft` crate 0.7.0). Completed foundations:
- `scheduler-server::cluster::raft_rs` validates stable string node-id -> non-zero raft `u64` id mapping, initial voters, and `MemStorage + RawNode` bootstrap.
- `raft_metadata` / `raft_members` persist local metadata and static peers.
- `raft_log_entries` / `raft_snapshots` exist with SeaORM entities/repository helpers for future Ready log/snapshot persistence; no database foreign keys are used.
- `/api/v1/raft/append-entries` now accepts a raft-rs-message-shaped DTO (`from/to/term/message_type/index/log_term/commit/entries/context/reject`) and validates/converts it to `eraftpb::Message`, but remains non-mutating and returns `accepted=false`.
- `mode=raft` still reports `role=unknown`, `can_schedule=false`, `leader_fencing_token=null`.

## Goal
Continue from the first raft-rs runtime ticker slice without fake leadership.

## Required work
1. Route already-validated inbound raft-rs messages into the runtime inbox, but return clear errors if the runtime is not started.
2. Implement Ready apply/state-machine bookkeeping beyond append persistence.
3. Wire outbound raft-rs messages to container/K8s/LB-safe peer HTTP transport.
4. Only set `can_schedule=true` and `leader_fencing_token` after raft-rs reports real leader state and the token is persisted/fenced.
5. Add dynamic membership/config change boundaries after the transport path is proven.
6. Update design/.memory/roadmap and tests.

## Constraints
- No fake leader. No raft-mode scheduling until proven raft-rs leadership + fencing token.
- No database foreign keys.
- Container/K8s/LB-safe transport only.
- Keep API envelope `{code,message,data}`.

## Validation
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -- --help`
- Web checks only if web files change.
- Commit and push with Lore trailers.
