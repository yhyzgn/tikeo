# Next Work

## Current pause point
- Paused after `.prompt/096-phase3-dispatch-queue-prometheus-metric.md`: the dispatch queue pending-age Grafana SLO metric now has a real Prometheus histogram emitted by the server and verified locally.
- User requested stopping after this in-progress item is completed. Resume only when the user asks for the next Phase 3/Phase 4 slice.

## Remaining production follow-ups intentionally not marked complete
- Real OIDC token exchange/JWKS validation/user mapping/session issuance.
- Real HTTP and Worker Tunnel TLS/mTLS listeners and certificate reload/rotation.
- Full multi-level script approval state machine, verified signatures/KMS, production release gates, and URL/File/Secret grants.
- Real alert provider delivery for email/Slack/DingTalk/Feishu/WeCom/PagerDuty/webhooks.
- Full business SLO metrics/histograms beyond the dispatch queue pending-age foundation, plus real OTLP exporter collector smoke.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
