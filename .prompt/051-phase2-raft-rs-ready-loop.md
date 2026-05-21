# 051 — Phase 2 raft-rs Ready loop and leader fencing

## Context
The project now uses TiKV raft-rs (`raft` crate 0.7.0). Completed foundations:
- `scheduler-server::cluster::raft_rs` validates stable string node-id -> non-zero raft `u64` id mapping, initial voters, and `MemStorage + RawNode` bootstrap.
- `raft_metadata` / `raft_members` persist local metadata and static peers.
- `raft_log_entries` / `raft_snapshots` exist with SeaORM entities/repository helpers for future Ready log/snapshot persistence; no database foreign keys are used.
- `/api/v1/raft/append-entries` now accepts a raft-rs-message-shaped DTO (`from/to/term/message_type/index/log_term/commit/entries/context/reject`) but remains non-mutating and returns `accepted=false`.
- `mode=raft` still reports `role=unknown`, `can_schedule=false`, `leader_fencing_token=null`.

## Goal
Implement the first real raft-rs runtime loop slice without fake leadership.

## Required work
1. Add a `RaftRuntimeCoordinator` behind `ClusterCoordinator` for `mode=raft`.
2. Drive `RawNode::tick()` on an internal interval and expose observed raft-rs role in diagnostics.
3. Implement Ready handling in the correct order:
   - persist HardState into `raft_metadata`;
   - persist entries into `raft_log_entries`;
   - persist snapshot metadata/pointer into `raft_snapshots` when present;
   - only then advance/apply Ready.
4. Route inbound raft-rs message DTOs into the runtime inbox, but reject/return clear errors if the runtime is not started.
5. Only set `ClusterRole::Leader`, `can_schedule=true`, and `leader_fencing_token` after raft-rs reports real leader state and the token is persisted/fenced.
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
