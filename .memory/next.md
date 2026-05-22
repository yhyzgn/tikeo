# Next Work

## Immediate next slice
- Continue with `.prompt/083-metrics-summary-and-slo.md`.
- Focus areas:
  1. Add deterministic `/api/v1/metrics/summary` for operator dashboards without requiring Prometheus/Grafana in tests.
  2. Summarize existing storage/state signals: instance status counts, worker online count, alert event counts, and governance failure counts.
  3. Continue remaining Phase 3 governance/metrics/tracing work after metrics summary lands.

## Current status
- Phase 079 materialized `script_execution_governance` failures into `audit_logs` with `failure_reason` filtering and Web audit-page support.
- Phase 080 added `alert_rules` / `alert_events` storage plus HTTP APIs and deterministic governance-driven alert event history.
- Phase 081 added deterministic alert recovery transitions and a resolve endpoint that appends `recovered` event history rows.
- Phase 082 added `GET /api/v1/alert-events:summary` rollups by rule/resource/failure class with firing/suppressed/silenced/recovered counts.
- Instance log compatibility from 078 remains intact; governance logs are still queryable via `page_token=script_execution_governance`.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
