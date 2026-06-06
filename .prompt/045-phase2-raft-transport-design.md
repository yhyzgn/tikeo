# 045 — Phase 2 Raft transport and leader fencing design

## Context
Raft mode now has safe config and persistence foundations: `[cluster] mode/node_id/peers`, `raft_metadata`, and `raft_members`. Startup persists configured metadata/members, but cluster status intentionally remains `role=unknown` and `can_schedule=false` until a real consensus runtime establishes leadership. Consensus implementation direction is TiKV raft-rs (`raft` crate 0.7.0); bootstrap validation exists, but event loop/transport/fencing remain gated.

## Goal
Design and implement the next smallest safe cluster slice without fake leadership.

## Required work
1. Add explicit Raft transport/API design for node-to-node communication over container/K8s/LB-safe HTTP/gRPC endpoints.
2. Add a leader fencing token shape to cluster status/storage design, but do not mark nodes schedulable without real consensus.
3. Decide whether to keep the current storage-backed no-op coordinator or introduce a `tikeo-cluster` crate boundary.
4. If implementation proceeds, add only transport DTOs/routes or fencing-token plumbing that cannot create fake leader behavior.
5. Update design/.memory/roadmap.

## Constraints
- No fake leader and no `can_schedule=true` in raft mode until real consensus state exists.
- No database foreign keys; Raft tables remain soft-linked by ids.
- Go/Python SDK remains Phase4.
- Docker bridge/K8s/LB networking assumptions must be preserved.

## Validation
- cargo fmt/clippy/test.
- Commit and push with Lore trailers.
