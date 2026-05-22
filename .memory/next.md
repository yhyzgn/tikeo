# Next Work

## Immediate next slice
- Continue with `.prompt/093-phase3-script-approval-signature-skeleton.md`.
- Focus areas:
  1. Add the next smallest locally verifiable Phase 3 script governance hardening slice.
  2. Prefer approval/signature metadata/status gates that do not execute scripts on Server and keep Worker execution outbound-only.
  3. Do not pull deferred Phase 4 items back into Phase 3.

## Current status
- Phase 087 added script publish/rollback policy gates for dangerous legacy policy snapshots plus failed audit rows.
- Phase 088 added a deterministic Grafana dashboard template under `observability/grafana/` with local JSON/metric-reference validation.
- Phase 089 added dispatch queue SLO summary fields to `GET /api/v1/metrics/summary` without external services.
- Phase 090 added disabled-by-default OTLP exporter config/readiness status without requiring a collector.
- Phase 091 added alert channel delivery readiness/redaction status without sending real notifications.
- Phase 092 added OIDC authorize/callback skeleton that fails closed without token verification.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
