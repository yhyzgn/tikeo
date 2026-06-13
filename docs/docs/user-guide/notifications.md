---
title: Notifications user guide
description: Human operator guide for the Tikeo notifications console page.
---

# Notifications user guide

Use Notification Center to configure reusable delivery channels, provider-specific message templates, policy rules, job/workflow bindings, delivery attempts, retry and dead-letter visibility. It is separate from Alerts: alerts decide whether a rule fires; notifications deliver messages.

![Notifications user guide screenshot](pathname:///img/screenshots/notifications.svg)

## Prerequisites

- You can sign in to the Tikeo console and your role grants read access to this page.
- The target namespace/app is known before you change runtime objects.
- At least one recent instance, worker session, or audit event exists when you are verifying live behavior.
- For production changes, prepare a rollback note and an expected observation before saving.

## When to use

- A Job should notify on running, retrying, success, failure, or always.
- A Workflow should send provider-specific cards.
- Alerts need a shared channel without duplicating webhook secrets.
- Operators need delivery attempts and test-send evidence.

## Key areas

| Area | What to read first |
| --- | --- |
| Channels | Provider, scope, secretRefs, target, supportsTestSend=true, and redacted summaries. |
| Templates | Provider message type: blockKit, actionCard, feedCard, interactive, share_chat, markdown_v2, template_card. |
| Policies | Event filters, owner scope, templateRef, retry, dedupe, and routing rules. |
| Deliveries | Rendered payload, status, retry count, provider response, and dead-letter action. |

## Typical workflow

1. Create or select a channel and test it if the provider supports test send.
2. Create a template with variable mapping for instance id, status, operator, time, trigger type, and public console URL.
3. Create a policy that binds event types to a channel/template pair.
4. Bind the policy to a Job or Workflow event.
5. Trigger a real instance and inspect delivery attempts, not only provider chat history.

## Decision table

| Situation | Human decision | Evidence to collect |
| --- | --- | --- |
| First setup | Use narrow scope and one small verification run. | Screenshot, object id, instance id, audit event. |
| Incident | Freeze risky changes until the failing object is understood. | Timeline, attempts, logs, delivery attempts. |
| Production rollout | Change one dimension at a time and compare before/after. | Version diff, Dashboard health, audit trail. |
| Rollback | Prefer reverting to a known version over ad-hoc edits. | Previous version id, rollback audit, new verification run. |


## Quick path: channel → template → policy → event → delivery

The fastest safe path is chainable: create a channel, create a template, create a policy, bind it to an execution event, then inspect the delivery attempt. Keep secrets in `secretRefs`; API summaries should show redacted references, not raw webhook URLs or tokens.

```bash
CHANNEL_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-channels \
  -H "authorization: Bearer $TIKEO_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"name":"prod-feishu","provider":"feishu","secretRefs":{"webhook":"secret://tikeo/feishu/webhook"},"supportsTestSend":true}' | jq -r '.data.id')"

TEMPLATE_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-templates \
  -H "authorization: Bearer $TIKEO_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"name":"job-failure-card","provider":"feishu","messageType":"interactive","templateRef":"builtin.job.failure.card"}' | jq -r '.data.id')"

POLICY_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/notification-policies \
  -H "authorization: Bearer $TIKEO_TOKEN" \
  -H 'content-type: application/json' \
  -d "{\"name\":\"job-failure\",\"channelId\":\"$CHANNEL_ID\",\"templateId\":\"$TEMPLATE_ID\",\"eventTypes\":[\"job_instance.failed\"],\"enabled\":true}" | jq -r '.data.id')"
```

Supported provider message families include `blockKit` for Slack, `actionCard` and `feedCard` for DingTalk, `interactive` and `share_chat` for Feishu, `markdown_v2` for WeCom, `template_card` for WeChat Work style cards, PagerDuty events, email messages, generic webhook payloads, and plugin webhook adapters.

## Verify

- The page shows a current object, not stale browser state.
- A user with read-only permissions can inspect evidence but cannot make privileged changes.
- A real operation produces a visible audit event and, when relevant, an instance or delivery record.
- The console link can be copied into an incident note and still identifies the same object.

## Troubleshooting

| Symptom | Response |
| --- | --- |
| Page looks empty | Check namespace/app filters and role permissions before assuming data loss. |
| Object exists but action is disabled | Confirm RBAC, object state, and whether the action would cross scope boundaries. |
| UI result differs from chat/email | Trust Tikeo delivery attempts and instance evidence first, then compare provider history. |
| Time order is confusing | Use server timestamps, attempt numbers, and audit request ids instead of local browser order. |

## Reference anchors

This guide intentionally keeps API details in the appendix. If you need to inspect implementation or automate the same workflow, use these anchors: `Notifications`, `crates/tikeo-server/src/http/routes/notifications.rs`, `crates/tikeo-server/src/http/routes/notification_templates.rs`, `notification_templates`, `/api/v1/notification-templates`, `/api/v1/notification-templates/{id}/render`, `templateRef`, `blockKit`, `actionCard`, `feedCard`, `interactive`, `share_chat`, `markdown_v2`, `template_card`, `PagerDuty`, `supportsTestSend=true`.

## Production checklist

- [ ] Owner scope and operational responsibility are clear.
- [ ] The change has a small verification path and rollback note.
- [ ] Evidence includes object id, time, operator, status, and related instance or delivery id.
- [ ] Public links use the configured platform URL when they leave the console.
- [ ] The team knows whether this page is describing execution, notification, alerting, or governance semantics.
