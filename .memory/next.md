# Next Work

## Phase 3 closeout status
- Phase 3 local foundations are closed out through `.prompt/095-phase3-closeout-review.md`.
- Final verification for the closeout pass should remain the standard Rust gate unless future Web files change.

## Remaining production follow-ups intentionally not marked complete
- Real OIDC token exchange/JWKS validation/user mapping/session issuance.
- Real HTTP and Worker Tunnel TLS/mTLS listeners and certificate reload/rotation.
- Full multi-level script approval state machine, verified signatures/KMS, production release gates, and URL/File/Secret grants.
- Real alert provider delivery for email/Slack/DingTalk/Feishu/WeCom/PagerDuty/webhooks.
- Full business SLO metrics/histograms and real OTLP exporter collector smoke.

## Deferred out of Phase 3
- Node.js SDK, K8s Helm Chart, PowerJob migration tooling, and XXL-JOB migration tooling belong to Phase 4.
