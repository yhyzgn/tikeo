# 111 — Phase 3 alert retry/DLQ foundation

## Goal
Add a locally verifiable retry/backoff/dead-letter foundation for persisted alert delivery attempts.

## Scope
- List due `retry_pending` alert delivery attempts by `next_retry_at`.
- Mark consumed retry rows and append a new attempt result when retrying a matching current notification channel.
- Move exhausted, missing source event/rule, invalid channels, or missing matching channel attempts to `dead_letter`.
- Provide a bounded `POST /api/v1/alert-delivery-attempts:retry-due` management route returning scanned/retried/dead_lettered/skipped counts.
- Keep production delivery policy safe by default; tests may use explicit local loopback policy.

## Out of scope
- Continuous background retry worker scheduling.
- External provider live smoke tests.
- Production SMTP TLS/auth and secret-backed credentials.
