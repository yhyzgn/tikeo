---
title: Notification Center reference
description: Source-backed API, configuration, storage, event, redaction, retry, DLQ, and UI reference for Tikeo Notification Center.
keywords: [notification center reference, notification api, notification_delivery, notification policies, notification channels]
---

# Notification Center reference

This page is the source-backed reference for the generic Notification Center implementation. It complements the operator workflow in [Notifications](../user-guide/notifications) and the incident boundary in [Alerts](../user-guide/alerts).

Primary sources:

- Design and boundary: `design/notification-center-alerting-plan.md`
- Materialization and delivery: `crates/tikeo-server/src/notification.rs`
- HTTP routes and OpenAPI annotations: `crates/tikeo-server/src/http/routes/notifications.rs`
- Storage repositories and redaction: `crates/tikeo-storage/src/repository/notification.rs`
- Storage entities/migration: `crates/tikeo-storage/src/entities/notification_*.rs`, `crates/tikeo-storage/src/migration/notification_center.rs`
- Config defaults: `crates/tikeo-config/src/lib.rs`, `config/dev.toml`, `config/container.toml`
- Web UI: `web/src/pages/NotificationCenterPage.tsx`, `web/src/api/notifications.ts`

## Domain model

| Record | Table | Purpose |
| --- | --- | --- |
| Channel | `notification_channels` | Reusable outbound destination with scope, provider, redacted config, secret refs, and safety policy. |
| Policy/subscription | `notification_policies` | Owner/event filter that maps source events to ordered channel references and optional template refs. |
| Message | `notification_messages` | Normalized outbound message produced from a source event and policy. |
| Delivery attempt | `notification_delivery_attempts` | One provider attempt for one message/channel pair, with retry and DLQ state. |

The migration follows the repository convention of soft links instead of database-level foreign keys. IDs such as `policy_id`, `message_id`, and `channel_id` are checked explicitly by repository and service code.

## Configuration

Generic delivery worker settings are under `notification_delivery`.

```toml
[notification_delivery]
enabled = true
interval_seconds = 60
batch_size = 50
max_attempts = 3
backoff_seconds = 300
```

| Key | Default | Environment variable | Meaning |
| --- | --- | --- | --- |
| `notification_delivery.enabled` | `true` | `TIKEO__NOTIFICATION_DELIVERY__ENABLED` | Runs the generic delivery worker. |
| `notification_delivery.interval_seconds` | `60` | `TIKEO__NOTIFICATION_DELIVERY__INTERVAL_SECONDS` | Delay between due-attempt scans. |
| `notification_delivery.batch_size` | `50` | `TIKEO__NOTIFICATION_DELIVERY__BATCH_SIZE` | Maximum due attempts scanned per worker iteration. |
| `notification_delivery.max_attempts` | `3` | `TIKEO__NOTIFICATION_DELIVERY__MAX_ATTEMPTS` | Attempts before dead-lettering. |
| `notification_delivery.backoff_seconds` | `300` | `TIKEO__NOTIFICATION_DELIVERY__BACKOFF_SECONDS` | Retry delay for failed provider attempts. |

The manual retry endpoint accepts per-call overrides and clamps them to safe ranges: `limit <= 500`, `maxAttempts` between `1` and `20`, and `backoffSeconds` between `1` and `86400`.

## RBAC

The notification migration seeds these permissions:

| Permission | Used by |
| --- | --- |
| `notifications:read` | List channel types/channels/policies/messages/delivery attempts and queue status. |
| `notifications:manage` | Create, update, and delete channels and policies. |
| `notifications:test` | Run the retry-due delivery scan endpoint. |

The Web route `/notifications` also requires `notifications:read`.

## API envelope

All endpoints return the shared API envelope:

```json
{
  "code": 0,
  "message": "success",
  "data": {}
}
```

Examples in this reference use placeholders. Do not include real tokens, webhook URLs with secrets, SMTP passwords, PagerDuty routing keys, or authorization headers in documentation.

## Endpoint summary

| Method/path | Purpose | Permission |
| --- | --- | --- |
| `GET /api/v1/notification-channel-types` | Built-in provider metadata plus enabled plugin channel types. | `notifications:read` |
| `GET /api/v1/notification-channels` | List channels; supports scope/provider/enabled filters. | `notifications:read` |
| `POST /api/v1/notification-channels` | Create a channel. | `notifications:manage` |
| `GET /api/v1/notification-channels/{id}` | Read one redacted channel summary. | `notifications:read` |
| `PATCH /api/v1/notification-channels/{id}` | Update channel config/scope/provider/enabled/safety policy. | `notifications:manage` |
| `DELETE /api/v1/notification-channels/{id}` | Delete only when no policy references the channel. | `notifications:manage` |
| `GET /api/v1/notification-policies` | List policies with owner/event/enabled filters. | `notifications:read` |
| `POST /api/v1/notification-policies` | Create a policy. | `notifications:manage` |
| `GET /api/v1/notification-policies/{id}` | Read one policy. | `notifications:read` |
| `PATCH /api/v1/notification-policies/{id}` | Update policy fields. | `notifications:manage` |
| `DELETE /api/v1/notification-policies/{id}` | Delete a policy. | `notifications:manage` |
| `POST /api/v1/notification-policies/{id}:validate` | Validate channel references. | `notifications:read` |
| `GET /api/v1/notification-messages` | List normalized messages. | `notifications:read` |
| `GET /api/v1/notification-delivery-attempts` | List delivery attempts. | `notifications:read` |
| `GET /api/v1/notification-delivery-attempts:queue-status` | Count retry/DLQ state and return recent dead letters. | `notifications:read` |
| `POST /api/v1/notification-delivery-attempts:retry-due` | Process due attempts in a bounded scan. | `notifications:test` |

The current source does **not** expose a separate `POST /api/v1/notification-channels/{id}:test` endpoint. Channel type metadata now reports `supportsTestSend: false`; the implemented operator action is the generic retry-due scan.

## Channel request fields

`CreateNotificationChannelRequest` fields:

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `scopeType` | string | yes | `global`, `namespace`, `app`, or `worker_pool`. |
| `namespace` | string/null | no | Scope qualifier. |
| `app` | string/null | no | Scope qualifier. |
| `workerPool` | string/null | no | Scope qualifier. |
| `name` | string | yes | Must not be blank. Unique with scope columns by index. |
| `provider` | string | yes | Lowercase slug, built-in or enabled plugin type. |
| `enabled` | boolean | no | Defaults to `true`. Disabled channels do not deliver. |
| `config` | object | no | Provider config. API summaries redact sensitive keys and URL-like values. |
| `secretRefs` | object | no | Secret references. `secretRefsJson` is skipped in API serialization. |
| `safetyPolicy` | object/null | no | Optional local-smoke transport override. |

Provider validation:

- Webhook-style providers require `url`, `webhookUrl`, or `webhook_url`.
- PagerDuty requires `routingKey`, `routing_key`, `integrationKey`, or `integration_key`.
- Email requires `to` or `recipients`, plus SMTP URL/config through direct config or secret ref. Runtime accepts `secretRefs.password` as the metadata-aligned SMTP password reference alias, along with `passwordSecretRef` / `password_secret_ref`; SMTP URL reference aliases include `smtpUrl`, `smtp_url`, `url`, `smtpUrlSecretRef`, and `smtp_url_secret_ref`.
- Secret resolution is environment-backed in this implementation: `env:NAME` and bare `NAME` are read from process environment variables.

## Channel response and redaction

`NotificationChannelSummary` includes `configJson`, `targetRedacted`, `targetConfigured`, and `secretConfigured`. Redaction behavior is implemented in `crates/tikeo-storage/src/repository/notification.rs`:

- Keys containing `secret`, `token`, `password`, `authorization`, or equal to routing-key variants are replaced with `***redacted***`.
- URL-like config values are rendered as scheme, host, optional port, and `...` path.
- Values under `config.headers` are always redacted, including names such as `X-API-Key` that do not contain obvious secret words.
- `secret_refs_json` is marked `skip_serializing` and should not appear in API responses.
- `targetRedacted` is a cached display target; it should be used in UI/logs instead of raw provider config.

## Policy request fields

`CreateNotificationPolicyRequest` fields:

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `ownerType` | string | yes | API accepts `global`, `namespace`, `app`, `job`, `workflow`, `workflow_node`, `alert_rule`, or `worker_pool`; current runtime materialization only matches `global`, `namespace`, `app`, and `job` for `job_instance`. |
| `ownerId` | string/null | no | Soft-linked owner; `global` often omits it. |
| `name` | string | yes | Must not be blank. |
| `eventFamily` | string | yes | API accepts `job_instance`, `workflow`, `alert`, `worker`, or `script_governance`; current runtime materialization is implemented for `job_instance` only. |
| `eventFilter` | object | no | Job materializer supports `statuses` and `eventTypes`/`event_types`. |
| `channelRefs` | array | yes | Ordered channel refs. Empty list is rejected. |
| `templateRef` | string/null | no | Stored as a soft link; current materializer uses built-in rendering. |
| `severity` | string | yes | If blank in service materialization, default severity is derived from event. |
| `enabled` | boolean | no | Defaults to `true`. |
| `dedupeSeconds` | integer | no | Defaults to `300`. |

`PATCH` additionally accepts nullable `throttle`, `quietHours`, and `escalation` JSON fields, persisted as JSON strings. Current job-event materialization source only enforces event filtering and dedupe; future UI/runbooks should not claim full throttle/quiet-hours/escalation behavior until service code implements it.

## Message fields

`NotificationMessageSummary` contains:

- `sourceType` and `sourceId`, such as `job_instance` and the instance id.
- `policyId`.
- `eventType`, such as `job_instance.failed`.
- `resourceType` and `resourceId`, such as `job` and the job id.
- `severity`, `subject`, `body`, and provider-neutral `payloadJson`.
- `dedupeKey` and optional `traceId`.
- `status`, `createdAt`, and `updatedAt`.

The job materializer creates subjects like `Tikeo job <name>: <status-token>` and payload fields including `eventType`, `jobId`, `jobName`, `namespace`, `app`, `instanceId`, `status`, and `reason`.

## Delivery attempt fields

`NotificationDeliveryAttemptSummary` contains `messageId`, `policyId`, `channelId`, `provider`, `targetRedacted`, `attempt`, `delivered`, optional `statusCode`, optional `error`, `retryState`, optional `nextRetryAt`, and `createdAt`.

Delivery retry behavior:

1. Due attempts are loaded by retry state and due timestamp.
2. Attempts at or above `maxAttempts` are moved to `dead_letter`.
3. Missing message, missing channel, or disabled channel also dead-letters the attempt.
4. The provider call runs while the current attempt remains retryable, so a crash before or during delivery does not remove the only pending row.
5. After a provider result is recorded, a new delivered/retry/DLQ row is persisted and the previous due attempt is marked `retry_consumed`.
6. Success marks the message delivered. Failure appends a new `retry_pending` attempt or dead-letters when exhausted.

## Job event contract

Implemented job event names and default severities:

| Event type | Filter status | Default severity |
| --- | --- | --- |
| `job_instance.retry_scheduled` | `retry_scheduled` | `warning` |
| `job_instance.retry_exhausted` | `retry_exhausted` | `critical` |
| `job_instance.succeeded` | `succeeded` | `info` |
| `job_instance.failed` | `failed` | `critical` |
| `job_instance.partial_failed` | `partial_failed` | `critical` |
| `job_instance.cancelled` | `cancelled` | `warning` |
| `job_instance.no_eligible_worker` | `no_eligible_worker` | `critical` |
| `job_instance.script_governance_failure` | `script_governance_failure` | `critical` |

`JobNotificationEvent::from_terminal_status()` maps only terminal instance states: `succeeded`, `failed`, `partial_failed`, and `cancelled`. Pending, dispatching, and running do not emit terminal notifications.

Worker-result failure semantics are retry-aware: a failure that schedules another attempt emits `job_instance.retry_scheduled`; a non-retrying terminal failure emits `job_instance.failed`; `job_instance.retry_exhausted` is emitted only after an enabled retry policy with at least one retry has exhausted its configured attempts.

## Provider delivery behavior

The generic delivery worker reuses provider adapter shapes from alert delivery:

| Provider | Delivered payload behavior |
| --- | --- |
| `webhook` | POST provider-neutral JSON payload. |
| `slack` | POST `{text}` containing a compact notification summary. |
| `dingtalk` | POST text message body. |
| `feishu` | POST Feishu/Lark text message body. |
| `wechat_work` | POST WeCom text message body. |
| `pagerduty` | POST Events API v2 trigger payload; default URL is `https://events.pagerduty.com/v2/enqueue` when not configured. |
| `email` | Uses the email branch of `AlertDispatcher` with the normalized message converted to an alert-like payload. |
| plugin webhook | POST provider-neutral JSON payload with configured headers. |

URL safety uses `alert::validate_webhook_url()`. Production targets should be HTTPS and publicly routable; `safetyPolicy.allowInsecureLoopback` is only for explicit local smoke tests.

## UI reference

`web/src/pages/NotificationCenterPage.tsx` loads these endpoints in parallel:

- `GET /api/v1/notification-channel-types`
- `GET /api/v1/notification-channels`
- `GET /api/v1/notification-policies`
- `GET /api/v1/notification-messages`
- `GET /api/v1/notification-delivery-attempts:queue-status`

The page renders statistics for channels, policies, retry-pending attempts, and DLQ count. Tabs show channel summaries, policy summaries, queue/DLQ state with a **Retry due** action, and the latest 20 messages. Operators with `notifications:manage` can create, edit, and delete channels/policies; all policy validation is backed by `POST /api/v1/notification-policies/{id}:validate`.

## Static examples that are safe to copy

List provider metadata:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/notification-channel-types \
  -H 'Authorization: Bearer <operator-token>'
```

List queue status:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/notification-delivery-attempts:queue-status \
  -H 'Authorization: Bearer <operator-token>'
```

List failed messages:

```bash
curl -fsS 'http://127.0.0.1:9090/api/v1/notification-messages?status=failed' \
  -H 'Authorization: Bearer <operator-token>'
```

These commands require a running authenticated server, so docs verification checks the command syntax and source paths; end-to-end execution belongs in a server smoke test with test credentials.

## Alert boundary

Alerting should become a producer of notification messages, not the owner of reusable provider credentials. Until that migration is complete, keep these rules:

- Alert rules remain the abnormal-condition evaluator.
- Notification Center owns new reusable outbound destinations.
- `alert_rules.channels_json` is compatibility behavior, not the preferred place to duplicate provider secrets.
- Normal job success/failure/cancel messages belong to notification policies.
- Incident states such as `firing`, `suppressed`, `silenced`, and `recovered` belong to Alerts.
