---
title: Instances user guide
description: Human operator guide for the Tikeo instances console page.
---

# Instances user guide

Use Instances as the execution evidence center. It shows who triggered work, which Worker attempted it, status transitions, retry intervals, stdout/stderr, business payload, runtime exception stack, delivery attempts, and audit context.

![Instances user guide screenshot](pathname:///img/screenshots/instances.svg)

## Prerequisites

- You can sign in to the Tikeo console and your role grants read access to this page.
- The target namespace/app is known before you change runtime objects.
- At least one recent instance, worker session, or audit event exists when you are verifying live behavior.
- For production changes, prepare a rollback note and an expected observation before saving.

## When to use

- A Job or Workflow failed.
- A task seems stuck in pending or retrying.
- You need the exact instance ID for notification templates.
- You need a public console link for external reviewers.

## Key areas

| Area | What to read first |
| --- | --- |
| Timeline | Created, pending, running, retrying, succeeded, failed, cancelled, and timestamps. |
| Attempts | Attempt number, worker id, assignment token, duration, retry reason, and terminal error. |
| Console | Logs, checkpoints, stdout, stderr, structured payload, and exception stack. |
| Delivery evidence | Notification messages sent for running, success, failure, retry, or always events. |

## Typical workflow

1. Filter by job, status, owner, or time range.
2. Open the newest failed or retrying instance first.
3. Read attempts before logs so you know which retry generated which output.
4. Use the console tab to copy stack traces and payload IDs.
5. Confirm notification delivery links point to the public console URL.

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

This guide intentionally keeps API details in the appendix. If you need to inspect implementation or automate the same workflow, use these anchors: `Instances`, `web/src/pages/InstancesPage.tsx`, `/api/v1/instances/{instance}`, `/api/v1/instances/{instance}/logs`.

## Production checklist

- [ ] Owner scope and operational responsibility are clear.
- [ ] The change has a small verification path and rollback note.
- [ ] Evidence includes object id, time, operator, status, and related instance or delivery id.
- [ ] Public links use the configured platform URL when they leave the console.
- [ ] The team knows whether this page is describing execution, notification, alerting, or governance semantics.
