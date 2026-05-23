# Next Work

## Current pause point
- Paused after `.prompt/097-phase3-business-slo-prometheus-snapshots.md`: existing summary data now emits real Prometheus gauges for worker online, job instance status, job instance success ratio, alert status, and script governance failures.
- Resume with the next Phase 3 production gap that can be made locally verifiable without pulling Phase 4 scope back in.

## Remaining production follow-ups intentionally not marked complete
- Real OIDC token exchange/JWKS validation/user mapping/session issuance.
- Real HTTP and Worker Tunnel TLS/mTLS listeners and certificate reload/rotation.
- Full multi-level script approval state machine, verified signatures/KMS, production release gates, and URL/File/Secret grants.
- Real alert provider delivery for email/Slack/DingTalk/Feishu/WeCom/PagerDuty/webhooks.
- Full business SLO metrics/histograms beyond the current summary-backed snapshots, plus live Prometheus/Grafana recording-rule validation and real OTLP exporter collector smoke.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
