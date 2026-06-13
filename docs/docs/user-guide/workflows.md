---
title: Workflows user guide
description: Human operator guide for the Tikeo workflows console page.
---

# Workflows user guide

Use Workflows to design, validate, run, and replay DAG-based orchestration. The page is for humans who need to see dependency shape, not for dumping JSON into a text area.

![Workflows user guide screenshot](pathname:///img/screenshots/workflows.svg)

## Prerequisites

- You can sign in to the Tikeo console and your role grants read access to this page.
- The target namespace/app is known before you change runtime objects.
- At least one recent instance, worker session, or audit event exists when you are verifying live behavior.
- For production changes, prepare a rollback note and an expected observation before saving.

## When to use

- A business process has multiple dependent steps.
- You need visual readiness and replay evidence.
- A failure should block or skip downstream work deliberately.
- Notification nodes should be part of the execution story.

## Key areas

| Area | What to read first |
| --- | --- |
| Canvas | Node shape, dependency edges, validation markers, and selected-node details. |
| Version panel | Published DAG versions, diff, author, and rollback/replay entry points. |
| Run panel | Trigger source, input payload, dry-run result, and instance link. |
| Replay | Node timeline, attempts, logs, downstream effects, and delivery attempts. |

## Typical workflow

1. Sketch the DAG from business order, not from implementation convenience.
2. Validate before publishing and fix cycles or missing inputs.
3. Run a dry-run when selectors or dynamic inputs are involved.
4. Trigger a small real run and inspect replay before production scale.
5. Use notification nodes or policies for success/failure/always events.

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

This guide intentionally keeps API details in the appendix. If you need to inspect implementation or automate the same workflow, use these anchors: `Workflows`, `web/src/pages/WorkflowsPage.tsx`, `/api/v1/workflows`, `DAG`.

## Production checklist

- [ ] Owner scope and operational responsibility are clear.
- [ ] The change has a small verification path and rollback note.
- [ ] Evidence includes object id, time, operator, status, and related instance or delivery id.
- [ ] Public links use the configured platform URL when they leave the console.
- [ ] The team knows whether this page is describing execution, notification, alerting, or governance semantics.
