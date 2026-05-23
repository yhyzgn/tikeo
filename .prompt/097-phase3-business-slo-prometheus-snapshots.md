# 097 — Phase 3 business SLO Prometheus snapshots

## Context
Phase 096 made the dispatch queue pending-age Grafana SLO query backed by a real Prometheus histogram. The metrics summary endpoint already has additional business/SLO data for workers, instances, alerts, and script governance failures that should also be visible through `/metrics`.

## Objectives
1. Record worker online, job instance status, job instance success ratio, alert status, and script governance failure counts into the router-local Prometheus recorder.
2. Keep the data source local and deterministic by reusing `GET /api/v1/metrics/summary` aggregation rather than requiring a live Prometheus server.
3. Extend regression coverage to prove the summary-then-scrape path emits the new metric names.

## Constraints
- Preserve the HTTP envelope `{ code, message, data }`.
- Do not pull Phase 4 items back into Phase 3: Node.js SDK, K8s Helm, PowerJob migration, XXL-JOB migration.
- Do not mark full business SLO coverage complete; end-to-end dispatch latency histograms, workflow/map-reduce SLA, and live recording-rule validation remain future work.

## Expected verification
- `cargo test -p tikee-server metrics_summary_reports_storage_registry_and_alert_counts --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md`.
- Commit with Lore trailers and push.
