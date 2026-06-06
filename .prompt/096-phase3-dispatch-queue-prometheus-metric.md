# 096 — Phase 3 dispatch queue Prometheus metric

## Context
Phase 088 added a Grafana dashboard template and Phase 089 added dispatch queue SLO fields to `/api/v1/metrics/summary`, but the dashboard pending-age query still needed a real metric emitted by `/metrics`.

## Objectives
1. Record dispatch queue pending-age SLO values from the existing summary path into the router-local Prometheus recorder.
2. Expose the metric through `/metrics` without relying on global recorder state or external Prometheus.
3. Add regression coverage that proves the summary scrape path makes the metric visible.

## Constraints
- Keep the HTTP envelope `{ code, message, data }` unchanged for JSON APIs.
- Do not pull Phase 4 items back into Phase 3: Node.js SDK, K8s Helm, PowerJob migration, XXL-JOB migration.
- Do not claim complete business SLO coverage; this slice only covers dispatch queue pending-age and pending/running queue gauges.

## Expected verification
- `cargo test -p tikeo-server metrics_summary_reports_storage_registry_and_alert_counts --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`

## Completion notes
- Update `design/tikeo-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md`.
- Commit with Lore trailers and push.
