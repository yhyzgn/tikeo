# 107 — Phase 3 alert non-webhook provider delivery

## Goal
Move Phase 3 alerting beyond generic webhooks by adding locally verifiable provider-specific delivery adapters for common operations channels.

## Scope
- Support Slack, DingTalk, Feishu/Lark, WeChat Work/WeCom, and PagerDuty notification channel variants.
- Reuse the production-safe URL policy from webhook delivery: HTTPS/public targets by default, loopback HTTP only under explicit local test policy.
- Emit provider-specific JSON payload shapes while returning structured, redacted delivery results.

## Out of scope
- Email SMTP delivery.
- Retry queues, backoff, DLQ, and persisted delivery attempt history.
- Live external provider integration tests.
