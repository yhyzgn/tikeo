# 044 — Phase 2 Raft runtime evaluation

## Context
Cluster config shape exists: `[cluster] mode/node_id/peers`. `mode=raft` currently reports `role=unknown`, `can_schedule=false`, and does not run ownership loops. This is intentional to avoid fake leader semantics before consensus exists.

## Goal
Evaluate and introduce the first real Raft runtime slice, or explicitly defer with evidence if the slice is too broad.

## Required work
1. Check openraft compatibility and required storage traits for Rust 1.95 / current dependency graph.
2. Decide whether to add a new `crates/scheduler-cluster` crate or keep runtime under `scheduler-server::cluster` for now.
3. Implement the smallest honest runtime slice:
   - in-memory single-node Raft bootstrap that can become leader, OR
   - persisted metadata schema + no-op runtime with explicit blocked status if real runtime is too broad.
4. Ensure `ClusterCoordinator` remains the only ownership source for tick/dispatcher loops.
5. Update design/.memory/roadmap.

## Constraints
- No fake leader without real consensus state.
- No DB foreign keys.
- Go/Python SDK remains Phase4.
- Keep bridge/K8s networking assumption.

## Validation
- cargo fmt/clippy/test.
- Commit and push with Lore trailers.
