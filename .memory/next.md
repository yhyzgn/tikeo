# Next Work

## Immediate next slice
- Continue with `.prompt/092-phase3-oidc-callback-skeleton.md`.
- Focus areas:
  1. Add the next smallest locally verifiable Phase 3 security hardening slice.
  2. Prefer OIDC login/callback shape validation or state/callback readiness without requiring a live IdP.
  3. Do not pull deferred Phase 4 items back into Phase 3.

## Current status
- Phase 086 added TLS/mTLS config/status diagnostics while keeping dev plaintext defaults.
- Phase 087 added script publish/rollback policy gates for dangerous legacy policy snapshots plus failed audit rows.
- Phase 088 added a deterministic Grafana dashboard template under `observability/grafana/` with local JSON/metric-reference validation.
- Phase 089 added dispatch queue SLO summary fields to `GET /api/v1/metrics/summary` without external services.
- Phase 090 added disabled-by-default OTLP exporter config/readiness status without requiring a collector.
- Phase 091 added alert channel delivery readiness/redaction status without sending real notifications.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
