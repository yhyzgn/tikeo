# 050 — Phase 2 raft-rs event loop and fencing runtime

## Context
The project has switched the consensus plan from OpenRaft to TiKV raft-rs (`raft` crate 0.7.0). `tikeo-server::cluster::raft_rs` now validates the crate/config/storage boundary by deriving stable non-zero `u64` raft ids from string node ids, building initial voters from `[cluster].peers`, and constructing `MemStorage + RawNode` without ticking, campaigning, or granting tikeo ownership. `mode=raft` still returns `role=unknown`, `can_schedule=false`, and `leader_fencing_token=null`.

## Goal
Implement the next safe raft-rs runtime slice without fake leadership.

## Required work
1. Design and implement the raft-rs event-loop skeleton behind `ClusterCoordinator`:
   - drive `tick()` on an interval;
   - consume inbound raft messages from the reserved transport;
   - process `Ready` only after persistence boundaries are explicit.
2. Replace the placeholder `/api/v1/raft/append-entries` shape with a raft-rs message transport DTO/API, while preserving the `{code,message,data}` envelope for management-facing routes.
3. Define durable storage mapping for raft-rs HardState/log entries/snapshots in SeaORM migrations, still with zero database foreign keys.
4. Only set `ClusterRole::Leader`, `can_schedule=true`, and `leader_fencing_token` after the real raft-rs state reports leadership and the leader token is persisted/fenced.
5. Update design/.memory/roadmap and diagnostics to explain the exact runtime state.

## Constraints
- No fake leader and no scheduling in raft mode before proven consensus leadership.
- No database foreign keys; all Raft tables use soft id relationships.
- Container/K8s/LB-safe transport only; no host network assumptions.
- Keep SDKs independently publishable and outside server Docker build scope.

## Validation
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -- --help`
- Web checks only if web files change.
- Commit and push with Lore trailers after all checks pass.
