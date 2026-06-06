# 137 — Phase4 P2 Job Versioning and Rollback

## Goal
Deliver the first Phase4 P2 service-differentiation slice: immutable job version snapshots plus rollback, without introducing large single-file implementations.

## Scope
- Storage: add `job_versions` metadata table/repository/entity with soft links only; no DB foreign keys.
- Server API: expose job version listing and rollback endpoints under `/api/v1/jobs/{job}/versions` and `/api/v1/jobs/{job}/rollback`.
- Behavior: create version 1 when a job is created; append a new version after every successful update; rollback applies a selected historical version by creating a new latest version.
- Web: show current version, expose version history and rollback actions from the Jobs page.
- Docs: mark Phase3/4 P2 `任务版本管理与回滚` as completed after tests pass.

## Constraints
- Keep files modular; split new repository/entity/API helpers rather than growing giant files.
- Preserve current namespace/app immutability for job edits.
- Rollback must use normal job write permission and existing namespace/app scope checks.
- Do not implement PowerJob/XXL-JOB migration tools in this slice; they are lowest-priority backlog.

## Verification
- `cargo test -p tikeo-storage job_version -- --nocapture`
- `cargo test -p tikeo-server job_version -- --nocapture`
- `cd web && bun run lint && bun run build && bun test src/api/client.test.ts`
- `git diff --check`
