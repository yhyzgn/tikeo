---
title: SDK and API integration guide
description: Step-by-step integration guide for connecting application Workers, creating API jobs, triggering tasks, wiring notifications, and verifying execution evidence.
keywords: [tikeo sdk integration, management api, worker tunnel, api trigger, notification integration]
---

# SDK and API integration guide

This page is for application teams integrating Tikeo into a service. It tells you what to build, which credentials to request, how Workers connect, how jobs are created and triggered, and what evidence proves the integration works.

## Integration model

A typical integration has two independent clients:

| Client | Credential | Direction | Purpose |
| --- | --- | --- | --- |
| Worker SDK | Worker identity/config | Worker → Worker Tunnel | Registers processors, receives `DispatchTask`, streams `TaskLog`, returns `TaskResult`. |
| Management SDK/API client | `x-tikeo-api-key` | Application → Server HTTP API | Creates API-triggered jobs, triggers jobs, reads instances/logs. |

Do not use the Worker Tunnel for management calls, and do not use human Web session tokens in application services.

## Before writing code

Ask your platform operator for:

- Server HTTP base URL, for example `https://tikeo.example.com`.
- Worker Tunnel endpoint, for example `https://tikeo-worker.example.com` or `http://tikeo-server:9998` inside a private network.
- `namespace`, `app`, and `workerPool` naming convention.
- One app-scoped SDK API key for `x-tikeo-api-key`.
- Expected processor names and payload schema.
- Notification channel/template/policy expectations if task status must notify humans.

## Step 1: choose a language SDK

| Language | Page | Best fit |
| --- | --- | --- |
| Rust | [Rust SDK](../sdks/rust) | Native Rust services and high-throughput Workers. |
| Go | [Go SDK](../sdks/go) | Small static Worker services and platform agents. |
| Java/Spring Boot | [Java SDK and Spring Boot](../sdks/java-spring-boot) | Spring services and annotation-driven processors. |
| Python | [Python SDK](../sdks/python) | Data/automation jobs and Python service teams. |
| Node.js | [Node.js SDK](../sdks/nodejs) | TypeScript/JavaScript services and quick demos. |

Each SDK page documents dependency coordinates, WorkerConfig defaults, minimal Worker code, Management client credentials, and a live verification runbook.

## Step 2: implement a Worker processor

Your Worker should declare the exact processors it can run. Example processor naming convention:

```text
namespace: billing
app: invoices
workerPool: default
processorName: invoice.send-reminder
```

The Worker connects outbound and advertises capabilities. Keep capability declarations honest: do not advertise a processor, script backend, or plugin type until that Worker can actually execute it and report a safe failure.

## Step 3: create an API-triggered job

Use the Management SDK or raw HTTP API. The important fields are:

- `triggerType=api`
- `executionMode=single` for one selected Worker result, or broadcast helpers when every matching Worker should run.
- Processor name matching what Workers advertise.
- Namespace/app scope matching the app-scoped API key.

HTTP shape:

```bash
curl -fsS -X POST "$TIKEO_URL/api/v1/jobs" \
  -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY" \
  -H 'content-type: application/json' \
  -d '{
    "name":"send invoice reminder",
    "namespace":"billing",
    "app":"invoices",
    "processorType":"sdk",
    "processorName":"invoice.send-reminder",
    "triggerType":"api",
    "executionMode":"single",
    "enabled":true
  }' | jq .
```

For exact typed helper names, see the SDK pages: `ManagementClient`, `NewManagementClient`, `HttpTikeoJobClient`, `apiJob`, `apiTrigger`, `broadcastApiTrigger`, and `BroadcastSelectorRequest`.

## Step 4: trigger and inspect an instance

```bash
INSTANCE_ID="$(curl -fsS -X POST "$TIKEO_URL/api/v1/jobs/$JOB_ID:trigger" \
  -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY" \
  -H 'content-type: application/json' \
  -d '{"payload":{"invoiceId":"inv_123"}}' | jq -r .data.instanceId)"

curl -fsS "$TIKEO_URL/api/v1/instances/$INSTANCE_ID" \
  -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY" | jq .

curl -fsS "$TIKEO_URL/api/v1/instances/$INSTANCE_ID/logs" \
  -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY" | jq .
```

Expected evidence:

- Instance status becomes `success` or a deliberate failure with clear logs.
- Logs include Worker-emitted task log lines.
- Worker table shows the Worker online and recently heartbeating.
- Audit logs show job or trigger operations where applicable.

## Broadcast integration

Broadcast jobs are for “run on every matching Worker” cases. They require deliberate selector design:

- Match by namespace/app/workerPool.
- Optionally match labels/tags/capabilities.
- Expect multiple child attempts and per-Worker result rows.
- Use `broadcastSelector` and SDK helper `BroadcastSelectorRequest` instead of custom JSON conventions.

Do not use broadcast when the business operation must run exactly once.

## Notification integration

For task success/failure/always notifications, do not hardcode provider calls inside Workers. Use Notification Center:

1. Operator creates a channel with provider credentials.
2. Operator creates or selects a template.
3. Job owner creates a job notification binding for success/failure/always.
4. Runtime materializes message payload with fields such as `jobId`, `instanceId`, `status`, `operatorName`, `executionMode`, and `logsUrl`.
5. Operator verifies delivery attempts and message trace.

See [Notifications](../user-guide/notifications) and [Notification Center reference](../reference/notification-center).

## Error handling contract

Worker processors should:

- Validate payloads at the boundary and return actionable errors.
- Emit task logs before and after external calls.
- Avoid logging secrets, tokens, provider URLs, passwords, or authorization headers.
- Use idempotency keys in downstream systems if Tikeo retry can call the processor again.
- Treat cancellation and timeout as normal operational states.

Management clients should:

- Store `TIKEO_MANAGEMENT_API_KEY` in secret management.
- Retry only idempotent reads or explicitly idempotent triggers.
- Record `instanceId` in business logs so operators can correlate evidence.

## Local integration smoke

The fastest complete integration check is:

```bash
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```

It starts an isolated environment, creates app-scoped credentials, starts the Node.js Worker demo with `TIKEO_WORKER_CONNECT=1`, creates and triggers an API job, and stores evidence under `.dev/reports/management-trigger-e2e-*`.

## Prerequisites

- A reachable Server HTTP API and Worker Tunnel endpoint.
- Namespace/app/workerPool approved by the platform owner.
- App-scoped SDK API key in `TIKEO_MANAGEMENT_API_KEY`.
- At least one Worker SDK dependency installed.
- A payload schema and processor naming convention.

## Verify

A successful integration has these artifacts:

```bash
curl -fsS "$TIKEO_URL/api/v1/jobs/$JOB_ID" -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY"
curl -fsS "$TIKEO_URL/api/v1/instances/$INSTANCE_ID" -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY"
curl -fsS "$TIKEO_URL/api/v1/instances/$INSTANCE_ID/logs" -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY"
```

The Web console should show the Worker online, the job enabled, the instance completed, and logs attached to the instance.

## Troubleshooting

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| API key returns 401/403 | Wrong key, wrong app scope, disabled service account | Reissue app-scoped key and confirm namespace/app. |
| Job triggers but stays pending | No Worker advertises matching processor/scope | Check Worker capabilities and namespace/app/workerPool. |
| Worker connects but receives no tasks | Processor name mismatch or disabled job | Compare job processor fields with Worker registration. |
| Logs missing | Worker does not stream task logs or failed before handler | Add log emission around handler start/end. |
| Duplicate downstream effect | Retry without idempotency | Add idempotency key using `instanceId` or business key. |

## Production checklist

- [ ] Worker and Management clients use separate credentials/config.
- [ ] Processor names and payload schemas are documented with the service team.
- [ ] API jobs use `triggerType=api` and expected `executionMode`.
- [ ] Instance ID is stored in business logs.
- [ ] Worker logs do not contain secrets.
- [ ] Notification bindings are configured through Notification Center, not ad-hoc provider code.
