---
title: Workers user guide
description: Human operator guide for the Tikeo workers console page.
---

# Workers user guide

Use Workers to verify private execution capacity: online tunnel sessions, processor capabilities, lease freshness, lost reasons, and whether a Job selector can actually match real Workers.

![Workers user guide screenshot](pathname:///img/screenshots/workers.svg)

## Prerequisites

- You can sign in to the Tikeo console and your role grants read access to this page.
- The target namespace/app is known before you change runtime objects.
- At least one recent instance, worker session, or audit event exists when you are verifying live behavior.
- For production changes, prepare a rollback note and an expected observation before saving.

## When to use

- Before binding a Job to a new processor.
- When Jobs remain pending.
- After rotating Worker credentials or gateway configuration.
- When a broadcast selector might target too many machines.

## Key areas

| Area | What to read first |
| --- | --- |
| Session list | Worker id, display name, generation, online/lost state, heartbeat age, and tunnel endpoint. |
| Capability matrix | Processor names, script/plugin support, tags, region, cluster, and runtime metadata. |
| Diagnostics | Transport errors, last lost reason, reconnect count, and session history. |
| Dispatch evidence | Recent DispatchTask records and links to Instances that used this Worker. |

## Typical workflow

1. Open Workers before creating a selector-heavy Job.
2. Verify the needed processor appears exactly as the Job will reference it.
3. Check heartbeat age; stale sessions should not be used for new production rollouts.
4. Compare tags, region, and cluster before enabling broadcast.
5. If a Worker is lost, open related Instances and Audit before restarting blindly.

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

This guide intentionally keeps API details in the appendix. If you need to inspect implementation or automate the same workflow, use these anchors: `Workers`, `web/src/pages/WorkersPage.tsx`, `Worker Tunnel`, `DispatchTask`.

## Production checklist

- [ ] Owner scope and operational responsibility are clear.
- [ ] The change has a small verification path and rollback note.
- [ ] Evidence includes object id, time, operator, status, and related instance or delivery id.
- [ ] Public links use the configured platform URL when they leave the console.
- [ ] The team knows whether this page is describing execution, notification, alerting, or governance semantics.
