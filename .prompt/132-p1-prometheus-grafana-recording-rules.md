# 132 — P1 Prometheus/Grafana recording-rule validation

## Context
Previous P1 OIDC tenant/app/role mapping is complete. Continue production observability hardening without exceeding the source-file size rule.

## Goal
Close the Prometheus/Grafana operational validation gap: committed recording rules, Prometheus scrape config, Grafana dashboard queries aligned with recording series, and a runbook operators can execute locally.

## Scope
- Add `observability/prometheus/tikee-recording-rules.yml` for core HTTP, dispatch queue, workflow, worker, and script-governance SLO series.
- Add a minimal Prometheus config that scrapes tikee `/metrics` and loads the rules.
- Wire Docker Compose observability profile for optional Prometheus smoke.
- Update Grafana dashboard queries to prefer recording rules where appropriate.
- Add tests that parse the rule/dashboard assets and verify referenced metrics/rules are coherent.
- Add an operations runbook for scrape, recording-rule, and Grafana triage.

## Validation target
- `cargo test -p tikee-server grafana --all-features`
- `cargo check -p tikee-server --all-features`
- source-size check excluding generated/build artifacts remains `<1500` lines per file.
