# 051 — Phase 2 raft-rs Ready loop and leader fencing

## Context
Current done: `RaftRuntimeCoordinator` starts for `mode=raft` with storage, drives `RawNode::tick()` every 100ms, persists Ready HardState/log/snapshot in order, advances Ready, and accepts already-validated HTTP raft messages into a bounded runtime inbox. It still does not campaign, does not wire outbound transport, and keeps tikeo ownership fenced (`can_schedule=false`, `leader_fencing_token=null`).

The project now uses TiKV raft-rs (`raft` crate 0.7.0). Completed foundations:
- `tikeo-server::cluster::raft_rs` validates stable string node-id -> non-zero raft `u64` id mapping, initial voters, and `MemStorage + RawNode` bootstrap.
- `raft_metadata` / `raft_members` persist local metadata and static peers.
- `raft_log_entries` / `raft_snapshots` exist with SeaORM entities/repository helpers for future Ready log/snapshot persistence; no database foreign keys are used.
- `/api/v1/raft/append-entries` now accepts a raft-rs-message-shaped DTO (`from/to/term/message_type/index/log_term/commit/entries/context/reject`), validates/converts it to `eraftpb::Message`, and returns `accepted=true` only when the raft runtime inbox accepts the message.
- `mode=raft` exposes the raft-rs observed role but still reports `can_schedule=false`, `leader_fencing_token=null`.

## Goal
This 051 slice is complete: runtime ticker, Ready persistence order, and inbound runtime inbox are now wired without fake leadership. Continue with `.prompt/052-phase2-raft-rs-outbound-apply-fencing.md`.

## Completed work
1. `RaftRuntimeCoordinator` starts for `mode=raft` with a 100ms ticker.
2. Ready HardState/log/snapshot persistence runs before `advance()`.
3. Already-validated inbound raft-rs messages are submitted into the runtime inbox.
4. `can_schedule` remains `false` and `leader_fencing_token` remains `null`.
5. Design/.memory/roadmap/tests were updated for this slice.

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
