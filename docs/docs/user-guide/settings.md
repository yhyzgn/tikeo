---
title: Settings and governance guide
description: Human operator guide for the Tikeo settings console page.
---

# Settings and governance guide

Use Settings to manage platform-level governance: users, roles, API-Key access, RBAC, scope bindings, platform public URL, and integration defaults used by notifications and external console links.

![Settings and governance guide screenshot](pathname:///img/screenshots/settings.svg)

## Prerequisites

- You can sign in to the Tikeo console and your role grants read access to this page.
- The target namespace/app is known before you change runtime objects.
- At least one recent instance, worker session, or audit event exists when you are verifying live behavior.
- For production changes, prepare a rollback note and an expected observation before saving.

## When to use

- Onboarding or offboarding users.
- Creating app-scoped SDK API keys.
- Changing the public console URL used in notification buttons.
- Reviewing RBAC before production rollout.

## Key areas

| Area | What to read first |
| --- | --- |
| Users and roles | Owner/admin/operator/viewer style responsibilities, invitation, disable, and audit. |
| API-Key | App-scoped keys, expiration, rotation, and least privilege. |
| Scope management | Namespace/app/execution-pool hierarchy used by Jobs, Workers, Notifications, API-Key scope bindings, and Audit filters. |
| Platform URL | Public console base URL for delivery templates and no-login console pages. |

## Typical workflow

1. Review RBAC before adding powerful users or keys.
2. Create API keys at the narrowest namespace/app scope that still works.
3. Set the platform public URL before enabling external notification buttons.
4. Rotate keys deliberately and confirm dependent Workers or automation have been updated.
5. Use Audit to confirm every governance change.

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

This guide intentionally keeps API details in the appendix. If you need to inspect implementation or automate the same workflow, use these anchors: `Settings`, `web/src/routes.tsx`, `API-Key`, `RBAC`.

## Production checklist

- [ ] Owner scope and operational responsibility are clear.
- [ ] The change has a small verification path and rollback note.
- [ ] Evidence includes object id, time, operator, status, and related instance or delivery id.
- [ ] Public links use the configured platform URL when they leave the console.
- [ ] The team knows whether this page is describing execution, notification, alerting, or governance semantics.


### Scope model and execution pools

Tikeo uses `Namespace → App → Worker Pool` as its scope model. Namespace is the environment, team, or business boundary; App is the application boundary; Worker Pool is an optional execution-resource group under one App.

A Worker Pool can represent a Worker service, runtime class, machine group, or isolated queue. Use it when you need capacity isolation, queue/concurrency quotas, narrower API-Key/OIDC permissions, notification scope, job routing, or operations lookup. Small deployments can leave Worker Pool empty and continue matching by Namespace/App. Workers join a pool by registering the `worker_pool` or `worker-pool` label.
