---
title: Management trigger smoke runbook
description: Maintainer runbook for the operator-verified Management API create and trigger e2e smoke.
---

# Management trigger smoke runbook

`scripts/management-trigger-e2e-smoke.sh` is the contributor-facing smoke for the SDK Management API create-and-trigger path. It is intentionally heavier than a unit test: it starts a real local `tikeo` server, creates a real app-scoped machine credential, starts the Node.js demo worker over the outbound Worker Tunnel, creates a job through the Node.js SDK `ManagementClient`, triggers it with `apiTrigger`, and verifies the instance result and persisted logs.

Use this runbook when changing Management API auth, SDK helper names, Worker Tunnel registration, instance result persistence, task logs, Node.js demo worker behavior, or CI wiring around `other-cross-language-smoke`.

## What the smoke proves

The script is operator-verified by `scripts/management-trigger-e2e-smoke.sh` and the shared helpers in `deploy/smoke/lib/tikeo-smoke-lib.sh`. It verifies these real paths instead of HTTP-only checks:

- local server startup through `serve --config "$SERVER_CONFIG"` with an isolated SQLite database at `DB_PATH`;
- readiness through `tikeo_smoke_wait_for_http server "$API_URL/readyz"`;
- namespace, app, and worker pool seed data through `/api/v1/namespaces`, `/api/v1/apps`, and `/api/v1/worker-pools`;
- machine-to-machine auth through `POST /api/v1/management/service-accounts`, `POST /api/v1/management/api-keys`, and `x-tikeo-api-key`;
- a live Node.js demo worker with `TIKEO_WORKER_CONNECT=1`, `TIKEO_WORKER_ENDPOINT`, `TIKEO_WORKER_NAMESPACE`, `TIKEO_WORKER_APP`, and `TIKEO_WORKER_POOL`;
- SDK job creation and trigger through `ManagementClient`, `apiJob`, `apiTrigger`, `createJob`, and `triggerJob`;
- instance status and evidence through `/api/v1/instances/$instance_id`, `/api/v1/instances/$instance_id/logs`, `result.success`, and the log/result text `nodejs demo echo processed`.

The important case IDs are `management-scope-seed`, `management-sdk-api-key`, `management-worker-online`, `management-sdk-create-trigger`, and `management-instance-result`. A passing run finishes by calling `tikeo_smoke_finalize_report` and prints `management trigger e2e report:` plus `management trigger e2e evidence:`.

## Prerequisites

Run from the repository root. Required commands are checked by the script with `tikeo_smoke_need_cmd`:

```bash
cargo --version
bun --version
python3 --version
curl --version
```

The default ports must be free unless you override them:

```bash
# Defaults used by the script.
export TIKEO_HTTP_URL=http://127.0.0.1:19093
export TIKEO_WORKER_ENDPOINT=http://127.0.0.1:19993
```

The script can build `target/debug/tikeo` itself. For a faster local loop, build once and reuse the binary:

```bash
cargo build --bin tikeo
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```

If you want a completely self-contained run that rebuilds the server binary when needed, use:

```bash
scripts/management-trigger-e2e-smoke.sh
```

## Useful environment overrides

Use these variables to make evidence deterministic in local debugging or CI reruns:

| Variable | Default | Use |
|---|---|---|
| `TIKEO_MANAGEMENT_TRIGGER_RUN_ID` | `management-trigger-e2e-<UTC>-<pid>` | Stable run ID for report filenames. |
| `TIKEO_MANAGEMENT_TRIGGER_REPORT_DIR` | `.dev/reports/$RUN_ID` | Evidence directory override. |
| `TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER` | `1` | Set `0` after `cargo build --bin tikeo`. |
| `TIKEO_MANAGEMENT_TRIGGER_NAMESPACE` | `sdk-smoke` | Namespace seeded for the app-scoped SDK key. |
| `TIKEO_MANAGEMENT_TRIGGER_APP` | `management` | App seeded for the SDK key and job. |
| `TIKEO_MANAGEMENT_TRIGGER_WORKER_POOL` | `nodejs-blue` | Worker pool used by the demo worker. |
| `TIKEO_MANAGEMENT_TRIGGER_CLIENT_INSTANCE_ID` | `nodejs-management-trigger-smoke` | Expected worker `clientInstanceId`. |
| `TIKEO_HTTP_URL` | `http://127.0.0.1:19093` | Server API URL. |
| `TIKEO_WORKER_ENDPOINT` | `http://127.0.0.1:19993` | Worker Tunnel endpoint dialed by the demo worker. |

Example deterministic run:

```bash
export TIKEO_MANAGEMENT_TRIGGER_RUN_ID=management-trigger-e2e-local
export TIKEO_MANAGEMENT_TRIGGER_REPORT_DIR=.dev/reports/management-trigger-e2e-local
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```

## Evidence layout

A normal run writes files under `.dev/reports/management-trigger-e2e-*`. The exact path is also printed after completion:

```text
management trigger e2e report: .dev/reports/management-trigger-e2e-.../management-trigger-e2e-....json
management trigger e2e evidence: .dev/reports/management-trigger-e2e-...
```

Key files:

| File pattern | Meaning |
|---|---|
| `*-config.yml` | Generated server config with isolated SQLite and plaintext local listeners. |
| `*-server.log` | Server startup, auth, storage, and dispatch logs. |
| `*-nodejs-worker.log` | Node.js demo worker startup and Worker Tunnel logs. |
| `*-service-account.json` | Service Account creation response. |
| `*-api-key.json` | API key creation response. Keep it in `.dev/`; do not commit it. |
| `*-sdk-key-jobs-list.json` | Proof that `x-tikeo-api-key` can list jobs. |
| `*-sdk-create-trigger.json` | SDK `ManagementClient` create/trigger output. |
| `*-instance.json` | Final instance state. |
| `*-instance-logs.json` | Persisted instance logs. |
| `*-cases.jsonl` | Individual smoke case records. |
| `*.json` report | `tikeo_smoke_finalize_report` aggregate result. |
| `*-summary.json` | Evidence file index for quick artifact browsing. |

## Failure triage

Use the failure point to decide which subsystem regressed:

1. **Server never reaches `/readyz`**: inspect `*-server.log`, generated `*-config.yml`, and whether `TIKEO_HTTP_URL` or `TIKEO_WORKER_ENDPOINT` ports are already in use.
2. **Service Account or API key creation fails**: inspect `/api/v1/management/service-accounts`, `/api/v1/management/api-keys`, `x-tikeo-api-key`, auth bootstrap, and RBAC scope changes.
3. **Worker never appears online**: inspect `*-nodejs-worker.log`, `TIKEO_WORKER_CONNECT=1`, `TIKEO_WORKER_ENDPOINT`, `/api/v1/workers`, `clientInstanceId`, namespace/app/pool values, and `structuredCapabilities.normalProcessors` for `demo.echo`.
4. **SDK create/trigger fails**: inspect `sdks/nodejs/tikeo/src/management.ts`, helper names `apiJob` and `apiTrigger`, endpoint `/api/v1/jobs`, endpoint `/api/v1/jobs/{job}:trigger`, and default `executionMode=single`.
5. **Instance never succeeds**: inspect dispatcher logs, `/api/v1/instances/$instance_id`, `/api/v1/instances/$instance_id/logs`, `result.success`, and whether `nodejs demo echo processed` appears in both result/log evidence.

Do not treat a dry-run worker as a valid pass. The script deliberately avoids `TIKEO_WORKER_DRY_RUN=1`; the worker must connect outbound over the real Worker Tunnel path.
