# Next Work

## Immediate next slice
- Continue with `.prompt/090-phase3-otel-exporter-foundation.md`.
- Focus areas:
  1. Add the next smallest locally verifiable Phase 3 tracing/observability hardening slice.
  2. Prefer OTLP exporter configuration/readiness plumbing that can be tested without a live collector.
  3. Do not pull deferred Phase 4 items back into Phase 3.

## Current status
- Phase 084 added HTTP trace-id propagation/generation and local tracing spans without external OTLP collector requirements.
- Phase 085 added OIDC/SSO config/status foundation while preserving local login.
- Phase 086 added TLS/mTLS config/status diagnostics while keeping dev plaintext defaults.
- Phase 087 added script publish/rollback policy gates for dangerous legacy policy snapshots plus failed audit rows.
- Phase 088 added a deterministic Grafana dashboard template under `observability/grafana/` with local JSON/metric-reference validation.
- Phase 089 added dispatch queue SLO summary fields to `GET /api/v1/metrics/summary` without external services.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
