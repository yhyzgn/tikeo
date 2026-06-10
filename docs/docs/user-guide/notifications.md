---
title: Notifications user guide
description: Operator workflow for Tikeo Notification Center channels, policies, messages, delivery retry, DLQ, UI inspection, and alerting boundaries.
keywords: [notification center, outbound notifications, webhook, slack, pagerduty, retry, dlq]
---

# Notifications user guide

Notification Center is the reusable outbound delivery layer in Tikeo. Use it when you need to send lifecycle or operational messages to Slack, DingTalk, Feishu/Lark, WeChat Work, PagerDuty, email, generic webhooks, or plugin-provided webhook-compatible providers.

The source-backed boundary is important:

- **Notification Center** owns reusable outbound channels, policies/subscriptions, normalized messages, delivery attempts, retry, and DLQ state. Current source: `crates/tikeo-server/src/notification.rs`, `crates/tikeo-server/src/http/routes/notifications.rs`, `crates/tikeo-storage/src/repository/notification.rs`, and `web/src/pages/NotificationCenterPage.tsx`.
- **Alerts** own abnormal-condition rules, alert events, incident-like states, silence/recovery/suppression semantics, and the compatibility alert delivery ledger. See [Alerts](./alerts) before using an alert rule for a normal job-completion message.

## When to use notifications

Use Notification Center for outbound messages that are not necessarily incidents:

| Use case | Recommended event family | Example event types |
| --- | --- | --- |
| A job succeeds and a team wants a confirmation message | `job_instance` | `job_instance.succeeded` |
| A job reaches terminal failure or retry exhaustion | `job_instance` | `job_instance.failed`, `job_instance.retry_exhausted` |
| Broadcast work partially fails | `job_instance` | `job_instance.partial_failed` |
| Dispatch cannot find a matching worker | `job_instance` | `job_instance.no_eligible_worker` |
| Script governance blocks execution | `job_instance` or `script_governance` | `job_instance.script_governance_failure` |
| Alerting produces an incident event | `alert` | `alert.firing`, `alert.recovered` are accepted policy-family concepts but not yet materialized by Notification Center. |

Use Alerts instead when you need condition evaluation, dedupe/silence/recovery behavior, abnormal-condition history, or incident review.

## Provider types

The implemented built-in channel types come from `builtin_channel_types()` in `crates/tikeo-server/src/http/routes/notifications.rs`.

| Provider | Required config keys | Secret config keys | Notes |
| --- | --- | --- | --- |
| `webhook` | `url` | `authorization` | Generic JSON POST target. |
| `slack` | `url` | none in metadata | Slack incoming webhook-style target. |
| `dingtalk` | `url` | `signingKey` | DingTalk robot webhook. |
| `feishu` | `url` | `signingKey` | Feishu/Lark bot webhook. |
| `wechat_work` | `url` | none in metadata | WeCom/WeChat Work robot webhook. |
| `pagerduty` | `routingKey` | `routingKey` | PagerDuty Events v2 integration. |
| `email` | `smtpUrl`, `to` | `password`, `smtpUrl` | SMTP delivery through the shared provider adapter. |
| plugin type | usually `url` | plugin-defined | Plugin alert channel metadata is accepted as a compatibility alias for notification providers. |

Webhook-style providers accept `url`, `webhookUrl`, or `webhook_url`. PagerDuty accepts `routingKey`, `routing_key`, `integrationKey`, or `integration_key`. Email accepts `to` or `recipients`; its SMTP endpoint can come from `config.smtpUrl`, `config.smtp_url`, `config.url`, `secretRefs.smtpUrl`, `secretRefs.smtp_url`, `secretRefs.url`, `config.smtpUrlSecretRef`, `config.smtp_url_secret_ref`, `secretRefs.smtpUrlSecretRef`, or `secretRefs.smtp_url_secret_ref`. SMTP auth passwords use `config.passwordSecretRef`, `config.password_secret_ref`, `secretRefs.password`, `secretRefs.passwordSecretRef`, or `secretRefs.password_secret_ref`.

Runtime secret resolution for Notification Center currently resolves `env:` references or bare environment variable names through the process environment. Do not enter raw secret values in `config` or `secretRefs`.

## Setup flow

1. **Check access.** The route metadata in `web/src/routes.tsx` exposes `/notifications` to users with `notifications:read`. Creating/updating channels and policies requires `notifications:manage`. Retrying due delivery attempts requires `notifications:test`.
2. **Create a channel.** A channel is a reusable outbound destination. Scope it as `global`, `namespace`, `app`, or `worker_pool`.
3. **Create a policy.** A policy binds an owner, event family, event filter, severity, dedupe window, and ordered channel references.
4. **Validate the policy.** Validation checks that channel references exist and are enabled.
5. **Trigger or wait for source events.** Implemented job lifecycle events materialize messages through `NotificationCenter::emit_job_instance_event()`.
6. **Inspect messages and delivery attempts.** The UI shows recent messages and queue state; API endpoints expose filters.
7. **Operate retry/DLQ.** Let the background worker scan due attempts, or use the retry-due endpoint for operator-driven retry scans.

## Safe channel creation example

The API uses the shared `{code,message,data}` envelope. The examples below intentionally use placeholder URLs and secret references. Do not paste real webhook tokens, SMTP passwords, routing keys, or authorization headers into docs, screenshots, tickets, or chat.

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-channels \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "scopeType": "app",
    "namespace": "prod",
    "app": "billing",
    "name": "billing-ops-webhook",
    "provider": "webhook",
    "enabled": true,
    "config": {
      "url": "https://hooks.example.invalid/tikeo/billing"
    },
    "secretRefs": {
      "authorization": "env:TIKEO_NOTIFICATION_WEBHOOK_AUTH"
    }
  }'
```

Expected response shape:

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "id": "notification-channel-example",
    "scopeType": "app",
    "provider": "webhook",
    "targetRedacted": "https://hooks.example.invalid/...",
    "targetConfigured": true,
    "secretConfigured": true
  }
}
```

The exact `id` is generated by storage. `secretRefsJson` is skipped during serialization, and `configJson` is redacted by `NotificationChannelSummary::from()` in `crates/tikeo-storage/src/repository/notification.rs`.

## Safe policy creation example

`channelRefs` may contain strings or objects with `channelId`, `channel_id`, or `id`; both the materializer and policy validator extract those forms.

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-policies \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "ownerType": "app",
    "ownerId": "prod/billing",
    "name": "billing terminal failures",
    "eventFamily": "job_instance",
    "eventFilter": {
      "eventTypes": ["job_instance.failed", "job_instance.retry_exhausted"],
      "statuses": ["failed", "retry_exhausted"]
    },
    "channelRefs": [
      {"channelId": "notification-channel-example"}
    ],
    "templateRef": null,
    "severity": "critical",
    "enabled": true,
    "dedupeSeconds": 300
  }'
```

Validate after creation:

```bash
curl -fsS -X POST \
  http://127.0.0.1:9090/api/v1/notification-policies/notification-policy-example:validate \
  -H 'Authorization: Bearer <operator-token>'
```

Validation returns `valid`, `channelCount`, `missingChannelIds`, `disabledChannelIds`, and `issues`.

## Owner and event semantics

The API currently accepts owner types `global`, `namespace`, `app`, `job`, `workflow`, `workflow_node`, `alert_rule`, and `worker_pool`, and event families `job_instance`, `workflow`, `alert`, `worker`, and `script_governance`. Current runtime materialization is implemented for `job_instance` policies only, matching `global`, `namespace`, `app`, and `job` owners.

For job-instance notifications, the materializer currently matches:

- `global` policies for all jobs.
- `namespace` policies when `ownerId` equals the job namespace.
- `app` policies when `ownerId` equals either the app name or `namespace/app`.
- `job` policies when `ownerId` equals the job id.

The filter checks `eventFilter.statuses` against the stable status token and `eventFilter.eventTypes` or `eventFilter.event_types` against the full event type. Empty `statuses` or `eventTypes` arrays mean that dimension is not restricted.

## Implemented job event types

These stable event names are implemented in `JobNotificationEvent`:

| Event type | Default severity | Meaning |
| --- | --- | --- |
| `job_instance.retry_scheduled` | `warning` | A failed attempt scheduled another retry. |
| `job_instance.retry_exhausted` | `critical` | Attempts are exhausted. |
| `job_instance.succeeded` | `info` | Instance reached terminal success. |
| `job_instance.failed` | `critical` | Instance reached terminal failure. |
| `job_instance.partial_failed` | `critical` | Broadcast completed with at least one failed child. |
| `job_instance.cancelled` | `warning` | User or system cancelled the instance. |
| `job_instance.no_eligible_worker` | `critical` | Dispatcher could not find an eligible worker. |
| `job_instance.script_governance_failure` | `critical` | Script governance failure materialized. |

Do not treat every failed attempt as `job_instance.failed` if a retry was scheduled. `retry_scheduled` is the noise-control event; terminal failure uses `failed` or `retry_exhausted`.

## Queue, retry, and DLQ

Generic delivery attempts are stored in `notification_delivery_attempts`. Current runtime-created attempt retry states are `retry_pending`, `retry_consumed`, `delivered`, and `dead_letter`; queue status reports unknown or legacy states in the `failed` bucket. Current runtime-created message statuses are `pending`, `delivered`, and `dead_letter`; the storage field is string-based and reserves additional future statuses.

The generic delivery worker defaults come from `notification_delivery` in `crates/tikeo-config/src/lib.rs` and the committed `config/dev.toml` and `config/container.toml` examples:

| Key | Default |
| --- | --- |
| `notification_delivery.enabled` | `true` |
| `notification_delivery.interval_seconds` | `60` |
| `notification_delivery.batch_size` | `50` |
| `notification_delivery.max_attempts` | `3` |
| `notification_delivery.backoff_seconds` | `300` |

Operator retry scan:

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-delivery-attempts:retry-due \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{"limit":50,"maxAttempts":3,"backoffSeconds":300}'
```

The handler clamps `limit` to at most `500`, `maxAttempts` to `1..20`, and `backoffSeconds` to `1..86400`.

## UI workflow

Open **Notification Center / 通知中心** at `/notifications`. The console page is backed by `web/src/pages/NotificationCenterPage.tsx` and `web/src/api/notifications.ts`; it now supports redacted channel/policy inspection plus governed channel and policy create/edit/delete/validate operations.

| Tab | What to check |
| --- | --- |
| Channels | Channel name, provider, scope, redacted target, whether a secret is configured, enabled state, create/edit/delete drawers, and backend conflict handling for referenced channels. |
| Policies | Owner, event family, severity, dedupe seconds, enabled state, create/edit/delete drawers, channel multi-select, JSON event filters, and policy validation. |
| Delivery | Total attempts, delivered count, retry-pending count, retry-consumed count, DLQ count, failed count, recent DLQ rows, and **Retry due** action. |
| Messages | Recent normalized messages, event type, resource, subject, status, and creation time. |

Use the UI for common channel and policy CRUD/validation, and keep using the Management API for automation, bulk changes, or fields not yet optimized by the form UX.

## Troubleshooting runbook

| Symptom | Check | Likely fix |
| --- | --- | --- |
| `/notifications` is hidden | Route permission requires `notifications:read` in `web/src/routes.tsx`. | Grant read permission or use an Owner/Operator role with notification permissions from the migration seed. |
| Channel create fails with provider error | `validate_channel_request()` only accepts built-ins or enabled plugin-provided types. | Use `GET /api/v1/notification-channel-types` and correct `provider`. |
| Channel create fails with missing target | Webhook-style providers require `url`/`webhookUrl`; PagerDuty requires routing/integration key; email requires recipients and SMTP config. | Add a non-secret target plus secret refs where needed. |
| Delete channel returns conflict | `delete_channel()` refuses channels referenced by policies. | Disable/update/delete referencing policies first. |
| Policy validation reports missing/disabled channels | `validate_policy()` checks `channelRefs`. | Correct IDs or enable required channels. |
| Attempts stay `retry_pending` | Check `notification_delivery.enabled`, scan interval, `nextRetryAt`, and queue status. | Run the retry-due endpoint for a bounded scan; verify background worker config. |
| Attempts move to `dead_letter` | Max attempts exhausted, source message missing, channel missing, or channel disabled. | Fix channel/message context, create a new message/event, or update policy/channel before retrying. |
| Webhook URL is rejected | Delivery uses the alert provider URL safety policy. | Use HTTPS/public targets in production; use `safetyPolicy.allowInsecureLoopback` only for explicit local smoke tests. |
| Secrets appear in output | This should not happen for API responses; summaries redact config and skip `secretRefsJson`. | Stop sharing the output and file a security bug with the source response and route. |

## Alert boundary checklist

Before creating a notification policy, ask:

- Is this a normal lifecycle message? Use Notification Center.
- Is this an abnormal condition that needs incident semantics? Use Alerts, then let Alerting produce notification messages as the migration path matures.
- Does the destination need to be reused across jobs, alerts, and workflows? Put it in Notification Center as a channel, not inline in an alert rule.
- Does the message contain credentials or tokens? Put them in secret references; never show them in examples or UI captures.
