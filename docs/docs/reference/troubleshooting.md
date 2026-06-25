---
title: Troubleshooting
description: Operator-first checklist for local Tikeo startup, auth, Worker Tunnel, dispatch, realtime streams, alerts, notifications, and deployment failures.
---

# Troubleshooting

Use this runbook from the outside in: confirm the process is running, confirm the HTTP API is healthy, confirm auth, then check workers, dispatch, logs, and integration-specific queues. Do not change code or production configuration until you have captured the failing command and the relevant Server log lines.

## Collect the first evidence

From the repository root:

```bash
pwd
git rev-parse --short HEAD
cargo run --bin tikeo -- serve --config config/dev.yml
```

From a second shell:

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

Keep these details with any incident report:

- Server commit and config path
- database backend and database URL type, with credentials redacted
- auth method used: local bearer token, SDK API key, or OIDC session
- Worker SDK language and version/path
- failing route and full HTTP status
- relevant instance ID, job ID, worker ID, audit ID, or delivery attempt ID

## Server does not start

### Check command and config

```bash
cargo run --bin tikeo -- serve --config config/dev.yml
```

`config/dev.yml` defaults to:

| Setting | Local value |
| --- | --- |
| HTTP API | `config/dev.yml` binds all interfaces on port `9090`; use `http://127.0.0.1:9090` from local clients |
| Worker Tunnel | `config/dev.yml` binds all interfaces on port `9998`; use `http://127.0.0.1:9998` from local workers |
| Storage | `sqlite://.dev/tikeo-dev.db?mode=rwc` |
| Local login | enabled |

### Common causes

| Symptom | Check | Fix |
| --- | --- | --- |
| Address already in use | Another Server or test process owns port `9090` or `9998`. | Stop the old process or use a separate config with different ports. |
| Config parse error | YAML syntax or invalid environment override. | Re-run with `config/dev.yml`, then reapply overrides one by one. |
| SQLite open error | DB file path permissions or stale directory. | Start from a writable directory or remove only disposable local DB files. |
| PostgreSQL/MySQL connect error | DB host, credentials, TLS, or database not created. | Verify with the native DB client before restarting Tikeo. |
| TLS/plaintext mismatch | Client uses HTTPS or mTLS against a plaintext local listener, or the reverse. | Align `transport_security.http` and `transport_security.worker_tunnel` with client endpoints. |

## Health or readiness check fails

Run:

```bash
curl -i http://127.0.0.1:9090/healthz
curl -i http://127.0.0.1:9090/readyz
```

Interpretation:

- `healthz` failing means the HTTP listener is not reachable or the process is down.
- `readyz` failing means the process is up but storage, migration, or required runtime readiness failed.

Next checks:

1. Read the Server log from startup to the first readiness failure.
2. Confirm the configured DB exists and is writable.
3. Confirm migrations completed.
4. Confirm no environment variables override the YAML unexpectedly.
5. For container or Kubernetes runs, compare the pod/container readiness probe path with `/readyz`.

## Auth and bootstrap problems

Check bootstrap status:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/auth/bootstrap | jq .
```

If registration is open, create the first local Owner using the flow in [Quickstart](../getting-started/quickstart). If registration is closed, login with an existing local admin by reading credentials from your private shell environment:

```bash
TOKEN="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d "$(python3 - <<'PY'
import json, os
print(json.dumps({
    "username": os.environ["TIKEO_ADMIN_USERNAME"],
    "password": os.environ["TIKEO_ADMIN_PASSWORD"],
}))
PY
)" | jq -r .data.token)"
test -n "$TOKEN" && test "$TOKEN" != "null"
```

| Symptom | Check |
| --- | --- |
| Bootstrap is closed | This DB already has an Owner. Login or reset only the disposable local DB. |
| Bearer token route returns unauthorized | Missing `Authorization: Bearer ...`, expired token, or wrong local DB. |
| SDK route returns unauthorized | SDK Management clients must use `x-tikeo-api-key`, not a human bearer token. |
| Scope error mentions namespace/app | API key or service account scope does not cover the target namespace/app. |

## Worker is invisible

Workers connect outbound to the Worker Tunnel. They do not need inbound business HTTP ports.

Check Server listener and worker list:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/workers \
  -H "authorization: Bearer $TOKEN" \
  | jq '.data.items[] | {clientInstanceId,status,namespace,app,workerPool,structuredCapabilities}'
```

Common causes:

| Symptom | Check | Fix |
| --- | --- | --- |
| Worker never appears | `TIKEO_WORKER_ENDPOINT`, local tunnel port `9998`, TLS/plaintext match. | Use `TIKEO_WORKER_ENDPOINT=http://127.0.0.1:9998` for local plaintext. |
| Worker appears offline | Process stopped, heartbeat expired, or network path closed. | Restart worker and inspect worker logs. |
| Worker has no useful processors | `structuredCapabilities` does not include the processor required by the job. | Set the demo worker processor env, for example `TIKEO_WORKER_NORMAL_PROCESSORS=demo.echo`. |
| Stale worker blocks diagnosis | Old worker sessions remain in history. | Compare live status and `clientInstanceId`; inspect `/api/v1/workers/history`. |

## Job stays pending or queued

Start with scheduling advice and queue state:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/jobs/${JOB_ID}/scheduling-advice \
  -H "authorization: Bearer $TOKEN" | jq .

curl -fsS http://127.0.0.1:9090/api/v1/dispatch-queue \
  -H "authorization: Bearer $TOKEN" | jq .
```

Check these fields together:

- job namespace/app
- job `processorName` and optional `processorType`
- worker namespace/app/worker pool
- worker structured capability list
- queue depth and worker pool quota
- canary target job if canary routing is enabled

Do not fix pending jobs by adding broad wildcard worker capabilities. Fix the job binding, plugin processor registration, script runtime, worker pool assignment, or worker runtime installation.

## Instance failed or logs are missing

Fetch the instance, attempts, and logs:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/instances/${INSTANCE_ID} \
  -H "authorization: Bearer $TOKEN" | jq .

curl -fsS http://127.0.0.1:9090/api/v1/instances/${INSTANCE_ID}/attempts \
  -H "authorization: Bearer $TOKEN" | jq .

curl -fsS http://127.0.0.1:9090/api/v1/instances/${INSTANCE_ID}/logs \
  -H "authorization: Bearer $TOKEN" | jq .
```

Common failure classes:

| Failure | What to inspect |
| --- | --- |
| Assignment token rejection | Worker and Server logs; stale assignment, wrong worker, or duplicate result. |
| Script runtime unavailable | Script backend installation, sandbox auto-install setting, worker capability advertisement. |
| Script governance denied | Approval, signature metadata, digest, URL/file/secret grants, timeout, output limit. |
| Plugin processor rejected | `/api/v1/plugins` contains the processor type and allowed processor name. |
| Broadcast partial failure | Child attempts and per-worker logs. |

## Realtime UI does not update

Tikeo Web uses Server-Sent Events for realtime console updates. Check the stream directly with curl before changing frontend code:

```bash
curl -N http://127.0.0.1:9090/api/v1/workers/stream \
  -H "authorization: Bearer $TOKEN" \
  -H 'Accept: text/event-stream'
```

Expected behavior: the request stays open and emits `workers.snapshot` events when the visible worker snapshot changes.

If the direct local stream works but the browser does not:

- Check browser DevTools Network for `text/event-stream`.
- Check reverse proxy buffering, gzip, caching, and idle timeouts.
- Confirm query-token fallback is not stripped when Web uses `EventSource`.
- See [SSE realtime deployment notes](../deployment/sse-realtime).

## Inbound webhook trigger fails

Route:

```text
POST /api/v1/events/webhooks/{job}:trigger
```

Checks:

| Symptom | Check |
| --- | --- |
| `job not found` | The `{job}` path value must be the job ID, with `:trigger` suffix in the route. |
| Forbidden scope | Token scope must allow the job namespace/app and `instances:execute`. |
| Signature required | If any signature field is present, provide `secretRef`, `signature`, `timestamp`, and `nonce`. |
| Timestamp out of window | Timestamp must be within 300 seconds of Server time. |
| Nonce replay detected | Use a unique nonce per signed event. |
| Secret unresolved | `secretRef` must resolve from the Server environment, for example an `env:` reference. |

Inbound webhooks start jobs. They are not Notification Center outbound channels.

## Outbound notifications or alerts do not deliver

Notification Center and Alerts have separate route families and queues.

Notification Center checks:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/notification-channel-types \
  -H "authorization: Bearer $TOKEN" | jq .

curl -fsS http://127.0.0.1:9090/api/v1/notification-delivery-attempts:queue-status \
  -H "authorization: Bearer $TOKEN" | jq .
```

Alert checks:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/alert-delivery-attempts:queue-status \
  -H "authorization: Bearer $TOKEN" | jq .
```

Troubleshoot in this order:

1. Channel or alert rule is enabled.
2. Provider type is registered.
3. Provider target is configured through `secretRefs`, not raw values in docs or screenshots.
4. Server environment contains the referenced variables.
5. Retry worker is enabled in `notification_delivery` or `alert_retry` config.
6. Queue status and DLQ explain the last delivery state.

## Docker or image build is slow

Server image validation compiles the Rust workspace and is slower than Web/docs image validation on a cold runner. Before rerunning a full image build, check the faster commands:

```bash
cargo build --workspace --all-features
(cd web && bun run build)
(cd docs && bun run docs:build)
```

If CI is slow but local builds pass, compare cache keys, Docker BuildKit status, and whether the runner is rebuilding the full Rust dependency graph.

## Kubernetes or proxy deployment failures

Use health probes and direct API paths before debugging the UI:

```bash
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

For clusters:

- Read pod logs before changing probes.
- Use `/readyz` for readiness and `/healthz` for liveness.
- Do not use SSE stream routes as probes.
- Confirm DB secrets, TLS settings, service ports, and ingress path rewrites.
- For Worker Tunnel, confirm the worker can reach the tunnel service and uses the same TLS mode as the Server.

## Escalation package

When handing an issue to another operator, include:

- exact command that failed
- exact HTTP status and response body, with secrets redacted
- Server config file path and relevant overrides
- Server startup log and failure log window
- health/readiness output
- instance/job/worker/audit/delivery IDs
- worker SDK language and demo path if a worker is involved
- whether the failure reproduces through direct `127.0.0.1` access or only through a proxy/Ingress

## Prerequisites

Use the setup, authentication, and access requirements described in this page before running any command. For local examples, start the Server with `config/dev.yml`, use `127.0.0.1` as the client host, and keep tokens in shell variables rather than pasted into files.

## Verify

After following the page, verify the result with the documented API, UI, build, smoke, or deployment checks. A valid verification includes the command that was run, the route or file that was inspected, and the observed status or artifact.

## Troubleshooting

When a step fails, first capture the exact command, response status, and Server log window. Then check authentication, namespace/app scope, Worker eligibility, storage readiness, and proxy behavior before changing production configuration.

## Production checklist

- [ ] Secrets are referenced through environment or platform secret mechanisms and are not written into examples.
- [ ] Commands have been adapted from local `127.0.0.1` to the real host, TLS, and authentication model.
- [ ] Rollback and evidence collection are documented for the changed surface.
- [ ] Operators can repeat the verification without private shell history or hidden state.
