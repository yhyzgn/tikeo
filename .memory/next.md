# Next Work

## Immediate next slice
- Continue with `.prompt/080-alert-rule-event-history.md`.
- Focus areas:
  1. Add alert rule management/query API for the existing `AlertRule` / `AlertCondition` model.
  2. Add deterministic alert event history materialization for script governance failures and recovery-capable state transitions.
  3. Keep notification dispatch safe and test-only: no external webhook smoke required unless explicitly enabled.

## Current status
- Phase 079 materialized `script_execution_governance` failures into `audit_logs` with `failure_reason` filtering and Web audit-page support.
- Instance log compatibility from 078 remains intact; governance logs are still queryable via `page_token=script_execution_governance`.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
