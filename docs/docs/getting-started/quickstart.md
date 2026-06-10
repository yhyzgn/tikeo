---
title: "Quickstart: Server + Web + Worker + SDK trigger"
description: Start Tikeo locally, bootstrap an Owner, create app-scoped SDK credentials, connect a Worker, trigger a job, and collect acceptance evidence.
---

# Quickstart: Server + Web + Worker + SDK trigger

This quickstart is the shortest path that proves Tikeo's actual architecture: the Server is healthy, the Web console can run, an app-scoped SDK API key can create and trigger a job, and a Worker connects outbound to the Worker Tunnel instead of exposing an inbound executor port.

## What you will prove

By the end, you should have evidence for:

- Server HTTP API and Worker Tunnel listeners are up.
- The first Owner can be bootstrapped in an isolated local database.
- A namespace/app/worker pool can be created.
- A service account and app-scoped SDK API key can be created.
- A Node.js Worker demo can connect with `TIKEO_WORKER_CONNECT=1` and advertise `demo.echo` as a structured SDK processor.
- The SDK Management client can create an API-scheduled job and trigger it with `executionMode=single`.
- Instance result/log evidence includes `nodejs demo echo processed`.

If you only start the Server and look at `/healthz`, you have not validated Worker Tunnel dispatch.

## Phase 0: prepare a clean local shell

```bash
cd tikeo
cargo build --bin tikeo
cd web && bun install --frozen-lockfile && cd ..
cd docs && bun install --frozen-lockfile && cd ..
cd examples/nodejs/worker-demo && bun install --frozen-lockfile && cd ../../..
```

Keep one terminal for the Server and one for commands. If you already ran older local demos, either stop those processes or use the smoke script because it creates isolated ports and DB files.

## Phase 1: start the Server

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

Expected listeners from `config/dev.toml`:

| Listener | Default | Purpose |
| --- | --- | --- |
| HTTP API | `0.0.0.0:9090` | Management API, health, readiness, metrics, OpenAPI, Web gateway surface. |
| Worker Tunnel | `0.0.0.0:9998` | gRPC/HTTP2 endpoint for outbound Worker connections. |
| Storage | `sqlite://tikeo-dev.db?mode=rwc` | Local database file. |

From another shell:

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

If `readyz` fails, inspect the Server log first. Most early failures are DB path permissions, stale port binding, or invalid config overrides.

## Phase 2: Bootstrap the first Owner

Check bootstrap status:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/auth/bootstrap | jq .
```

If `data.registrationOpen` is true, register the first Owner and export the returned bearer token for the remaining steps:

```bash
TOKEN="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/auth/bootstrap/register \
  -H 'content-type: application/json' \
  -d '{"username":"bootstrap_admin","email":"bootstrap.admin@example.com","password":"Tikeo@2026!","confirmPassword":"Tikeo@2026!"}' \
  | tee /tmp/tikeo-bootstrap.json \
  | jq -r .data.token)"
test -n "$TOKEN" && test "$TOKEN" != "null"
```

If bootstrap is already closed, login with the local Owner for this DB and export the token:

```bash
TOKEN="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"bootstrap_admin","password":"Tikeo@2026!"}' | jq -r .data.token)"
test -n "$TOKEN" && test "$TOKEN" != "null"
```

Do not use the sample password in a shared environment. It is here so a reader can reproduce local API calls against a disposable SQLite DB.

## Phase 3: open the Web console

```bash
cd web
bun run dev -- --host 0.0.0.0 --port 5173 --strictPort
```

Open `http://127.0.0.1:5173`. The console talks to the local API through the configured development path. Use the same Owner login if bootstrap is closed. The Web console is useful for visual inspection, but the quickstart continues with API commands so the acceptance evidence is repeatable.

## Phase 4: create namespace/app/pool

The Management trigger smoke script automates this, but the manual shape is:

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/namespaces \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"name":"sdk-smoke"}' | jq .

curl -fsS -X POST http://127.0.0.1:9090/api/v1/apps \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"namespace":"sdk-smoke","name":"management"}' | jq .

curl -fsS -X POST http://127.0.0.1:9090/api/v1/worker-pools \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"namespace":"sdk-smoke","app":"management","name":"nodejs-blue"}' | jq .
```

The scope values must match your Worker and Management SDK client. If a job is created under one namespace/app and the Worker registers under another, dispatch will not match even if the processor name is correct.

## Phase 5: Create an app-scoped SDK API key

Human bearer tokens and SDK API keys are different. SDK Management clients send `x-tikeo-api-key` and are intended for service/app-scoped automation.

Create a service account:

```bash
SERVICE_ACCOUNT_ID="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/management/service-accounts \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"name":"quickstart-sa","description":"Quickstart machine identity","namespace":"sdk-smoke","app":"management","workerPool":"nodejs-blue"}' \
  | jq -r .data.id)"
```

Create the SDK API key:

```bash
TIKEO_MANAGEMENT_API_KEY="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/management/api-keys \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d "{\"name\":\"quickstart-management-key\",\"namespace\":\"sdk-smoke\",\"app\":\"management\",\"service_account_id\":\"$SERVICE_ACCOUNT_ID\",\"scopes\":[\"jobs:read\",\"jobs:write\",\"instances:execute\"],\"expires_at\":null}" \
  | jq -r .data.api_key)"
export TIKEO_MANAGEMENT_API_KEY
```

Verify the key can list jobs:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/jobs -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY" | jq .code
```

## Phase 6: connect a Worker outbound

Start the Node.js demo Worker:

```bash
cd examples/nodejs/worker-demo
TIKEO_WORKER_ENDPOINT=http://127.0.0.1:9998 \
TIKEO_WORKER_CONNECT=1 \
TIKEO_WORKER_NAMESPACE=sdk-smoke \
TIKEO_WORKER_APP=management \
TIKEO_WORKER_POOL=nodejs-blue \
TIKEO_WORKER_CLUSTER=local \
TIKEO_WORKER_REGION=local \
TIKEO_WORKER_CLIENT_INSTANCE_ID=nodejs-quickstart-worker \
TIKEO_WORKER_SDK_PROCESSORS=demo.echo \
TIKEO_ENABLE_PLUGIN_SQL=0 \
TIKEO_SANDBOX_AUTO_INSTALL=0 \
bun start
```

Expected log snippets:

- `nodejs worker demo configured`
- `nodejs worker connected: worker_id=...`
- a structured capability snapshot containing `demo.echo`

The Worker is the outbound client. Do not create a business Worker Service or expose a Worker HTTP port.

## Phase 7: create and trigger a job from the SDK Management client

From a command terminal at the repository root, create a temporary Bun script in the repository so the relative source import resolves:

```bash
cat >tikeo-quickstart-trigger.ts <<'TS'
import { ManagementClient, apiJob, apiTrigger } from "./sdks/nodejs/tikeo/src/index";

const management = new ManagementClient(
  process.env.TIKEO_MANAGEMENT_ENDPOINT ?? "http://127.0.0.1:9090",
  process.env.TIKEO_MANAGEMENT_API_KEY ?? "",
  "sdk-smoke",
  "management",
);

const created = await management.createJob(apiJob("quickstart-nodejs-echo", "demo.echo"));
const instance = await management.triggerJob(created.id, apiTrigger());
console.log(JSON.stringify({ jobId: created.id, instanceId: instance.id, triggerType: instance.triggerType, executionMode: instance.executionMode }, null, 2));
TS

TIKEO_MANAGEMENT_ENDPOINT=http://127.0.0.1:9090 \
TIKEO_MANAGEMENT_API_KEY="$TIKEO_MANAGEMENT_API_KEY" \
bun tikeo-quickstart-trigger.ts
rm -f tikeo-quickstart-trigger.ts
```

If you intentionally run the script from outside the repository, install `@yhyzgn/tikeo` first and change the import to the published package name. The repository-source import above is runnable only from the repository root.

## Phase 8: Acceptance evidence

Check workers:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/workers \
  -H "authorization: Bearer $TOKEN" | jq '.data.items[] | {clientInstanceId,status,namespace,app,structuredCapabilities}'
```

Check instances and logs in the Web console, or use API endpoints from the Management OpenAPI reference:

```bash
curl -fsS 'http://127.0.0.1:9090/api/v1/instances?page=1&pageSize=20' \
  -H "authorization: Bearer $TOKEN" | jq .
```

The expected successful Worker message for the Node.js path is `nodejs demo echo processed`.

## Automated acceptance path

The maintained quick acceptance script is stronger than manual copy-paste because it isolates ports, DB, evidence files, Server logs, Worker logs, and case reports:

```bash
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```

It writes evidence under `.dev/reports/management-trigger-e2e-*`, including:

- generated TOML config
- SQLite DB path
- Server log
- Node.js Worker log
- service-account JSON
- API-key JSON
- case JSONL
- summary JSON
- final management trigger e2e report

Use this script before claiming the local SDK create+trigger path works.

## Clean shutdown

Stop the Worker and Server with `Ctrl-C`. For smoke script runs, cleanup is handled by the script trap. If you reuse `tikeo-dev.db`, remember that bootstrap is one-time for that DB; remove the DB only when you intentionally want to reset local state:

```bash
rm -f tikeo-dev.db
```

## Troubleshooting quickstart failures

| Failure | Check |
| --- | --- |
| `readyz` fails | DB URL, port conflicts, invalid config env overrides, Server log. |
| bootstrap returns closed | Existing DB already has a bootstrap admin; login instead or use a fresh DB. |
| SDK key cannot list jobs | Key missing scopes, wrong namespace/app, or using bearer token where `x-tikeo-api-key` is required. |
| Worker online but job pending | Worker namespace/app/processor does not match job; check `structuredCapabilities.sdkProcessors`. |
| Worker never appears | Wrong `TIKEO_WORKER_ENDPOINT`, Server Worker Tunnel not listening, TLS/plaintext mismatch, or demo in dry-run mode because `TIKEO_WORKER_CONNECT` is disabled. |
| Instance failed | Inspect instance logs; unsupported processor names fail closed in demos. |

## Next production question

After this passes, pick one production path:

- Docker Compose for VM-like packaging.
- Helm/Kubernetes for cluster deployment.
- Language SDK page for integrating a real application service.
- Configuration reference for TLS/mTLS, OIDC, OTel, and external DB defaults.
