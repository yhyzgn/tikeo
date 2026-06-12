---
title: Notifications user guide
description: Operator workflow for Tikeo Notification Center channels, policies, messages, delivery retry, DLQ, UI inspection, and alerting boundaries.
keywords: [notification center, outbound notifications, webhook, slack, pagerduty, retry, dlq]
---

# Notifications user guide

Notification Center is the reusable outbound delivery layer in Tikeo. Use it when you need to send lifecycle or operational messages to Slack, DingTalk, Feishu/Lark, WeChat Work, PagerDuty, email, generic webhooks, or plugin-provided webhook-compatible providers.

The operator-verified boundary is important:

- **Notification Center** owns reusable outbound channels, policies/subscriptions, normalized messages, delivery attempts, retry, and DLQ state. Current source: `crates/tikeo-server/src/notification.rs`, `crates/tikeo-server/src/http/routes/notification_providers.rs`, `crates/tikeo-storage/src/repository/notification.rs`, and `web/src/pages/NotificationCenterPage.tsx`.
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
| Alerting produces an incident event | `alert` | `alert.firing` and `alert.recovered` are materialized into Notification Center messages and delivery attempts for matching `global` or `alert_rule` policies. |

Use Alerts instead when you need condition evaluation, dedupe/silence/recovery behavior, abnormal-condition history, or incident review.

## Provider types

The implemented built-in channel types come from `builtin_channel_types()` in `crates/tikeo-server/src/http/routes/notification_providers.rs`.

| Provider | Non-secret config | Target / secret refs | Message types exposed by the drawer |
| --- | --- | --- | --- |
| `webhook` | none required | `secretRefs.url`, optional `authorization` | `json` |
| `slack` | optional `threadTs` | `secretRefs.url` | `text`, `blockKit`, `attachments` |
| `dingtalk` | `atMobiles`, `atUserIds`, `isAtAll` | `secretRefs.url`, optional `signingKey` | `text`, `markdown`, `link`, `actionCard`, `feedCard` |
| `feishu` | none required | `secretRefs.url`, optional `signingKey` | `text`, `post`, `image`, `share_chat`, `interactive` |
| `wechat_work` | mention lists | `secretRefs.url` | `text`, `markdown`, `markdown_v2`, `image`, `news`, `file`, `voice`, `template_card` |
| `pagerduty` | `source`, `component`, `group`, `class`, `client`, `clientUrl`, `links`, `images`, `customDetails` | `secretRefs.routingKey` / aliases | `trigger`, `acknowledge`, `resolve` |
| `email` | `to`/`recipients`, `from`, `username` | `secretRefs.smtpUrl`, optional `password` | `plain`, stored `html` shape; runtime text/plain |
| plugin type | plugin-defined | plugin-defined | plugin-defined |

Webhook-style providers accept `url`, `webhookUrl`, or `webhook_url` as target keys, but for built-ins the UI and validation prefer `secretRefs`. PagerDuty accepts `routingKey`, `routing_key`, `integrationKey`, or `integration_key` through `secretRefs`. Email accepts `to` or `recipients`; its SMTP endpoint can come from `secretRefs.smtpUrl`, `secretRefs.smtp_url`, `secretRefs.url`, `config.smtpUrlSecretRef`, `config.smtp_url_secret_ref`, `secretRefs.smtpUrlSecretRef`, or `secretRefs.smtp_url_secret_ref`. SMTP auth passwords use `config.passwordSecretRef`, `config.password_secret_ref`, `secretRefs.password`, `secretRefs.passwordSecretRef`, or `secretRefs.password_secret_ref`.

Runtime secret resolution for Notification Center currently resolves `env:` references or bare environment variable names through the process environment. Do not enter raw secret values in `config` or `secretRefs`.

### Per-channel secretRefs

Every notification channel row owns its own `secretRefs` object. Do not configure one global `FEISHU_WEBHOOK_URL`, Slack webhook, PagerDuty routing key, or SMTP credential and reuse it for all channels. Use stable names that encode the business destination or provider/message type, for example `env:TIKEO_NOTIFICATION_CHANNEL_BILLING_FEISHU_WEBHOOK_URL`, `env:TIKEO_NOTIFICATION_CHANNEL_BILLING_FEISHU_SIGNING_KEY`, or `env:TIKEO_NOTIFICATION_CHANNEL_ONCALL_PAGERDUTY_ROUTING_KEY`. The built-in seed/generated examples use the pattern `env:TIKEO_NOTIFICATION_CHANNEL_<PROVIDER>_<MESSAGE_TYPE>_<PURPOSE>` to make this scoping visible.

For app-style providers or plugins that require credentials such as `appId` and `appSecret`, keep those references in the same channel's `secretRefs` as well; do not move them into server-wide config. The current built-in Feishu/Lark custom bot uses `secretRefs.url` and optional `secretRefs.signingKey`.

## Quick path: channel → template → policy → event → delivery

Use this path when you want one job failure policy to deliver through one reusable outbound channel. It is intentionally chainable: each command stores the ID returned by the previous command. Keep real URLs and auth values in environment variables referenced by `secretRefs`; do not put raw provider tokens in channel `config`, templates, tickets, screenshots, or examples.

Prerequisites:

- You have a bearer token with `notifications:manage` and `notifications:test` when using retry scan.
- The Server process can resolve this channel's `env:TIKEO_NOTIFICATION_CHANNEL_BILLING_OPS_WEBHOOK_URL` and optional `env:TIKEO_NOTIFICATION_CHANNEL_BILLING_OPS_WEBHOOK_AUTH` from its own environment.
- You know the namespace/app or job owner that should emit the event.
- Built-in providers report `supportsTestSend=true`; after saving a channel, use the edit drawer **Send a test** action or `POST /api/v1/notification-channels/{id}/test-send` to validate the saved channel with redacted output. Also keep policy validation, generated messages, delivery attempts, and retry/DLQ state in the rollout checklist.

```bash
export TOKEN='<operator-bearer-token>'

CHANNEL_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-channels \
  -H "Authorization: Bearer ${TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{
    "scopeType": "app",
    "namespace": "prod",
    "app": "billing",
    "name": "billing-ops-webhook",
    "provider": "webhook",
    "enabled": true,
    "config": {"messageType": "json"},
    "secretRefs": {
      "url": "env:TIKEO_NOTIFICATION_CHANNEL_BILLING_OPS_WEBHOOK_URL",
      "authorization": "env:TIKEO_NOTIFICATION_CHANNEL_BILLING_OPS_WEBHOOK_AUTH"
    }
  }' | jq -r '.data.id')"

test -n "${CHANNEL_ID}" && test "${CHANNEL_ID}" != "null"

TEMPLATE_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-templates \
  -H "Authorization: Bearer ${TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{
    "templateKey": "ops.webhook.failure",
    "name": "Ops webhook failure",
    "provider": "webhook",
    "messageType": "json",
    "enabled": true,
    "body": {
      "subject": "[{{severity}}] {{subject}}",
      "body": "{{body}}",
      "eventType": "{{eventType}}",
      "resourceId": "{{resourceId}}"
    },
    "variables": {"severity": "critical"}
  }' | jq -r '.data.id')"

test -n "${TEMPLATE_ID}" && test "${TEMPLATE_ID}" != "null"

curl -fsS -X POST "http://127.0.0.1:9090/api/v1/notification-templates/${TEMPLATE_ID}/render" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{"sample":{"subject":"Nightly failed","body":"exit 2","eventType":"job_instance.failed","resourceId":"instance-demo","severity":"critical"}}' | jq .data

POLICY_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-policies \
  -H "Authorization: Bearer ${TOKEN}" \
  -H 'Content-Type: application/json' \
  -d "$(python3 - <<'PYJSON'
import json, os
print(json.dumps({
  "ownerType": "app",
  "ownerId": "prod/billing",
  "name": "billing terminal failures",
  "eventFamily": "job_instance",
  "eventFilter": {
    "eventTypes": ["job_instance.failed", "job_instance.retry_exhausted"],
    "statuses": ["failed", "retry_exhausted"]
  },
  "channelRefs": [{"channelId": os.environ["CHANNEL_ID"]}],
  "templateRef": os.environ["TEMPLATE_ID"],
  "severity": "critical",
  "enabled": True,
  "dedupeSeconds": 300
}, separators=(",", ":")))
PYJSON
)" | jq -r '.data.id')"

test -n "${POLICY_ID}" && test "${POLICY_ID}" != "null"

curl -fsS -X POST "http://127.0.0.1:9090/api/v1/notification-policies/${POLICY_ID}:validate" \
  -H "Authorization: Bearer ${TOKEN}" | jq .data
```

Trigger a matching job event by running a job in the same owner scope and letting it reach `failed` or `retry_exhausted`, then inspect delivery:

```bash
curl -fsS 'http://127.0.0.1:9090/api/v1/notification-messages?eventFamily=job_instance' \
  -H "Authorization: Bearer ${TOKEN}" | jq '.data.items[0]'

curl -fsS http://127.0.0.1:9090/api/v1/notification-delivery-attempts \
  -H "Authorization: Bearer ${TOKEN}" | jq '.data.items[0]'

curl -fsS http://127.0.0.1:9090/api/v1/notification-delivery-attempts:queue-status \
  -H "Authorization: Bearer ${TOKEN}" | jq .data
```

Verification is complete when policy validation reports `valid=true`, a matching event creates a normalized message, the delivery attempt references `${CHANNEL_ID}`, and the attempt reaches `delivered`, `retry_pending`, or `dead_letter` with a redacted target.

## Safe channel creation example

The API uses the shared `{code,message,data}` envelope. The quick path above is the preferred copy-paste flow. This abbreviated body shows the important channel safety rule: credentials belong in `secretRefs`, not in `config`.

```json
{
  "scopeType": "app",
  "namespace": "prod",
  "app": "billing",
  "name": "billing-ops-webhook",
  "provider": "webhook",
  "enabled": true,
  "config": {"messageType": "json"},
  "secretRefs": {
    "url": "env:TIKEO_NOTIFICATION_CHANNEL_BILLING_OPS_WEBHOOK_URL",
    "authorization": "env:TIKEO_NOTIFICATION_CHANNEL_BILLING_OPS_WEBHOOK_AUTH"
  }
}
```

The exact `id` is generated by storage. `secretRefsJson` is skipped during serialization, and `configJson` is redacted by `NotificationChannelSummary::from()` in `crates/tikeo-storage/src/repository/notification.rs`.

## Reusable templates

Use templates when the same policy-level message body should be reused across channels or edited without touching channel credentials. A template has `templateKey`, `provider`, `messageType`, `body`, optional `variables`, and `enabled`. For built-in providers, the drawer and backend validate the selected message type and its required body fields from provider metadata.

Rendering is intentionally a safe token replacer, not an arbitrary expression engine. Unknown tokens such as `{{env.SECRET}}`, malformed delimiters, and malformed JSON array/object fields are rejected before a template can be saved or previewed.

Safe Slack Block Kit template example:

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-templates \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{
    "templateKey": "ops.slack.failure",
    "name": "Ops Slack failure",
    "provider": "slack",
    "messageType": "blockKit",
    "enabled": true,
    "body": {
      "subject": "[{{severity}}] {{subject}}",
      "body": "{{body}}",
      "text": "{{subject}}",
      "blocks": [
        {"type":"section","text":{"type":"mrkdwn","text":"*{{subject}}*\n{{body}}"}}
      ]
    },
    "variables": {"severity": "critical"}
  }'
```

Dry-run render before attaching the template to a policy:

```bash
curl -fsS -X POST \
  http://127.0.0.1:9090/api/v1/notification-templates/ops.slack.failure/render \
  -H 'Authorization: Bearer <operator-token>' \
  -H 'Content-Type: application/json' \
  -d '{"sample":{"subject":"Nightly failed","body":"exit 2","severity":"critical","eventType":"job_instance.failed"}}'
```

Set a policy `templateRef` to the stored template `id` or `templateKey`. During job-instance materialization, enabled templates can override the normalized message `subject` and `body`, and the rendered provider body is stored under `payload.template`. Provider delivery then uses that rendered template before any channel inline `config.template`, so policy-selected enabled storage templates are not silently shadowed by channel defaults. Missing or disabled template refs are ignored for compatibility, so production policies should reference existing enabled templates and be checked in the UI before rollout.

Template bodies are not secret stores. Keep webhook URLs, signing keys, PagerDuty routing keys, SMTP URLs/passwords, authorization headers, and custom header values in that channel row's own `secretRefs`.

Rich message types that need provider-specific fields fail closed when no channel inline template or enabled policy template is available. DingTalk `link`/`actionCard`/`feedCard`, Feishu `image`/`share_chat`, and WeCom `image`/`news`/`file`/`voice`/`template_card` must be backed by real operator-supplied template fields; Tikeo does not synthesize placeholder URLs or fake media IDs for production delivery.

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
      {"channelId": "${CHANNEL_ID}"}
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
  http://127.0.0.1:9090/api/v1/notification-policies/${POLICY_ID}:validate \
  -H 'Authorization: Bearer <operator-token>'
```

Validation returns `valid`, `channelCount`, `missingChannelIds`, `disabledChannelIds`, and `issues`.

## Owner and event semantics

The API currently accepts owner types `global`, `namespace`, `app`, `job`, `workflow`, `workflow_node`, `alert_rule`, and `worker_pool`, and event families `job_instance`, `workflow`, `alert`, `worker`, and `script_governance`. Runtime materialization is implemented for `job_instance` policies (`global`, `namespace`, `app`, `job` owners), alert events (`global`, `alert_rule` owners), and workflow notification nodes (`global`, `workflow`, `workflow_node` owners).

For job-instance notifications, the materializer currently matches:

- `global` policies for all jobs.
- `namespace` policies when `ownerId` equals the job namespace.
- `app` policies when `ownerId` equals either the app name or `namespace/app`.
- `job` policies when `ownerId` equals the job id.

The filter checks `eventFilter.statuses` against the stable status token and `eventFilter.eventTypes` or `eventFilter.event_types` against the full event type. Empty `statuses` or `eventTypes` arrays mean that dimension is not restricted.

For workflow notification nodes, the runtime emits `workflow_node.notification_requested` with filter status `notification_requested`. `workflow` policies match `ownerId=<workflow id>`. `workflow_node` policies match `ownerId=<workflow id>:<node key>` (the inline node compiler uses this shape), the node instance id, or the node key. Optional `eventFilter.workflowIds` and `eventFilter.nodeKeys` further narrow matching. A notification node can also carry inline `config.channelRefs`; at materialization time Tikeo compiles/updates a workflow-node policy and creates the message plus delivery attempts.

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

Open **Notification Center / 通知中心** at `/notifications`. The console page is backed by `web/src/pages/NotificationCenterPage.tsx`, `web/src/pages/notifications/TemplateDrawer.tsx`, and `web/src/api/notifications.ts`; it supports redacted channel/policy inspection plus governed channel, template, and policy create/edit/delete/validate operations.

| Tab | What to check |
| --- | --- |
| Channels | Channel name, provider, scope, redacted target, whether a secret is configured, enabled state, create/edit/delete drawers, and backend conflict handling for referenced channels. |
| Templates | Provider, message type, enabled state, schema-driven body fields, variables JSON, backend render preview, create/edit/delete actions, and no secret-field exposure. |
| Policies | Owner, event family, severity, dedupe seconds, enabled state, create/edit/delete drawers, channel multi-select, template selector, JSON event filters, and policy validation. |
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


## Verify

After creating or editing a notification flow, verify the full chain rather than only the form save:

1. `GET /api/v1/notification-channels/${CHANNEL_ID}` returns the expected provider, scope, enabled state, and redacted target summary.
2. `POST /api/v1/notification-policies/${POLICY_ID}:validate` returns `valid=true` and the expected `channelCount`.
3. A matching job-instance event creates a normalized message with the expected `eventType`, `subject`, `body`, `severity`, and resource identifiers.
4. The delivery attempt references the selected channel and reaches `delivered`, `retry_pending`, or `dead_letter` with an inspectable reason.
5. UI tabs show the same channel, template, policy, message, and attempt state without exposing `secretRefs` values.

For a saved built-in channel, run the edit drawer **Send a test** action or `POST /api/v1/notification-channels/${CHANNEL_ID}/test-send` with `notifications:test` permission. The response shows delivered/retry state, status code, rendered payload, and redacted target; it does not expose `secretRefs`. Keep policy validation, real matching events, message rows, delivery attempts, and retry/DLQ status as the broader acceptance path.


## Production checklist

- [ ] Every provider token, webhook URL, signing key, routing key, SMTP URL, SMTP password, authorization header, and custom header secret is referenced through `secretRefs`; no raw secret is stored in channel `config`, templates, docs, screenshots, or shell history.
- [ ] Channel scope matches the intended blast radius: global channels are rare, namespace/app channels are preferred, and job-specific policies are used for noisy workloads.
- [ ] Message templates include real provider-required fields for rich message types; Tikeo must not invent placeholder links, fake media IDs, or missing card fields during production delivery.
- [ ] Job policies filter terminal and retry events deliberately so retry noise does not page the same audience as terminal failure.
- [ ] Operators know the retry/DLQ runbook, including `notification_delivery.*` defaults, bounded `retry-due` scans, and how to inspect `dead_letter` reasons before creating a replacement event.
- [ ] Alert rules and notification policies are named so incident semantics remain separate from normal lifecycle delivery.
