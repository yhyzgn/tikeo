# 040 — Phase 2 realtime task log stream

## Context
Go/Python SDKs are explicitly postponed to Phase 4. Continue Phase 2 by completing realtime logs. Existing Worker Tunnel already accepts `TaskLog` and persists it to `job_instance_logs`; HTTP currently exposes pull-based `/api/v1/instances/{instance}/logs`.

## Goal
Implement realtime task log streaming over gRPC server streaming, backed by persisted logs plus live fan-out.

## Required work
1. Extend Worker Tunnel proto copies with `SubscribeTaskLogs(SubscribeTaskLogsRequest) returns (stream TaskLog)`.
2. Server should replay existing logs for an instance, then stream new logs appended through Worker Tunnel.
3. Keep persisted logs as source of truth; live stream is best-effort and must not replace DB writes.
4. Add tests for replay + live delivery.
5. Update design roadmap and `.memory`.

## Validation
- Regenerate/check Rust protobuf build through cargo tests.
- Update SDK proto copies if needed but do not implement Go/Python SDKs now.
- Run cargo fmt/clippy/test and relevant SDK smoke tests if protocol changes affect SDK builds.
- Commit and push with Lore trailers.
