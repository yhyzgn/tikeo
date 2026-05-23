# 110 — Phase 3 email SMTP delivery foundation

## Goal
Move email notification channels from explicitly unsupported to a locally verifiable SMTP delivery foundation.

## Scope
- Extend `NotificationChannel::Email` with recipients, optional `smtp_url`, and optional envelope sender.
- Accept legacy/simple `to` and `url` aliases for email channel JSON.
- Deliver plain SMTP messages to `smtp://` loopback endpoints only when the explicit local delivery policy allows insecure loopback.
- Keep production-safe behavior by failing closed without `smtp_url`, recipients, or local-loopback policy.
- Update alert delivery readiness so email requires both recipients and an SMTP endpoint.

## Out of scope
- Production SMTP TLS/auth, provider-specific email services, and secret-backed credentials.
- Retry/backoff/DLQ processing.
- Live external SMTP provider smoke tests.
