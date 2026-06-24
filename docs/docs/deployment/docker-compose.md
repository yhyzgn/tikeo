---
title: Docker Compose deployment
description: Deploy Tikeo with Docker Hub images, mounted config/tikeo.yml, persistent data/log/tls mounts, and SQLite/PostgreSQL/MySQL Compose stacks.
---

# Docker Compose deployment

The checked-in Compose files use published Docker Hub images by default:

- `yhyzgn/tikeo-server:latest`
- `yhyzgn/tikeo-web:latest`

They do **not** build local images. Pin `TIKEO_IMAGE` and `TIKEO_WEB_IMAGE` in `.env` when you need rollback-safe production operations.

## Configuration ownership

| Surface | What belongs here | What does not belong here |
| --- | --- | --- |
| `config/tikeo.yml` | Tikeo service behavior: listeners, structured database fields, auth, TLS, logs, cluster, retry, notification delivery. | Docker image tags, host ports, Docker volume names. |
| `.env` | Docker/Compose parameters: image tags, host ports, named volume names, database container credentials, timezone, mimalloc knobs, local worker-demo helpers. | `TIKEO__...` service overrides for normal deployment. |
| Compose `environment` | Container runtime values such as `TZ` and mimalloc knobs. | Tikeo service settings; edit `config/tikeo.yml` instead. |

## Mounted runtime paths

| Runtime path | SQLite stack | PostgreSQL stack | MySQL stack | Meaning |
| --- | --- | --- | --- | --- |
| `/config/tikeo.yml` | `./config/tikeo.yml:/config/tikeo.yml:ro` | same | same | Single formal Server config file. |
| `/config/tls` | `./config/tls:/config/tls:ro` | same | same | Optional HTTP/Worker Tunnel TLS/mTLS certs. |
| `/data` | `tikeo-data:/data` | `tikeo-data:/data` | `tikeo-data:/data` | SQLite DB path and uniform runtime data mount. |
| `/logs` | `tikeo-logs:/logs` | `tikeo-logs:/logs` | `tikeo-logs:/logs` | Optional file logs when `log_dir=/logs`. |
| DB service data | n/a | `tikeo-postgres-data:/var/lib/postgresql/data` | `tikeo-mysql-data:/var/lib/mysql` | Durable self-hosted database storage. |

## First run

```bash
cp deploy/compose/tikeo.env.example .env
# Edit .env for Docker parameters.
# Edit config/tikeo.yml for Tikeo service settings.

docker compose --env-file .env pull
docker compose --env-file .env up -d
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
open http://127.0.0.1:${TIKEO_WEB_PORT:-8080}
```

## PostgreSQL stack

Before startup, edit `config/tikeo.yml`:

```yaml
storage:
  database:
    type: postgres
    host: postgres
    port: 5432
    username: tikeo
    password: "p@ss/word:with#chars"
    database: tikeo
    params:
      sslmode: disable
```

Then run:

```bash
cp deploy/compose/tikeo.env.example .env
# Keep .env DB container credentials aligned with config/tikeo.yml.
docker compose --env-file .env -f docker-compose.postgres.yml pull
docker compose --env-file .env -f docker-compose.postgres.yml up -d
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
```

## MySQL stack

Before startup, edit `config/tikeo.yml`:

```yaml
storage:
  database:
    type: mysql
    host: mysql
    port: 3306
    username: tikeo
    password: "p@ss/word:with#chars"
    database: tikeo
```

Then run:

```bash
cp deploy/compose/tikeo.env.example .env
# Keep .env DB container credentials aligned with config/tikeo.yml.
docker compose --env-file .env -f docker-compose.mysql.yml pull
docker compose --env-file .env -f docker-compose.mysql.yml up -d
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
```

## Why structured database fields

Use `storage.database.*` fields for database settings. Passwords with `@`, `/`, `:`, or `#` are valid plain config values; Tikeo percent-encodes the generated internal URL automatically.

## Optional Prometheus

```bash
docker compose --env-file .env --profile observability up -d prometheus
curl -fsS http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/-/ready
```

## Worker connectivity

Workers dial out to the Server Worker Tunnel. For local demos use `http://127.0.0.1:9998` or `TIKEO_WORKER_TUNNEL_PUBLIC_ENDPOINT` from `.env`. Do not expose arbitrary business Worker ports.

If a Worker container advertises script runners, preinstall sandbox tools in that Worker image or mount a populated `TIKEO_SANDBOX_TOOLS_DIR`; see [Worker sandbox tools and Dockerfiles](./worker-sandbox-tools).
## Prerequisites

- Docker and Docker Compose v2 are installed for Compose examples.
- `config/tikeo.yml` exists and has been reviewed for the target database, TLS, logs, and public URLs.
- Required host directories or named volumes for `/config/tikeo.yml`, `/config/tls`, `/data`, and `/logs` are available.

## Verify

Run the documented command, then verify `/healthz`, `/readyz`, and the Web console. For database-backed stacks, also confirm the DB volume or managed database has a current backup.

## Troubleshooting

If startup fails, inspect `docker compose logs tikeo-server`, confirm `config/tikeo.yml` is mounted at `/config/tikeo.yml`, and check structured database host, port, username, password, and database values.

## Production checklist

- [ ] Images are pinned for rollback-sensitive environments.
- [ ] `/config/tikeo.yml`, `/config/tls`, `/data`, and `/logs` mounts match the deployment mode.
- [ ] Database credentials are not committed to source control.
- [ ] Worker Tunnel and SSE proxy behavior are validated before production traffic.
