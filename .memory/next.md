# Next Work

## Immediate next slice
- Continue with `.prompt/084-opentelemetry-tracing-foundation.md`.
- Focus areas:
  1. Add minimal OpenTelemetry/tracing foundation without requiring an external collector in tests.
  2. Preserve local `tracing_subscriber` output and the standard HTTP envelope.
  3. Continue remaining Phase 3 governance/security work after tracing foundation lands.

## Current status
- Phase 079 materialized `script_execution_governance` failures into `audit_logs` with `failure_reason` filtering and Web audit-page support.
- Phase 080 added `alert_rules` / `alert_events` storage plus HTTP APIs and deterministic governance-driven alert event history.
- Phase 081 added deterministic alert recovery transitions and a resolve endpoint that appends `recovered` event history rows.
- Phase 082 added `GET /api/v1/alert-events:summary` rollups by rule/resource/failure class with firing/suppressed/silenced/recovered counts.
- Phase 083 added `GET /api/v1/metrics/summary` with worker, instance, alert, and governance counts for local dashboard/SLO groundwork.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
