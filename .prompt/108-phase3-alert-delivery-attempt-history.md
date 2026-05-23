# 108 — Phase 3 alert delivery attempt history

## Goal
Persist alert notification delivery attempts so Phase 3 alerting has an auditable delivery history and retry-state foundation instead of transient in-memory dispatch results only.

## Scope
- Add an `alert_delivery_attempts` metadata table/entity without database foreign keys.
- Record one delivery attempt per firing alert event/provider/target when script-governance alert materialization dispatches channels.
- Store provider, redacted target, delivered flag, HTTP status code, error detail, attempt number, retry state, next retry timestamp, and created timestamp.
- Expose `GET /api/v1/alert-delivery-attempts` with filters for event, rule, provider, and retry state using the standard `{code,message,data}` envelope.
- Include the route and schema in OpenAPI.
- Preserve production-safe delivery policy: rejected insecure/loopback production URLs should still be captured as failed attempts with redacted targets.

## Out of scope
- Worker retry/backoff loop and DLQ processing.
- Email/SMTP delivery.
- Live external provider smoke tests.
