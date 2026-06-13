---
title: Audit user guide
description: Human operator guide for the Tikeo audit console page.
---

# Audit user guide

Use Audit to reconstruct who changed what, when, from where, and with which scope. Audit is the cross-cutting evidence layer for Jobs, Workers, Scripts, Workflows, Notifications, Alerts, Settings, and authentication.

![Audit user guide screenshot](pathname:///img/screenshots/audit.svg)

## Prerequisites

- You can sign in to the Tikeo console and your role grants read access to this page.
- The target namespace/app is known before you change runtime objects.
- At least one recent instance, worker session, or audit event exists when you are verifying live behavior.
- For production changes, prepare a rollback note and an expected observation before saving.

## When to use

- Investigating a production incident.
- Preparing a change review or compliance export.
- Checking whether a rollback really happened.
- Confirming who changed notification or API-Key configuration.

## Key areas

| Area | What to read first |
| --- | --- |
| Search filters | Actor, action, resource type, resource id, scope, time range, and request id. |
| Event detail | Before/after summary, IP/user agent, auth method, and correlation ids. |
| Export | Time-bounded evidence package for review or compliance. |
| Cross-links | Direct jumps to Jobs, Instances, Workers, Scripts, Settings, and delivery records. |

## Typical workflow

1. Filter by time first, then narrow by resource id or actor.
2. Open the event detail and read correlation ids before drawing conclusions.
3. Use cross-links to inspect the affected runtime object.
4. Export only the bounded incident window needed for review.
5. Store the export alongside incident notes and notification evidence.

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

This guide intentionally keeps API details in the appendix. If you need to inspect implementation or automate the same workflow, use these anchors: `Audit`, `web/src/pages/AuditLogsPage.tsx`, `/api/v1/audit-logs`, `/api/v1/audit-logs:export`.

## Production checklist

- [ ] Owner scope and operational responsibility are clear.
- [ ] The change has a small verification path and rollback note.
- [ ] Evidence includes object id, time, operator, status, and related instance or delivery id.
- [ ] Public links use the configured platform URL when they leave the console.
- [ ] The team knows whether this page is describing execution, notification, alerting, or governance semantics.
