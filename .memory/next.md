# Next Work

## Immediate next slice
- Continue with `.prompt/086-mtls-transport-foundation.md`.
- Focus areas:
  1. Add TLS/mTLS configuration and status diagnostics without requiring real certificates in tests.
  2. Keep local dev listeners and SDK demos unchanged by default.
  3. Continue remaining Phase 3 security/governance work after mTLS foundation lands.

## Current status
- Phase 079 materialized `script_execution_governance` failures into `audit_logs` with `failure_reason` filtering and Web audit-page support.
- Phase 080 added `alert_rules` / `alert_events` storage plus HTTP APIs and deterministic governance-driven alert event history.
- Phase 081 added deterministic alert recovery transitions and a resolve endpoint that appends `recovered` event history rows.
- Phase 082 added `GET /api/v1/alert-events:summary` rollups by rule/resource/failure class with firing/suppressed/silenced/recovered counts.
- Phase 083 added `GET /api/v1/metrics/summary` with worker, instance, alert, and governance counts for local dashboard/SLO groundwork.
- Phase 084 added HTTP trace-id propagation/generation and local tracing spans without external OTLP collector requirements.
- Phase 085 added OIDC/SSO config/status foundation while preserving local login.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
