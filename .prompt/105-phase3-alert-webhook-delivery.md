# 105 — Phase 3 alert webhook delivery

## Goal
Close the locally verifiable part of the Phase 3 real alert-provider gap by making webhook channels perform an actual HTTP POST while preserving production-safe defaults.

## Scope
- Add deterministic alert delivery results so callers/tests can inspect provider, redacted target, status, and error.
- Keep default webhook policy HTTPS/public-only.
- Allow loopback HTTP only through an explicit test/local delivery policy.
- Materialize script-governance firing alert events into notification-channel delivery attempts.

## Out of scope
- Email/Slack/DingTalk/Feishu/WeCom/PagerDuty production adapters.
- Provider retry queues, backoff, DLQ, and persisted delivery attempt history.
- Phase 4 migration/Helm/Node SDK work.

## Verification notes
- Targeted tests cover default production rejection of insecure loopback and opt-in local loopback webhook POST delivery.
- Existing alert API/event tests remain no-network because their materialization fixtures use empty channels unless testing readiness parsing.
