---
title: Seed demo data
description: Step-by-step local demo data setup for Tikeo using supported API and script paths.
---

# Seed demo data

Use this page when you need a local Tikeo environment with namespaces, apps, worker pools, jobs, and plugin metadata that operators can inspect. Prefer API-based setup because it exercises the same auth, scope, audit, and validation paths used by the product.

## Prerequisites

Start from a repository checkout with the Server built and a local admin account available.

Required tools:

- `cargo`
- `curl`
- `python3`
- `jq` for the manual checks below
- `sqlite3` only if you intentionally use the direct SQLite seed path

Start the local Server in one terminal:

```bash
cargo run --bin tikeo -- serve --config config/dev.yml
```

`config/dev.yml` starts the HTTP API on port `9090`, the Worker Tunnel on port `9998`, and SQLite at `.dev/tikeo-dev.db`.

Verify the Server from another terminal:

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
```

If this is a fresh database, bootstrap the first local Owner before seeding. Follow [Quickstart](./quickstart) for the full bootstrap flow.

## Recommended path: seed through the API

Use `scripts/dev-integration-seed.sh` for repeatable local demo data. The script authenticates, then creates this topology through HTTP API calls:

| Resource type | Values created |
| --- | --- |
| Namespaces | `dev-alpha`, `dev-beta`, `dev-ops` |
| Apps | `dev-alpha/orders`, `dev-alpha/billing`, `dev-beta/analytics`, `dev-ops/automation` |
| Worker pools | `dev-alpha/orders/{boot2-blue,boot3-blue,go-blue,rust-blue,python-blue,nodejs-blue}`, `dev-alpha/billing/boot4-green`, `dev-beta/analytics/boot3-batch`, `dev-ops/automation/boot4-ops` |
| Plugin processor | `sql` with processor name `billing.sql-sync` |
| Jobs | `echo-api`, `context-api`, `bytes-api`, `report-api`, `sql-sync-api`, `workflow-step-api`, `heartbeat-api`, `fail-api` |

Run it after the Server is healthy:

```bash
TIKEO_HTTP_URL=http://127.0.0.1:9090 \
TIKEO_SMOKE_AUTH_TOKEN="$TOKEN" \
scripts/dev-integration-seed.sh
```

If you need username/password login instead, provide credentials for the owner you bootstrapped for this DB; there is no default administrator account:

```bash
TIKEO_HTTP_URL=http://127.0.0.1:9090 \
TIKEO_ADMIN_USERNAME="$TIKEO_BOOTSTRAP_USERNAME" \
TIKEO_ADMIN_PASSWORD="$TIKEO_BOOTSTRAP_PASSWORD" \
scripts/dev-integration-seed.sh
```

Do not paste production passwords or long-lived tokens into shell history. Prefer `TIKEO_SMOKE_AUTH_TOKEN` for repeatable runs, or set `TIKEO_ADMIN_USERNAME`/`TIKEO_ADMIN_PASSWORD` only in a private shell session.

## Verify the seeded data

List the main objects with the same bearer token used by the seed script:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/namespaces \
  -H "authorization: Bearer $TOKEN" | jq '.data.items[].name'

curl -fsS 'http://127.0.0.1:9090/api/v1/apps?namespace=dev-alpha' \
  -H "authorization: Bearer $TOKEN" | jq '.data[].name'

curl -fsS http://127.0.0.1:9090/api/v1/jobs \
  -H "authorization: Bearer $TOKEN" \
  | jq '.data.items[] | {namespace, app, name, processorName, processorType}'

curl -fsS http://127.0.0.1:9090/api/v1/plugins \
  -H "authorization: Bearer $TOKEN" \
  | jq '.data[] | {name, processorTypes}'
```

Expected evidence:

- `dev-alpha`, `dev-beta`, and `dev-ops` are present.
- `dev-alpha/orders` contains API jobs such as `echo-api`, `context-api`, and `bytes-api`.
- `dev-alpha/billing/sql-sync-api` uses `processorType=sql` and `processorName=billing.sql-sync`.
- The plugin list includes a processor type `sql` with processor name `billing.sql-sync`.

## Optional: start Java demo workers for the seeded topology

The seed script is designed to pair with the Java demo worker launcher:

```bash
scripts/start-java-demo-workers.sh
```

Use this only after the Server is healthy and API seed data exists. The workers connect outbound to the Worker Tunnel at port `9998`; they do not require inbound worker ports.

Verify workers:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/workers \
  -H "authorization: Bearer $TOKEN" \
  | jq '.data.items[] | {clientInstanceId,status,namespace,app,workerPool,structuredCapabilities}'
```

## Optional: direct SQLite seed for local UI inspection

`scripts/dev-seed.sh` applies `scripts/dev-seed.sql` directly to `.dev/tikeo-dev.db`. Use it only for disposable local UI inspection after migrations have created the schema:

```bash
scripts/dev-seed.sh .dev/tikeo-dev.db
```

The script checks that the database exists and that the `jobs` table is present before applying SQL. It then prints row counts for namespaces, apps, worker pools, jobs, scripts, workflows, and dispatch queue records. The direct SQL seed mirrors the same default demo topology used by the API seed and language demos, so a worker started with the documented defaults can match seeded `dev-alpha/orders` jobs. It is non-destructive by default: when `ns-dev-*` rows already exist, it exits without reapplying the upsert SQL. Use `scripts/dev-seed.sh --refresh .dev/tikeo-dev.db` or `TIKEO_DEV_SEED_REFRESH=1` only when you intentionally want to refresh the seeded demo rows.

Do not use direct SQL seeding for shared environments. It bypasses the HTTP API path and is not a substitute for validating auth, scopes, audit behavior, or runtime dispatch.

## Database compatibility seed smoke

To check the API seed path against PostgreSQL and MySQL compatibility environments, use:

```bash
scripts/db-seed-api-compat-smoke.sh
```

The smoke script starts isolated Server configs, runs `scripts/dev-integration-seed.sh`, and verifies expected jobs, worker pools, and plugin metadata. Reports are written under `.dev/reports/db-seed-compat-*`.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| `healthz` or `readyz` fails | Server log, DB path permissions, occupied ports, invalid YAML or environment overrides. |
| Seed script cannot authenticate | Bootstrap/login state for the current DB; `TIKEO_SMOKE_AUTH_TOKEN`, `TIKEO_ADMIN_USERNAME`, and `TIKEO_ADMIN_PASSWORD`. |
| API returns permission errors | The token must have management permissions for namespaces, apps, worker pools, jobs, and plugins. |
| Jobs exist but stay pending | Start a worker that advertises the matching namespace, app, worker pool, and processor name. |
| `billing.sql-sync` job fails validation | Confirm the `sql` plugin processor exists in `/api/v1/plugins`. |
| SQLite seed says tables are missing | Start the Server once so migrations create the schema, then rerun `scripts/dev-seed.sh`. |

## Cleanup

Stop demo workers and Server processes with `Ctrl-C`. For a clean local SQLite reset:

```bash
rm -f .dev/tikeo-dev.db .dev/tikeo-dev.db-shm .dev/tikeo-dev.db-wal
```

Only remove these files when you intentionally want to delete local state, including the bootstrapped Owner.

## Production checklist

Before using demo data in a shared environment:

- Replace sample usernames and passwords with environment-specific credentials.
- Use short-lived tokens or API keys with the minimum scope needed.
- Prefer API seeding over direct SQL.
- Record the seed command, Server config path, and verification output.
- Confirm workers are connected through the Worker Tunnel and not through ad hoc inbound ports.
- Remove demo namespaces, apps, jobs, plugins, and worker pools after the evaluation window.
