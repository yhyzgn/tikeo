---
title: Scripts user guide
description: Human operator guide for the Tikeo scripts console page.
---

# Scripts user guide

Use Scripts to manage reviewed, versioned, auditable code snippets that Workers can execute through controlled runtimes. Drafts are editable; published versions are immutable and can be bound to Jobs.

![Scripts user guide screenshot](pathname:///img/screenshots/scripts.svg)

## Prerequisites

- You can sign in to the Tikeo console and your role grants read access to this page.
- The target namespace/app is known before you change runtime objects.
- At least one recent instance, worker session, or audit event exists when you are verifying live behavior.
- For production changes, prepare a rollback note and an expected observation before saving.

## When to use

- A small operational task is better expressed as a script than a compiled normal processor.
- You need reviewable diffs and rollback.
- The same script version should be bound to multiple Jobs.
- Operators need stdout/stderr and exception stack evidence.

## Key areas

| Area | What to read first |
| --- | --- |
| Draft editor | Language/runtime, parameters, validation notes, and saved draft state. |
| Diff review | Line-level diff between draft and published version before release. |
| Published versions | Immutable version id, author, checksum, created time, and rollback candidate. |
| Bindings | Jobs that reference each version and recent instance evidence. |

## Typical workflow

1. Write or edit a draft with a clear expected input/output contract.
2. Review diff before publishing; do not publish hidden behavior changes.
3. Publish an immutable version and copy its version id.
4. Bind a Job to that exact version, then trigger a small run.
5. If the run fails, inspect stdout/stderr, runtime exception, and rollback to the previous version.

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

This guide intentionally keeps API details in the appendix. If you need to inspect implementation or automate the same workflow, use these anchors: `Scripts`, `web/src/pages/ScriptsPage.tsx`, `/api/v1/scripts`, `diff`.

## Production checklist

- [ ] Owner scope and operational responsibility are clear.
- [ ] The change has a small verification path and rollback note.
- [ ] Evidence includes object id, time, operator, status, and related instance or delivery id.
- [ ] Public links use the configured platform URL when they leave the console.
- [ ] The team knows whether this page is describing execution, notification, alerting, or governance semantics.
