# Next Work

## Current pause point
- Paused after `.prompt/099-phase3-api-token-scopes.md`: API tokens now support optional fine-grained `resource:action` scopes that narrow effective permissions and prevent `admin` role bypass when scoped.
- Resume with the next Phase 3 production gap that can be made locally verifiable without pulling Phase 4 scope back in.

## Remaining production follow-ups intentionally not marked complete
- Real OIDC token exchange/JWKS validation/user mapping/session issuance.
- Real HTTP and Worker Tunnel TLS/mTLS listeners and certificate reload/rotation.
- Full multi-level script approval state machine, verified signatures/KMS, production release gates, and URL/File/Secret grants.
- Real alert provider delivery for email/Slack/DingTalk/Feishu/WeCom/PagerDuty/webhooks.
- Multi-tenant namespace/app/worker-pool scope binding and API token rotation/expiry policy.
- Full business SLO metrics/histograms beyond the current summary-backed snapshots, plus live Prometheus/Grafana recording-rule validation and real OTLP exporter collector smoke.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
