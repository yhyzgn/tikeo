# 042 — Phase 2 Raft ownership gates

## Context
Cluster groundwork exists: `tikeo-server::cluster` defines `ClusterCoordinator`, `ClusterMode`, `ClusterRole`, and `StandaloneCoordinator`. `/api/v1/cluster` now reports explicit standalone status, not a fake leader. Dispatch queue DB conditional claim remains the final idempotency guard.

## Goal
Add scheduling ownership gates before implementing full Raft so future follower nodes cannot accidentally run ownership-sensitive loops.

## Required work
1. Extend `ClusterCoordinator` with an ownership check or use `ClusterStatus.can_schedule` consistently.
2. Gate CRON/fixed-rate tick loop and Worker dispatcher loop on coordinator status.
3. Keep standalone behavior unchanged (`can_schedule=true`).
4. Add tests with a mock follower coordinator proving tick/dispatch ownership-sensitive calls are skipped.
5. Update design/.memory and keep Go/Python SDK deferred to Phase4.

## Constraints
- Do not implement a fake Raft leader.
- Do not remove dispatch_queue DB conditional lease/claim protections.
- No database foreign keys.
- Keep API envelope unchanged.

## Validation
- cargo fmt/clippy/test.
- Commit and push with Lore trailers.
