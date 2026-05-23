# Next Work

## Current pause point
- Java Spring worker demo is currently running against the local tikee server and visible through the Worker API / Worker cluster page.
- Live local processes started for the user: backend `cargo run -- serve --config config/dev.toml`, web `bun run dev`, and Java demo `TIKEE_WORKER_DRY_RUN=false TIKEE_WORKER_ENDPOINT=http://127.0.0.1:9998 ./gradlew bootRun`.
- Resume Phase 3 with the next production gap that can be made locally verifiable without pulling Phase 4 scope back in.

## Remaining production follow-ups intentionally not marked complete
- Real OIDC token exchange/JWKS validation/user mapping/session issuance.
- Real HTTP and Worker Tunnel TLS/mTLS listeners and certificate reload/rotation.
- Full multi-level script approval state machine, verified signatures/KMS, production release gates, and URL/File/Secret grants.
- Remaining alert delivery hardening: production SMTP TLS/auth/secret handling and live provider smoke for external SMTP/Slack/DingTalk/Feishu/WeCom/PagerDuty endpoints.
- Destructive tenant/app/worker-pool lifecycle policy and OIDC identity-to-tenant mapping.
- Remaining business observability hardening: live Prometheus/Grafana recording-rule validation and real OTLP exporter collector smoke.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
