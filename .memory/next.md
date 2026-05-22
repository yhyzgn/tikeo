# Next Work

## Immediate next slice
- Continue with `.prompt/081-alert-recovery-and-notifications.md`.
- Focus areas:
  1. Add alert recovery transitions and notification history shaping for the existing `alert_events` table.
  2. Surface alert rule/status management cleanly through Web/API while preserving the no-external-webhook verification constraint.
  3. Keep the Phase 3 roadmap moving toward the remaining governance, metrics, and tracing items.

## Current status
- Phase 079 materialized `script_execution_governance` failures into `audit_logs` with `failure_reason` filtering and Web audit-page support.
- Phase 080 added `alert_rules` / `alert_events` storage plus HTTP APIs and deterministic governance-driven alert event history.
- Instance log compatibility from 078 remains intact; governance logs are still queryable via `page_token=script_execution_governance`.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
