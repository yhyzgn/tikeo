---
title: Management OpenAPI reference
description: Source-backed reference for the Tikeo HTTP management API, OpenAPI document route, app-scoped SDK API keys, and the job/instance endpoints used by SDK helpers.
---

# Management OpenAPI reference

This reference is curated from `crates/tikeo-server/src/http/openapi.rs`,
`crates/tikeo-server/src/http/router.rs`, and the route handlers under
`crates/tikeo-server/src/http/routes/`. The runtime OpenAPI document is exposed
at `/api-docs/openapi.json`; it is the source to use for generated clients,
API compatibility checks, and CI policy reviews. Tikeo does not expose a
browser documentation UI from the server binary, so this Docusaurus page is the
human-readable companion to the JSON document.

All business HTTP responses use the shared `ApiResponse` envelope:
`code` is the success indicator, `message` carries the human-readable result,
and `data` is always present even when it is `null`. SDK management clients
authenticate with app-scoped API keys through `x-tikeo-api-key`, typically
loaded from `TIKEO_MANAGEMENT_API_KEY`. Do not reuse browser/OIDC sessions as
machine SDK credentials.

## Source files and runtime route

| Contract | Source |
| --- | --- |
| OpenAPI assembly | `crates/tikeo-server/src/http/openapi.rs` |
| HTTP router and `/api-docs/openapi.json` | `crates/tikeo-server/src/http/router.rs` |
| Job and instance DTOs | `crates/tikeo-server/src/http/dto.rs` |
| Job/instance handlers | `crates/tikeo-server/src/http/routes/jobs.rs` |
| SDK API-key handlers | `crates/tikeo-server/src/http/sdk_api_keys.rs` |

The OpenAPI route is mounted outside `/api/v1` so operational checks can fetch
`/api-docs/openapi.json` without guessing the API version prefix. Management
operations themselves stay under `/api/v1` and remain subject to the authn/authz
layer selected by the handler.

## SDK management authentication boundary

The SDK create/trigger flow is machine-to-machine. Administrators create a
Service Account and issue a scoped SDK API key, then workers or automation send
that key in `x-tikeo-api-key`. The key is scoped to a namespace/app and optional
worker-pool boundary; it is not a user session token and should not be stored in
browser state.

Default helper semantics are intentionally narrow:

- Create helpers build a `CreateJobRequest` with an API schedule.
- Trigger helpers build a `TriggerJobRequest` with `triggerType=api`.
- The default trigger path uses `executionMode=single`.
- Broadcast requires an explicit broadcast helper and `broadcastSelector`.

## Post api v1 jobs

`POST /api/v1/jobs` creates a job definition and returns it in the shared
`ApiResponse` envelope. SDK helper names differ by language, but they all map
to this endpoint when creating an API-triggered processor job:
`ManagementCreateJobRequest::api`, `APIJob`, `CreateJobRequest.api`,
`api_job`, and `apiJob`.

The request body is represented by `CreateJobRequest`. For SDK helper usage the
important fields are the namespace/app scope carried by the client, the job
name, the processor name, and `scheduleType=api`. Server-side validation still
applies normal RBAC, scope, worker-pool, schedule, canary, and script-binding
rules; the helper does not bypass the scheduler state machine.

## Post api v1 jobs job trigger

`POST /api/v1/jobs/{job}:trigger` creates a job instance for manual/API
execution. The OpenAPI path is documented as `/api/v1/jobs/{job}:trigger` even
though Axum internally parses the action suffix through a route-compatible
handler. SDK default trigger helpers map here with `triggerType=api` and
`executionMode=single`.

Use the broadcast helper only when the intended behavior is fan-out across all
matching workers. Broadcast payloads set `executionMode=broadcast` and include a
`broadcastSelector`, making code review and audit logs distinguish fan-out from
the single-worker default.

## Get api v1 instances instance

`GET /api/v1/instances/{instance}` returns the current instance summary after a
create/trigger call. SDK examples use this endpoint, directly or through helper
methods, to poll for terminal state such as `succeeded` or `failed`. The
response includes the trigger type, execution mode, result summary, and
scheduler metadata needed to confirm that the Server dispatched through the
Worker Tunnel rather than executing user code itself.

When validating an API-trigger smoke, use this endpoint after
`/api/v1/jobs/{job}:trigger` to confirm that the created instance belongs to the
expected namespace/app and that the final result came from a real worker.

## Get api v1 instances instance logs

`GET /api/v1/instances/{instance}/logs` returns persisted task log records.
Logs are written by workers over the Worker Tunnel with assignment-token
authority and then surfaced through the HTTP management API. This endpoint is
the simplest user-facing proof that worker-side code processed a task.

For end-to-end evidence, pair instance polling with this logs endpoint and look
for processor-specific messages. The Management API trigger smoke does exactly
that: it verifies the instance result and then verifies worker log evidence.

## Endpoint checklist for SDK docs

SDK pages should link helper behavior to the exact reference anchors above:

- Create job helper → [`POST /api/v1/jobs`](./management-openapi#post-api-v1-jobs)
- Trigger helper → [`POST /api/v1/jobs/{job}:trigger`](./management-openapi#post-api-v1-jobs-job-trigger)
- Instance polling → [`GET /api/v1/instances/{instance}`](./management-openapi#get-api-v1-instances-instance)
- Log inspection → [`GET /api/v1/instances/{instance}/logs`](./management-openapi#get-api-v1-instances-instance-logs)

Keep those links source-backed. If a new SDK helper is documented, first verify
that the helper exists in committed SDK source and that its serialized payload
matches the OpenAPI request contract.
