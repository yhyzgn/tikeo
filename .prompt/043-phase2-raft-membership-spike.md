# 043 — Phase 2 Raft membership spike

## Context
Cluster coordinator groundwork and ownership gates are in place. Standalone nodes can schedule; mock follower tests prove CRON/fixed-rate tick and Worker dispatch skip work when `can_schedule=false`. The dispatch_queue DB conditional claim remains the final idempotency guard.

## Goal
Evaluate and implement the first safe Raft membership slice, or document why full Raft should remain deferred.

## Required work
1. Review current openraft crate compatibility with Rust 1.95 and project architecture.
2. Decide persistent Raft metadata storage boundary under `crates/` without foreign keys.
3. Add config shape for cluster mode/node id/peer endpoints if implementation proceeds.
4. Implement only a minimal safe slice: static config parsing + cluster status role source, or full in-memory Raft smoke if feasible.
5. Add tests and update design/.memory.

## Constraints
- No fake leader status.
- No Go/Python SDK work in Phase2.
- Do not remove DB lease/claim protections.
- Container/K8s bridge networking must remain the deployment assumption.

## Validation
- cargo fmt/clippy/test.
- Commit and push with Lore trailers.
