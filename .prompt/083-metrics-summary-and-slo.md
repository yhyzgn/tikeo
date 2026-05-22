# 083 — Metrics summary and SLO dashboard API

## Context
Phase 082 added operator-friendly alert notification history rollups over the persistent `alert_events` stream. The Phase 3 roadmap still has Prometheus/Grafana work partially complete: `/metrics` exists, but the management API lacks a deterministic `/api/v1/metrics/summary` shape for dashboard/SLO data and local tests.

## Objectives
1. Add a local, deterministic metrics summary API for platform operators without requiring Prometheus/Grafana in tests.
2. Summarize core runtime health signals from existing storage/state where possible: job/instance status counts, worker online count, alert firing/suppressed/recovered counts, and recent governance failure counts.
3. Preserve the standard `{ code, message, data }` envelope and existing auth/RBAC behavior.

## Constraints
- Do not require external Prometheus, Grafana, OTLP, or network smoke.
- No database foreign keys.
- Keep modules split by responsibility; do not grow HTTP route files into monoliths.
- Do not add Swagger.

## Expected verification
- Targeted Rust tests for `/api/v1/metrics/summary` counts and envelope.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/084-*.md` before commit.
- Commit with Lore trailers and push.
