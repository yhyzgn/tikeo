---
title: Dashboard user guide
description: Human operator guide for the Tikeo live operations cockpit.
---

# Dashboard user guide

Use the Dashboard as the live operations cockpit before you trigger work, roll out a new Worker, change notification channels, or investigate an incident. It aggregates Jobs, instances, Workers, dispatch queue, Notification Center delivery, cluster diagnostics, and audit activity into one page.

![Dashboard user guide screenshot](pathname:///img/screenshots/dashboard.svg)

## What the page shows

| Section | Source in the product | What to decide from it |
| --- | --- | --- |
| KPI strip | Jobs, instance summaries, Worker snapshot | Whether the platform has enabled Jobs, active instances, online Workers, broadcast workload, or visible failures. |
| 12-hour execution trend | Recent Job instances | Whether execution volume or failures are increasing, isolated, or quiet. Green segments are successful work; red segments are failed work. |
| Instance status distribution | Instance status plus result success/failure | Whether pending, dispatching, running, retrying, succeeded, failed, or cancelled states dominate right now. |
| Task schedule map | Job `scheduleType`, `scheduleExpr`, processor/script binding, enabled state | Which Cron/fixed/API plans are active and which processor or script each plan is bound to. |
| Queue pressure | Dispatch queue overview | Whether pending/running work is accumulating before you trigger more tasks. |
| Notification delivery | Alert delivery queue status | Whether provider delivery is healthy, retrying, failed, or in dead-letter state. |
| HA / gateway | Cluster diagnostics and Smart Gateway diagnostics | Whether Server HA, Worker gateway locality, remote gateway capacity, and outbox totals look safe. |
| Worker Mesh distribution | Worker snapshot grouped by namespace/app/cluster/region | Which application scopes have online capacity and which scopes lack a master or enough Workers. |
| Capability coverage | Structured Worker capabilities | Which SDK processors, script runners, plugin processors, and tags are actually advertised by online Workers. |
| Audit activity | Recent audit log page | What operators or API keys recently changed, and whether those actions succeeded or failed. |
| Risk signals | Derived from failures, queue, alert delivery, Worker count, and cluster status | Whether to continue normal operation or stop and triage first. |

The page is intentionally read-heavy. Use the action buttons to jump into Jobs, Instances, Workers, Dispatch Queue, Security, Notifications, or Audit when a panel needs detail.

## Realtime behavior

Dashboard opens multiple Server-Sent Event streams while the route is active:

| Stream | Purpose |
| --- | --- |
| `/api/v1/instances/stream` | Keeps Jobs and instance trend/status panels fresh. |
| `/api/v1/workers/stream` | Keeps Worker count, Worker Mesh, and capability coverage fresh. |
| `/api/v1/dispatch-queue/stream` | Keeps queue pressure fresh. |

It also runs a 3-second fallback refresh for the REST-backed panels, including cluster diagnostics, alert delivery queue status, audit logs, and job instance history. If the page is stale, verify both REST access and SSE proxy behavior. See [SSE realtime deployment notes](../deployment/sse-realtime).

## Typical operating workflow

1. Open Dashboard before making a production change.
2. Read risk signals and KPI strip first. If failures, queue backlog, or notification dead letters are non-zero, pause broad rollouts.
3. Check the 12-hour execution trend and status donut. If failures are clustered in the latest buckets, open Instances before triggering more work.
4. Check queue pressure. If pending/running work is accumulating, open Dispatch Queue and inspect ownership/Worker eligibility.
5. Check Worker Mesh and capability coverage. If the needed processor or script runner is absent, fix Worker deployment before editing Jobs.
6. Check HA / gateway. In Raft deployments, confirm diagnostics are not degraded and outbox totals are not growing unexpectedly.
7. Check notification delivery before relying on incident notifications.
8. After a deployment, keep the Dashboard open until trend, queue, Worker, and alert panels stabilize.

## Decision table

| Situation | Human decision | Evidence to collect |
| --- | --- | --- |
| First setup | Run one small verification job before adding more schedules. | Dashboard screenshot, Job id, instance id, Worker id, audit event. |
| Queue backlog | Do not trigger more bulk work until dispatch pressure is understood. | Queue status, oldest queued item, Worker capability coverage, instance logs. |
| Worker rollout | Compare Worker Mesh distribution and capability coverage before and after the rollout. | Worker ids, namespace/app, cluster/region, structured capability tags. |
| Notification issue | Treat provider delivery evidence as separate from job execution success. | Delivery queue status, delivery attempt id, provider response/error, related alert/job id. |
| Server HA incident | Use Dashboard for the overview, then inspect cluster diagnostics and the HA runbook. | `/api/v1/cluster/diagnostics`, outbox totals, shard/queue metrics, Kubernetes event window. |
| Production rollout | Change one dimension at a time and compare before/after. | Version diff, Dashboard before/after screenshots, audit trail, smoke result. |

## API and implementation anchors

The Dashboard is backed by these code and API surfaces:

| Surface | Anchor |
| --- | --- |
| React page | `web/src/pages/Dashboard.tsx` |
| Jobs | `/api/v1/jobs` |
| Job instances | `/api/v1/jobs/{jobId}/instances` |
| Instance stream | `/api/v1/instances/stream` |
| Workers | `/api/v1/workers` and `/api/v1/workers/stream` |
| Dispatch queue | `/api/v1/dispatch-queue` and `/api/v1/dispatch-queue/stream` |
| Cluster diagnostics | `/api/v1/cluster/diagnostics` |
| Alert delivery queue | `/api/v1/alert-delivery-attempts:queue-status` |
| Audit logs | `/api/v1/audit-logs?page_size=8` |

## Verify

- The page updates when a Worker connects, disconnects, or changes capabilities.
- The page updates when a Job instance is created or completes.
- Dispatch Queue changes are visible without a full browser refresh.
- Alert delivery dead letters or retries are visible in the notification panel.
- Recent audit actions appear after a privileged change.
- A read-only user can inspect evidence but cannot perform privileged changes through linked pages.

## Troubleshooting

| Symptom | Response |
| --- | --- |
| Dashboard loads but live panels stay stale | Check `/api/v1/instances/stream`, `/api/v1/workers/stream`, and `/api/v1/dispatch-queue/stream`; then apply the SSE proxy checklist. |
| Queue pressure is high but Workers are online | Compare Worker capability coverage with the Job processor/script binding; the online Workers may not be eligible. |
| Notification panel shows dead letters | Open Notification Center delivery attempts and inspect provider response, credentials, template rendering, and retry policy. |
| HA / gateway panel is degraded | Open the Server HA runbook and inspect `/api/v1/cluster/diagnostics`, outbox totals, and Worker gateway locality. |
| Audit activity does not show a recent change | Confirm the user/API key had permission, then inspect Audit page filters and Server logs. |
| The page looks empty | Confirm RBAC, bootstrap/login state, and whether the environment has any Jobs, Workers, instances, queue records, or audit logs yet. |

## Production checklist

- [ ] SSE stream routes work through the same proxy/Ingress used by the Web console.
- [ ] Operators know Dashboard is an overview and can drill into Instances, Workers, Dispatch Queue, Notifications, Security, and Audit for detail.
- [ ] Incident notes include Dashboard state plus the exact object ids behind the panel being investigated.
- [ ] HA deployments archive cluster diagnostics, outbox/queue metrics, and Dashboard before/after evidence during rollout.
- [ ] Notification delivery evidence is collected separately from task execution evidence.
