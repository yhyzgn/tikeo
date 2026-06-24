---
title: Notification Center reference
description: Operator-verified API, configuration, storage, event, redaction, retry, DLQ, and UI reference for Tikeo Notification Center.
keywords: [notification center reference, notification api, notification_delivery, notification policies, notification channels]
---

# Notification Center reference

This page is the operator reference for the generic Notification Center implementation. It complements the operator workflow in [Notifications](../user-guide/notifications) and the incident boundary in [Alerts](../user-guide/alerts).

Primary sources:

- Design and boundary: `design/notification-center-alerting-plan.md`
- Materialization and delivery: `crates/tikeo-server/src/notification.rs`
- HTTP routes and OpenAPI annotations: `crates/tikeo-server/src/http/routes/notifications.rs`, `crates/tikeo-server/src/http/routes/notification_templates.rs`
- Storage repositories and redaction: `crates/tikeo-storage/src/repository/notification.rs`, `crates/tikeo-storage/src/repository/notification_template.rs`
- Storage entities/migration: `crates/tikeo-storage/src/entities/notification_*.rs`, `crates/tikeo-storage/src/migration/notification_center.rs`
- Config defaults: `crates/tikeo-config/src/lib.rs`, `config/dev.toml`, `config/tikeo.yml`
- Web UI: `web/src/pages/NotificationCenterPage.tsx`, `web/src/api/notifications.ts`

## Domain model

| Record | Table | Purpose |
| --- | --- | --- |
| Channel | `notification_channels` | Reusable outbound destination with scope, provider, redacted config, secret refs, and safety policy. |
| Policy/subscription | `notification_policies` | Owner/event filter that maps source events to ordered channel references and optional template refs. |
| Template | `notification_templates` | Reusable provider/message-type template body with safe variable rendering and preview. |
| Message | `notification_messages` | Normalized outbound message produced from a source event and policy. |
| Delivery attempt | `notification_delivery_attempts` | One provider attempt for one message/channel pair, with retry and DLQ state. |

The migration follows the repository convention of soft links instead of database-level foreign keys. IDs such as `policy_id`, `message_id`, and `channel_id` are checked explicitly by repository and service code.

## Configuration

Generic delivery worker settings are under `notification_delivery`.

```toml
[notification_delivery]
enabled = true
# Optional. Set to the externally reachable Web base URL for notification card buttons.
# public_console_base_url = "https://tikeo.example.com"
interval_seconds = 60
batch_size = 50
max_attempts = 3
backoff_seconds = 300
```

| Key | Default | Environment variable | Meaning |
| --- | --- | --- | --- |
| `notification_delivery.enabled` | `true` | `TIKEO__NOTIFICATION_DELIVERY__ENABLED` | Runs the generic delivery worker. |
| `notification_delivery.public_console_base_url` | unset | `TIKEO__NOTIFICATION_DELIVERY__PUBLIC_CONSOLE_BASE_URL` | Optional externally reachable Web base URL used to turn `/public/instances/{id}/console` into an absolute Feishu/Lark card button URL. |
| `notification_delivery.interval_seconds` | `60` | `TIKEO__NOTIFICATION_DELIVERY__INTERVAL_SECONDS` | Recovery scan interval for due attempts. New attempts wake the local delivery worker immediately; this scan handles missed signals, restarts, cross-process handoff, and retries. |
| `notification_delivery.batch_size` | `50` | `TIKEO__NOTIFICATION_DELIVERY__BATCH_SIZE` | Maximum due attempts scanned per worker iteration. |
| `notification_delivery.max_attempts` | `3` | `TIKEO__NOTIFICATION_DELIVERY__MAX_ATTEMPTS` | Attempts before dead-lettering. |
| `notification_delivery.backoff_seconds` | `300` | `TIKEO__NOTIFICATION_DELIVERY__BACKOFF_SECONDS` | Retry delay for failed provider attempts. |

Delivery is event-driven first and scan-driven as a fallback. Job, workflow, and alert notification materialization writes normalized `notification_messages` plus due `notification_delivery_attempts`, then signals the local delivery worker immediately. The periodic `interval_seconds` scan is still retained for resilience: it catches missed in-process signals, process restarts, cross-process/HA handoff, and normal retry backoff windows. The manual retry endpoint accepts per-call overrides and clamps them to safe ranges: `limit <= 500`, `maxAttempts` between `1` and `20`, and `backoffSeconds` between `1` and `86400`.

## RBAC

The notification migration seeds these permissions:

| Permission | Used by |
| --- | --- |
| `notifications:read` | List channel types/channels/policies/messages/delivery attempts and queue status. |
| `notifications:manage` | Create, update, and delete channels and policies. |
| `notifications:test` | Send a saved channel test notification and run the retry-due delivery scan endpoint. |

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
| `POST /api/v1/notification-channels/{id}/test-send` | Send one test notification through a saved enabled built-in channel and return redacted delivery evidence. | `notifications:test` |
| `GET /api/v1/notification-policies` | List policies with owner/event/enabled filters. | `notifications:read` |
| `POST /api/v1/notification-policies` | Create a policy. | `notifications:manage` |
| `GET /api/v1/notification-policies/{id}` | Read one policy. | `notifications:read` |
| `PATCH /api/v1/notification-policies/{id}` | Update policy fields. | `notifications:manage` |
| `DELETE /api/v1/notification-policies/{id}` | Delete a policy. | `notifications:manage` |
| `POST /api/v1/notification-policies/{id}:validate` | Validate channel references. | `notifications:read` |
| `GET /api/v1/notification-templates` | List reusable templates with provider/message-type/enabled filters. | `notifications:read` |
| `POST /api/v1/notification-templates` | Create a reusable provider-specific template. | `notifications:manage` |
| `GET /api/v1/notification-templates/{id-or-key}` | Read one template by id or `templateKey`. | `notifications:read` |
| `PATCH /api/v1/notification-templates/{id}` | Update template metadata, body, variables, or enabled state. | `notifications:manage` |
| `DELETE /api/v1/notification-templates/{id}` | Delete a template row. Policies are soft-linked and should be updated first. | `notifications:manage` |
| `POST /api/v1/notification-templates/{id}/render` | Render a stored or unsaved draft template against sample JSON without provider delivery. The `{id}` value may be a generated id, `templateKey`, or draft key when the request body supplies `provider`, `messageType`, and `template`. | `notifications:read` |
| `GET /api/v1/jobs/{job}/notification-bindings` | List job-facing notification bindings backed by job-owned `notification_policies`. | `jobs:read` + `notifications:read` |
| `POST /api/v1/jobs/{job}/notification-bindings` | Create a job notification binding for success/failure/always/cancel/retry presets or advanced event lists. | `jobs:write` + `notifications:manage` |
| `GET/PATCH/DELETE /api/v1/jobs/{job}/notification-bindings/{binding}` | Read, update, or delete one job-owned notification binding; wrong-owner bindings return 404. | `jobs:read/write` + `notifications:read/manage` |
| `POST /api/v1/jobs/{job}/notification-bindings:validate` | Validate selected channels, template provider compatibility, and expanded job event types. | `jobs:read` + `notifications:read` |
| `POST /api/v1/jobs/{job}/notification-bindings:preview` | Render a sample job-instance payload against the selected template without delivery. | `jobs:read` + `notifications:read` |
| `GET /api/v1/notification-messages` | List normalized messages. | `notifications:read` |
| `GET /api/v1/notification-messages/{id}/trace` | Return one message with policy, delivery attempts, job/instance context, and a redacted execution log excerpt. | `notifications:read` plus tenant scope check when a job can be resolved |
| `GET /api/v1/notification-delivery-attempts` | List delivery attempts. | `notifications:read` |
| `GET /api/v1/notification-delivery-attempts:queue-status` | Count retry/DLQ state and return recent dead letters. | `notifications:read` |
| `POST /api/v1/notification-delivery-attempts:retry-due` | Process due attempts in a bounded scan. | `notifications:test` |

Built-in channel type metadata reports `supportsTestSend=true`. Use `POST /api/v1/notification-channels/{id}/test-send`, the list-row **Test** action, or the edit drawer **Test** action to exercise one saved enabled channel; use `POST /api/v1/notification-delivery-attempts:retry-due` for the generic due-attempt worker scan.

## Job notification bindings

The job drawer in `web/src/pages/notifications/JobNotificationConfigDrawer.tsx` is a job-facing configuration layer over `notification_policies`; it does not create a second delivery system. A binding stores:

| Field | Meaning |
| --- | --- |
| `trigger` | Preset: `success`, `failure`, `always`, `cancelled`, `retry_scheduled`, `retry_exhausted`, or `advanced`. |
| `eventTypes` | Advanced explicit events, validated against supported `job_instance.*` event names. Presets expand server-side. |
| `channelIds` | Existing enabled notification channels. At least one is required. |
| `templateRef` | Optional enabled `notification_templates` id or `templateKey`; provider must match at least one selected channel provider. |
| `includeLogLink` / `includeLogExcerpt` / `logExcerptLines` | Controls whether job messages include log navigation metadata and how much log context the operator expects to inspect. |
| `dedupeSeconds` | Reuses Notification Center dedupe; default is `300`. |

Runtime materialization still flows through `NotificationCenter::emit_job_instance_event`: matching job-owned policies create normalized `notification_messages` and delivery attempts. Message payloads include flat and nested context keys such as `jobId`, `jobName`, `namespace`, `app`, `instanceId`, `status`, `triggerType`, `executionMode`, `startedAt`, `finishedAt`, `workerId`, `operatorName`, `operatorType`, `reason`, `logsUrl`, and `consoleUrl`.

Supported template variables for job messages include the generic variables plus `{{jobId}}`, `{{jobName}}`, `{{namespace}}`, `{{app}}`, `{{instanceId}}`, `{{status}}`, `{{triggerType}}`, `{{executionMode}}`, `{{startedAt}}`, `{{finishedAt}}`, `{{workerId}}`, `{{operatorName}}`, `{{operatorType}}`, `{{reason}}`, `{{logsUrl}}`, and `{{consoleUrl}}`.

## Message trace and execution-log passthrough

Use `GET /api/v1/notification-messages/{id}/trace` or the **Details** action in the Notification Center messages tab when an operator needs to answer “which job instance produced this outbound message and what happened during execution?”. The response contains:

- `message`: normalized Notification Center message.
- `policy`: source policy when still present.
- `attempts`: provider attempts with redacted targets, HTTP status, retry state, and errors.
- `job` / `instance`: resolved execution context when the message is job-related.
- `logs`: `/instances/{instance}/logs` URL plus the latest 80 log lines; sensitive fragments containing password/token/secret/authorization/routingKey/signingKey key-value patterns are redacted for display.

Trace is read-only. It never calls external providers and never reveals stored channel `secretRefs`.

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
| `secretRefs` | object | no | Secret references owned by this channel row. `secretRefsJson` is skipped in API serialization. |
| `safetyPolicy` | object/null | no | Optional local-smoke transport override. |

Provider validation:

- Webhook-style providers require `url`, `webhookUrl`, or `webhook_url`.
- PagerDuty requires `routingKey`, `routing_key`, `integrationKey`, or `integration_key`.
- Email requires `to` or `recipients`, plus SMTP URL/config through direct config or secret ref. Runtime accepts `secretRefs.password` as the metadata-aligned SMTP password reference alias, along with `passwordSecretRef` / `password_secret_ref`; SMTP URL reference aliases include `smtpUrl`, `smtp_url`, `url`, `smtpUrlSecretRef`, and `smtp_url_secret_ref`.
- Secret resolution supports direct values configured in the drawer, which are stored server-side and take effect immediately without restarting the service. For backward/deployment compatibility, `env:NAME` and bare `NAME` can also be read from the Server process environment.

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
| `ownerType` | string | yes | API accepts `global`, `namespace`, `app`, `job`, `workflow`, `workflow_node`, `alert_rule`, or `worker_pool`; runtime materialization currently matches `global`/`namespace`/`app`/`job` for `job_instance`, `global`/`alert_rule` for `alert`, and `global`/`workflow`/`workflow_node` for workflow notification nodes. |
| `ownerId` | string/null | no | Soft-linked owner; `global` often omits it. |
| `name` | string | yes | Must not be blank. |
| `eventFamily` | string | yes | API accepts `job_instance`, `workflow`, `alert`, `worker`, or `script_governance`; runtime materialization is implemented for job instance events, alert events, and workflow notification-node requests. |
| `eventFilter` | object | no | Job materializer supports `statuses` and `eventTypes`/`event_types`; workflow notification-node materializer also supports `workflowIds` and `nodeKeys`. |
| `channelRefs` | array | yes | Ordered channel refs. Empty list is rejected. |
| `templateRef` | string/null | no | Soft link to `notification_templates.id` or `templateKey`. Enabled stored templates are loaded during `job_instance` materialization and can override subject/body plus `payload.template`. Missing/disabled refs are ignored for compatibility. |
| `severity` | string | yes | If blank in service materialization, default severity is derived from event. |
| `enabled` | boolean | no | Defaults to `true`. |
| `dedupeSeconds` | integer | no | Defaults to `300`. |

`PATCH` additionally accepts nullable `throttle`, `quietHours`, and `escalation` JSON fields, persisted as JSON strings. Current job-event materialization source only enforces event filtering and dedupe; future UI/runbooks should not claim full throttle/quiet-hours/escalation behavior until service code implements it.

## Template fields and rendering

`notification_templates` stores reusable provider-specific template bodies. The API shape is:

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `templateKey` | string | yes | Stable operator key; letters, numbers, dot, underscore, and dash only; maximum 128 bytes. |
| `name` | string | yes | Operator-facing name. |
| `description` | string/null | no | Optional description. |
| `provider` | string | yes | Built-in or plugin provider slug. |
| `messageType` | string | yes | Must be supported by the provider schema when provider is built-in. |
| `enabled` | boolean | no | Disabled templates remain stored but are skipped during runtime materialization. |
| `body` | object | no | Provider-specific message template body. Required fields are validated from provider metadata. |
| `variables` | object | no | Documentation/default-variable metadata; not secret storage. |

The render dry-run endpoint uses the same variable replacement engine as delivery payload rendering and returns only the rendered JSON body; it does not resolve channel secret refs or call external providers. Stored templates can be rendered by id or `templateKey`. Unsaved drafts are also supported when the request body supplies `provider`, `messageType`, and a `template` object; the path segment then acts only as a draft key for validation. Supported template variables include `{{subject}}`, `{{body}}`, `{{eventType}}`, `{{resourceType}}`, `{{resourceId}}`, `{{severity}}`, `{{messageId}}`, `{{policyId}}`, `{{dedupeKey}}`, `{{triggeredAt}}`, and `{{createdAt}}`.

The renderer is fail-closed for template syntax: unknown tokens such as `{{env.SECRET}}`, unopened `}}`, or unclosed `{{` delimiters are rejected on create, update, and render preview. Provider fields that are documented as JSON arrays or objects are also parsed during validation so malformed Block Kit, DingTalk feed cards, Feishu cards, WeCom cards, PagerDuty links/images, or webhook JSON bodies do not reach provider delivery.

When a job-instance policy references an enabled stored template by `id` or `templateKey`, the materializer renders the template before message insertion. `subject`/`title` can override the normalized message subject; `body`/`text`/`content` can override the normalized message body; the complete rendered JSON is stored under `payload.template` together with `templateRef` and `templateKey`. Provider renderers prefer `payload.template` over channel inline `config.template`, so one enabled stored template can drive Slack/DingTalk/Feishu/WeCom/PagerDuty/webhook/email payload shape without duplicating channel secrets or being shadowed by channel defaults.

Template rows never store provider credentials. Webhook URLs, signing keys, PagerDuty routing keys, SMTP URLs, SMTP passwords, authorization headers, custom secret headers, and app-style credentials such as `appId`/`appSecret` remain on the owning channel row's `secretRefs` only.

## Message fields

`NotificationMessageSummary` contains:

- `sourceType` and `sourceId`, such as `job_instance` and the instance id.
- `policyId`.
- `eventType`, such as `job_instance.failed`.
- `resourceType` and `resourceId`, such as `job` and the job id.
- `severity`, `subject`, `body`, and provider-neutral `payloadJson`.
- `dedupeKey` and optional `traceId`.
- `status`, `createdAt`, and `updatedAt`.

The job materializer creates subjects like `Tikeo job <name>: <status-token>` and payload fields including `eventType`, `jobId`, `jobName`, `namespace`, `app`, `instanceId`, `status`, `reason`, `logsUrl`, and `consoleUrl`.

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

## Provider schema and delivery behavior

`GET /api/v1/notification-channel-types` returns schema metadata used by the channel drawer. The metadata separates non-secret `requiredConfigKeys` from `requiredTargetKeys`, because built-in targets such as webhook URLs, PagerDuty routing keys, and SMTP URLs should normally be supplied through the channel row's `secretRefs` rather than raw config. Server validation also enforces provider `messageType` values and required template fields for built-in providers. Built-in seed/API examples use channel-scoped refs such as `env:TIKEO_NOTIFICATION_CHANNEL_FEISHU_INTERACTIVE_WEBHOOK_URL`, not shared refs like `env:FEISHU_WEBHOOK_URL`.

Official-document-backed built-in variants currently exposed by the drawer and delivery renderer:

| Provider | Message types and notable fields | Secret/ref behavior |
| --- | --- | --- |
| `webhook` | `json` body template. | Per-channel `secretRefs.url`, optional `secretRefs.authorization` or `secretRefs.headers.*`. |
| `slack` | `text`, `blockKit` (`blocks`), `attachments`; optional `threadTs` maps to Slack `thread_ts` for webhook thread replies when the parent message timestamp is known. | Incoming webhook URL should be a per-channel secret reference. |
| `dingtalk` | `text`, `markdown`, `link`, `actionCard` with single-button or `btns` JSON, and `feedCard`; `atMobiles`, `atUserIds`, `isAtAll`. | Per-channel webhook URL; optional per-channel `signingKey` signs URL with timestamp/HMAC. |
| `feishu` | `text`, `post`, `image` (`image_key`), `share_chat` (`share_chat_id`), and `interactive` card. | Per-channel webhook URL; optional per-channel `signingKey` adds body `timestamp`/`sign`. App-style `appId`/`appSecret` for plugins belongs in the same row's `secretRefs`. |
| `wechat_work` | `text`, `markdown`, `markdown_v2`, `image`, `news`, `file`, `voice`, and `template_card`; mentions for text-compatible messages. | Webhook URL as a per-channel secret ref. |
| `pagerduty` | Events API `trigger`, `acknowledge`, `resolve`; payload fields include `source`, `component`, `group`, `class`, `client`, `client_url`, `links`, `images`, and `custom_details`. | Routing/integration key must be supplied through this channel's `secretRefs.routingKey` / aliases. |
| `email` | `plain` text and stored `html` template shape. Runtime still sends text/plain through the SMTP adapter. | SMTP URL/password should be per-channel secret refs. |
| plugin webhook | Provider-neutral JSON unless plugin metadata supplies a custom template. | Plugin-defined. |

Rich provider families that require URLs, media IDs, cards, links, or image/chat identifiers fail closed unless the delivery has a channel inline `config.template` or an enabled policy `templateRef` rendered into `payload.template`. This covers DingTalk `link`/`actionCard`/`feedCard`, Feishu `image`/`share_chat`, and WeCom `image`/`news`/`file`/`voice`/`template_card`; placeholder provider payloads are not generated.

URL safety uses `alert::validate_webhook_url()`. Production targets should be HTTPS and publicly routable; `safetyPolicy.allowInsecureLoopback` is only for explicit local smoke tests.

Official/standard references used for the built-in schema include Slack incoming webhooks and `chat.postMessage` thread field semantics, DingTalk custom robot and robot security settings, Feishu custom bot and message-card custom-bot docs, WeCom group robot, PagerDuty Events API v2, IETF RFC 9110/8259 for generic HTTP/JSON, and RFC 5321/5322/2045/4954/6409 for email/SMTP concepts.

## UI reference

`web/src/pages/NotificationCenterPage.tsx` loads these endpoints in parallel:

- `GET /api/v1/notification-channel-types`
- `GET /api/v1/notification-channels`
- `GET /api/v1/notification-policies`
- `GET /api/v1/notification-templates`
- `GET /api/v1/notification-messages`
- `GET /api/v1/notification-delivery-attempts:queue-status`

The page renders statistics for channels, policies, templates, retry-pending attempts, and DLQ count. Tabs show channel summaries, reusable templates, policy summaries, queue/DLQ state with a **Retry due** action, and the latest 20 messages. Operators with `notifications:manage` can create, edit, delete, and render-preview templates; create/edit/delete channels and policies; and validate policies through `POST /api/v1/notification-policies/{id}:validate`. The template drawer is provider/message-type schema driven and intentionally does not show `secretRefsJson` or provider secret fields.

## Static examples that are safe to copy

List provider metadata:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/notification-channel-types \
  -H 'Authorization: Bearer <operator-token>'
```

Create and render a safe template preview:

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-templates \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "templateKey": "ops.slack.failure",
    "name": "Ops Slack failure",
    "provider": "slack",
    "messageType": "blockKit",
    "body": {
      "subject": "[{{severity}}] {{subject}}",
      "body": "{{body}}",
      "text": "{{subject}}",
      "blocks": [
        {"type":"section","text":{"type":"mrkdwn","text":"*{{subject}}*\n{{body}}"}}
      ]
    }
  }'

curl -fsS -X POST \
  http://127.0.0.1:9090/api/v1/notification-templates/ops.slack.failure/render \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{"sample":{"subject":"Nightly failed","body":"exit 2","severity":"critical"}}'
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
