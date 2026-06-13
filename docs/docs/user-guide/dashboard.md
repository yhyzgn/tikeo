---
title: Dashboard user guide
description: Human operator guide for the Tikeo dashboard console page.
---

# Dashboard user guide

Use the Dashboard as the morning command center for cluster health, queue pressure, Worker Tunnel availability, recent failures, and whether the platform is safe to operate before changing Jobs or Workflows.

![Dashboard user guide screenshot](pathname:///img/screenshots/dashboard.svg)

## Prerequisites

- You can sign in to the Tikeo console and your role grants read access to this page.
- The target namespace/app is known before you change runtime objects.
- At least one recent instance, worker session, or audit event exists when you are verifying live behavior.
- For production changes, prepare a rollback note and an expected observation before saving.

## When to use

- Start-of-day production check.
- Before and after a deployment.
- When Jobs are pending longer than expected.
- When an incident needs a single health snapshot.

## Key areas

| Area | What to read first |
| --- | --- |
| Cluster summary | Server health, storage reachability, scheduler heartbeat, and current release/version context. |
| Execution pressure | Pending/running/retrying/failed instance counters and queue age trends. |
| Worker availability | Online sessions, lost Workers, capability coverage, and transport health. |
| Incident strip | Recent failures, alert/notification delivery status, and links into instance evidence. |

## Typical workflow

1. Open Dashboard before making changes.
2. Read the top health strip from left to right.
3. If queue pressure is high, drill into Instances before triggering more work.
4. If Worker coverage is low, open Workers and compare capabilities before editing Jobs.
5. After a deployment, keep the Dashboard open until the metrics stabilize.

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

This guide intentionally keeps API details in the appendix. If you need to inspect implementation or automate the same workflow, use these anchors: `Dashboard`, `web/src/pages/Dashboard.tsx`, `/api/v1/metrics/summary`, `/api/v1/cluster`.

## Production checklist

- [ ] Owner scope and operational responsibility are clear.
- [ ] The change has a small verification path and rollback note.
- [ ] Evidence includes object id, time, operator, status, and related instance or delivery id.
- [ ] Public links use the configured platform URL when they leave the console.
- [ ] The team knows whether this page is describing execution, notification, alerting, or governance semantics.
