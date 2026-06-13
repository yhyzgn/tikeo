---
title: Jobs user guide
description: Human operator guide for the Tikeo jobs console page.
---

# Jobs user guide

Use Jobs to define reusable execution contracts: owner scope, processor or script binding, schedule, retry behavior, worker eligibility, notification bindings, version history, and manual or API triggers.

![Jobs user guide screenshot](pathname:///img/screenshots/jobs.svg)

## Prerequisites

- You can sign in to the Tikeo console and your role grants read access to this page.
- The target namespace/app is known before you change runtime objects.
- At least one recent instance, worker session, or audit event exists when you are verifying live behavior.
- For production changes, prepare a rollback note and an expected observation before saving.

## When to use

- Create a new API, cron, fixed-rate, or one-shot task.
- Change retry, schedule, or worker targeting.
- Trigger a manual single execution or broadcast execution.
- Review impact before modifying a widely used task.

## Key areas

| Area | What to read first |
| --- | --- |
| Definition form | Name, namespace, app, schedule type, processor/script/plugin binding, timeout, retry, and misfire settings. |
| Targeting panel | Worker pool, tags, region, cluster, broadcastSelector, and scheduling advice. |
| Version drawer | Immutable change history, author, created time, diff, rollback entry point. |
| Trigger panel | single trigger, broadcast trigger, API parameters, and result link to the created instance. |

## Typical workflow

1. Choose namespace and app before any execution details.
2. Select the executor binding and verify at least one Worker advertises that capability.
3. Set retry and timeout based on failure class, not by copying another Job blindly.
4. Save, inspect version history, then open scheduling advice.
5. Trigger a single run first; use broadcast only after selector preview is correct.

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

This guide intentionally keeps API details in the appendix. If you need to inspect implementation or automate the same workflow, use these anchors: `Jobs`, `web/src/pages/JobsPage.tsx`, `/api/v1/jobs`, `/api/v1/jobs/{job}:trigger`, `triggerType=api`, `executionMode=single`, `broadcastSelector`.

## Production checklist

- [ ] Owner scope and operational responsibility are clear.
- [ ] The change has a small verification path and rollback note.
- [ ] Evidence includes object id, time, operator, status, and related instance or delivery id.
- [ ] Public links use the configured platform URL when they leave the console.
- [ ] The team knows whether this page is describing execution, notification, alerting, or governance semantics.
