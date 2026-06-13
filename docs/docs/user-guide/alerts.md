---
title: Alerts user guide
description: Human operator guide for the Tikeo alerts console page.
---

# Alerts user guide

Use Alerts to define conditions that fire, recover, silence, and route operational incidents. Alerts may use Notification Center channels for delivery, but their lifecycle is rule-oriented rather than message-template-oriented.

![Alerts user guide screenshot](pathname:///img/screenshots/alerts.svg)

## Prerequisites

- You can sign in to the Tikeo console and your role grants read access to this page.
- The target namespace/app is known before you change runtime objects.
- At least one recent instance, worker session, or audit event exists when you are verifying live behavior.
- For production changes, prepare a rollback note and an expected observation before saving.

## When to use

- Cluster health or queue pressure should wake someone up.
- A recurring failure class needs suppression windows.
- A recovery event should close the loop.
- You need to distinguish alert ownership from job notification ownership.

## Key areas

| Area | What to read first |
| --- | --- |
| Rules | Metric/event condition, severity, scope, duration, fire and recovery semantics. |
| Silences | Maintenance windows, owner, reason, expiry, and audit trail. |
| Routing | Linked notification channel or policy, dedupe, and escalation target. |
| History | Fire, update, recover, suppress, and delivery evidence. |

## Typical workflow

1. Define the condition in terms a human can verify.
2. Choose severity and owner before choosing a delivery channel.
3. Attach a Notification Center channel or policy if humans must be reached.
4. Create a bounded silence for maintenance rather than disabling the rule.
5. After recovery, review alert history and linked deliveries.

## Decision table

| Situation | Human decision | Evidence to collect |
| --- | --- | --- |
| First setup | Use narrow scope and one small verification run. | Screenshot, object id, instance id, audit event. |
| Incident | Freeze risky changes until the failing object is understood. | Timeline, attempts, logs, delivery attempts. |
| Production rollout | Change one dimension at a time and compare before/after. | Version diff, Dashboard health, audit trail. |
| Rollback | Prefer reverting to a known version over ad-hoc edits. | Previous version id, rollback audit, new verification run. |

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

This guide intentionally keeps API details in the appendix. If you need to inspect implementation or automate the same workflow, use these anchors: `Alerts`, `/api/v1/alerts`, `Notification Center`, `supportsTestSend=true`.

## Production checklist

- [ ] Owner scope and operational responsibility are clear.
- [ ] The change has a small verification path and rollback note.
- [ ] Evidence includes object id, time, operator, status, and related instance or delivery id.
- [ ] Public links use the configured platform URL when they leave the console.
- [ ] The team knows whether this page is describing execution, notification, alerting, or governance semantics.
