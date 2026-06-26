---
title: Docker Compose deployment
description: Complete Docker Compose and docker run deployment guide for Tikeo with SQLite, PostgreSQL, MySQL, Web, Prometheus, persistent mounts, and SSE-safe proxying.
---

# Docker Compose deployment

This page is the operator runbook for running Tikeo from the published Docker Hub images. It covers the checked-in Docker Compose stacks and a lower-level `docker run` path for hosts where Compose is not available.

The Compose files use these images by default:

- `yhyzgn/tikeo-server:latest`
- `yhyzgn/tikeo-web:latest`
- `prom/prometheus:v3.0.1` when the `observability` profile is enabled

They do **not** build local images. For production, pin `TIKEO_IMAGE` and `TIKEO_WEB_IMAGE` in `.env` to a released version instead of using `latest`.

## Deployment choices at a glance

| Mode | File or command | Storage | Includes Web | Includes Prometheus | Best for |
| --- | --- | --- | --- | --- | --- |
| SQLite Compose | `docker-compose.yml` | `/data/tikeo.db` in `tikeo-data` | yes | optional profile | single-node demos, small installs, first validation |
| PostgreSQL Compose | `docker-compose.postgres.yml` | `tikeo-postgres` service and `tikeo-postgres-data` | yes | optional profile | shared environments that want PostgreSQL on the same host |
| MySQL Compose | `docker-compose.mysql.yml` | `tikeo-mysql` service and `tikeo-mysql-data` | yes | optional profile | shared environments standardized on MySQL |
| Compose + Prometheus | add `--profile observability` | same as selected stack | yes | `tikeo-prometheus` | local SLO and metrics validation |
| Docker run | explicit `docker run` commands | mounted path or external DB | optional | optional | hosts without Compose, debugging container wiring |

## Naming, networking, and image policy

The service keys and container names are intentionally stable:

- `tikeo-server`
- `tikeo-web`
- `tikeo-prometheus`
- `tikeo-postgres`
- `tikeo-mysql`

The names above are also Docker DNS names inside the Compose network. The Web nginx config sends API and SSE traffic to `http://tikeo-server:9090`, and Prometheus scrapes `tikeo-server:9090`. Do not rename only one side of that relationship; if you provide a custom Compose override, keep nginx and Prometheus upstreams aligned.

The image names are independent from service names. Keep `yhyzgn/tikeo-server` and `yhyzgn/tikeo-web` unless you deliberately publish and operate a private mirror.

## Configuration ownership

| Surface | What belongs here | What does not belong here |
| --- | --- | --- |
| `config/tikeo.yml` | Tikeo service behavior: HTTP listener, `server.worker_tunnel_addr`, `storage.database.*`, auth, TLS, logs, cluster settings, retry settings, notification public URL. | Docker image tags, host ports, Compose volume names. |
| `.env` copied from `deploy/compose/tikeo.env.example` | Docker parameters: image tags, host ports, named volume names, database container credentials, timezone, mimalloc knobs, local worker-demo helper values. | Normal service overrides such as `TIKEO__STORAGE__DATABASE__HOST`; edit the YAML file for regular deployments. |
| Compose `environment` | Container runtime values such as `TZ` and mimalloc policy. | Tikeo business configuration. |
| nginx `web/nginx/default.conf` | Web static serving, API reverse proxying, SSE-safe proxy behavior. | Server storage, auth, TLS, or worker scheduling configuration. |

Important configuration files:

| Path | Purpose |
| --- | --- |
| `docker-compose.yml` | Server + Web + optional Prometheus, SQLite by default. |
| `docker-compose.postgres.yml` | Server + Web + PostgreSQL + optional Prometheus. |
| `docker-compose.mysql.yml` | Server + Web + MySQL + optional Prometheus. |
| `deploy/compose/tikeo.env.example` | Copy to `.env`; controls image tags, host ports, volumes, DB container credentials, and local worker helper variables. |
| `config/tikeo.yml` | Mounted into the Server container as `/config/tikeo.yml`; this is the formal Tikeo Server config. |
| `observability/prometheus/prometheus.yml` | Prometheus scrape and rule-file config for the Compose observability profile. |
| `observability/prometheus/tikeo-recording-rules.yml` | Recording rules used by dashboards and alerts. |
| `observability/prometheus/tikeo-alert-rules.yml` | Alerting rules loaded by Prometheus. |

## Runtime mounts and persisted data

| Runtime path | SQLite stack | PostgreSQL stack | MySQL stack | Meaning |
| --- | --- | --- | --- | --- |
| `/config/tikeo.yml` | `./config/tikeo.yml:/config/tikeo.yml:ro` | same | same | Server configuration file. |
| `/config/tls` | `./config/tls:/config/tls:ro` | same | same | Optional HTTP and Worker Tunnel TLS or mTLS material. |
| `/data` | `tikeo-data:/data` | `tikeo-data:/data` | `tikeo-data:/data` | SQLite database path and common runtime data mount. |
| `/logs` | `tikeo-logs:/logs` | `tikeo-logs:/logs` | `tikeo-logs:/logs` | File logs when `observability.logging.channels.file.path` is `/logs`. |
| DB service data | not used | `tikeo-postgres-data:/var/lib/postgresql/data` | `tikeo-mysql-data:/var/lib/mysql` | Durable self-hosted database storage. |

Back up the volume that owns the active database. For SQLite that is `tikeo-data`; for PostgreSQL and MySQL it is the database service volume.

## Prepare `.env`

Create `.env` once per deployment directory:

```bash
cp deploy/compose/tikeo.env.example .env
```

Review these values before the first startup:

| Variable | Default | Meaning |
| --- | --- | --- |
| `TIKEO_IMAGE` | `yhyzgn/tikeo-server:latest` | Server image. Pin to a release tag for production. |
| `TIKEO_WEB_IMAGE` | `yhyzgn/tikeo-web:latest` | Web console image. Pin to the same release train as the Server. |
| `TIKEO_HTTP_PORT` | `9090` | Host port mapped to Server HTTP, OpenAPI, metrics, health checks, and Web upstream. |
| `TIKEO_WORKER_TUNNEL_PORT` | `9998` | Host port mapped to the Worker Tunnel; Workers dial this endpoint. |
| `TIKEO_WEB_PORT` | `8080` | Host port for the Web console. |
| `TIKEO_PROMETHEUS_PORT` | `9091` | Host port for Prometheus when observability is enabled. |
| `TIKEO_DATA_VOLUME` | `tikeo-data` | Server data volume. Owns SQLite data in the SQLite stack. |
| `TIKEO_LOGS_VOLUME` | `tikeo-logs` | Server log volume. |
| `TIKEO_POSTGRES_*` | see example file | PostgreSQL container database, user, password, host port, and volume. Keep credentials aligned with `config/tikeo.yml`. |
| `TIKEO_MYSQL_*` | see example file | MySQL container database, user, passwords, host port, and volume. Keep credentials aligned with `config/tikeo.yml`. |
| `TIKEO_WORKER_TUNNEL_PUBLIC_ENDPOINT` | `http://127.0.0.1:9998` | Helper value for local worker demos; not consumed by the Server container. |

## Prepare `config/tikeo.yml`

The Server container runs:

```bash
tikeo serve --config /config/tikeo.yml
```

Edit `config/tikeo.yml` before startup:

- `server.listen_addr` controls the Server listener inside the container.
- `server.worker_tunnel_addr` controls where the Worker Tunnel listens inside the container.
- `storage.database.*` selects SQLite, PostgreSQL, MySQL, or another supported database.
- `notification_delivery.public_console_base_url` should be the browser-reachable Web console URL, for example `http://127.0.0.1:8080` for a local Compose install.
- `transport_security.*` points to files under `/config/tls` when HTTP or Worker Tunnel TLS/mTLS is enabled.
- `observability.logging.channels.file.path` should stay `/logs` if you want file logs in the `tikeo-logs` volume.

Use structured database fields rather than URL strings. Passwords containing `@`, `/`, `:`, or `#` are valid YAML values, and Tikeo encodes the generated internal database URL safely.

## SQLite Compose runbook

Use this mode when a single Server container should own the database file.

1. Prepare configuration:

   ```bash
   cp deploy/compose/tikeo.env.example .env
   # Optional: edit .env to pin TIKEO_IMAGE, TIKEO_WEB_IMAGE, and host ports.
   # Keep config/tikeo.yml storage.database.type as sqlite.
   ```

2. Validate the rendered Compose file:

   ```bash
   docker compose --env-file .env -f docker-compose.yml config --quiet
   ```

3. Pull and start. Because `docker-compose.yml` is the default Compose file name, the short form and explicit `-f docker-compose.yml` form are equivalent:

   ```bash
   docker compose --env-file .env pull
   docker compose --env-file .env up -d
   ```

   ```bash
   docker compose --env-file .env -f docker-compose.yml pull
   docker compose --env-file .env -f docker-compose.yml up -d
   ```

4. Verify the business endpoints and console:

   ```bash
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/healthz
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/api/v1/metrics/summary
   curl -fsS http://127.0.0.1:${TIKEO_WEB_PORT:-8080}/
   ```

5. Inspect containers and logs:

   ```bash
   docker compose --env-file .env -f docker-compose.yml ps
   docker compose --env-file .env -f docker-compose.yml logs -f tikeo-server
   ```

6. Stop without deleting data:

   ```bash
   docker compose --env-file .env -f docker-compose.yml stop
   ```

7. Remove containers but keep named volumes:

   ```bash
   docker compose --env-file .env -f docker-compose.yml down
   ```

## PostgreSQL Compose runbook

Use this mode when the database should be a PostgreSQL container managed by the same Compose project.

1. Prepare `.env` and set database credentials:

   ```bash
   cp deploy/compose/tikeo.env.example .env
   # Edit TIKEO_POSTGRES_DB, TIKEO_POSTGRES_USER, TIKEO_POSTGRES_PASSWORD,
   # TIKEO_POSTGRES_PORT, and TIKEO_POSTGRES_DATA_VOLUME as needed.
   ```

2. Replace `storage.database` in `config/tikeo.yml` with matching values:

   ```yaml
   storage:
     database:
       type: postgres
       host: tikeo-postgres
       port: 5432
       username: tikeo
       password: "change-me"
       database: tikeo
       params:
         sslmode: disable
   ```

   The host must be `tikeo-postgres` for the checked-in Compose network.

3. Validate, pull, and start:

   ```bash
   docker compose --env-file .env -f docker-compose.postgres.yml config --quiet
   docker compose --env-file .env -f docker-compose.postgres.yml pull
   docker compose --env-file .env -f docker-compose.postgres.yml up -d
   ```

4. Verify Server readiness and database health:

   ```bash
   docker compose --env-file .env -f docker-compose.postgres.yml ps
   docker compose --env-file .env -f docker-compose.postgres.yml logs --tail=80 tikeo-postgres
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/api/v1/metrics/summary
   ```

5. Operate backups against `tikeo-postgres-data` or by running `pg_dump` from a controlled maintenance container. Do not rely on the Server `/data` volume for PostgreSQL backups.

## MySQL Compose runbook

Use this mode when the database should be a MySQL container managed by the same Compose project.

1. Prepare `.env` and set database credentials:

   ```bash
   cp deploy/compose/tikeo.env.example .env
   # Edit TIKEO_MYSQL_DATABASE, TIKEO_MYSQL_USER, TIKEO_MYSQL_PASSWORD,
   # TIKEO_MYSQL_ROOT_PASSWORD, TIKEO_MYSQL_PORT, and TIKEO_MYSQL_DATA_VOLUME.
   ```

2. Replace `storage.database` in `config/tikeo.yml` with matching values:

   ```yaml
   storage:
     database:
       type: mysql
       host: tikeo-mysql
       port: 3306
       username: tikeo
       password: "change-me"
       database: tikeo
   ```

   The host must be `tikeo-mysql` for the checked-in Compose network.

3. Validate, pull, and start:

   ```bash
   docker compose --env-file .env -f docker-compose.mysql.yml config --quiet
   docker compose --env-file .env -f docker-compose.mysql.yml pull
   docker compose --env-file .env -f docker-compose.mysql.yml up -d
   ```

4. Verify Server readiness and database health:

   ```bash
   docker compose --env-file .env -f docker-compose.mysql.yml ps
   docker compose --env-file .env -f docker-compose.mysql.yml logs --tail=80 tikeo-mysql
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/api/v1/metrics/summary
   ```

5. Operate backups against `tikeo-mysql-data` or by running `mysqldump` from a controlled maintenance container. Do not rely on the Server `/data` volume for MySQL backups.

## Add Prometheus to any Compose stack

Prometheus is behind the `observability` profile in all three main Compose files.

SQLite stack:

```bash
docker compose --env-file .env -f docker-compose.yml --profile observability up -d tikeo-prometheus
curl -fsS http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/-/ready
```

PostgreSQL stack:

```bash
docker compose --env-file .env -f docker-compose.postgres.yml --profile observability up -d tikeo-prometheus
curl -fsS http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/-/ready
```

MySQL stack:

```bash
docker compose --env-file .env -f docker-compose.mysql.yml --profile observability up -d tikeo-prometheus
curl -fsS http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/-/ready
```

Prometheus loads `prometheus.yml`, `tikeo-recording-rules.yml`, and `tikeo-alert-rules.yml`. The scrape target is `tikeo-server:9090`, which is the Server service name in the Compose network.

## Web nginx, `/api/v1/`, `/api/`, and SSE

The Web image serves static assets and reverse proxies API requests through nginx.

- `/api/v1/` is the versioned Management API route. It is declared before `/api/` and includes SSE-safe proxy settings: HTTP/1.1 upstream, buffering disabled, cache disabled, long read/send timeouts, gzip disabled, and `X-Accel-Buffering: no`.
- `/api/` is a generic fallback for non-v1 or future API paths. It intentionally does not carry the heavier long-lived stream settings.
- `/api-docs/` proxies the OpenAPI UI and JSON.

If a reverse proxy is placed in front of `tikeo-web`, keep SSE buffering disabled on the outer proxy too. See [SSE realtime deployment](./sse-realtime) for stream validation commands.

## Docker run overview

`docker run` is more manual than Compose: you must create the network, choose stable container names, mount the same files, and keep ports consistent yourself. Use it for single-host debugging or environments where Compose is not available.

### Docker run with SQLite

```bash
mkdir -p config/tls

docker network create tikeo-net

docker volume create tikeo-data
docker volume create tikeo-logs

docker run -d --name tikeo-server \
  --network tikeo-net \
  --restart unless-stopped \
  -p 9090:9090 \
  -p 9998:9998 \
  -v "$PWD/config/tikeo.yml:/config/tikeo.yml:ro" \
  -v "$PWD/config/tls:/config/tls:ro" \
  -v tikeo-data:/data \
  -v tikeo-logs:/logs \
  -e TZ=Asia/Shanghai \
  -e MIMALLOC_PURGE_DELAY=0 \
  -e MIMALLOC_PURGE_DECOMMITS=1 \
  -e MIMALLOC_ABANDONED_PAGE_PURGE=1 \
  yhyzgn/tikeo-server:latest serve --config /config/tikeo.yml

docker run -d --name tikeo-web \
  --network tikeo-net \
  --restart unless-stopped \
  -p 8080:80 \
  yhyzgn/tikeo-web:latest

curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:8080/
```

The Web container can reach the Server because both containers are on `tikeo-net` and the Server container is named `tikeo-server`.

### Docker run with PostgreSQL

Start PostgreSQL first, then use the same Server and Web commands as above after changing `config/tikeo.yml` to `type: postgres` and `host: tikeo-postgres`.

```bash
docker network create tikeo-net

docker volume create tikeo-postgres-data

docker run -d --name tikeo-postgres \
  --network tikeo-net \
  --restart unless-stopped \
  -p 15432:5432 \
  -v tikeo-postgres-data:/var/lib/postgresql/data \
  -e POSTGRES_DB=tikeo \
  -e POSTGRES_USER=tikeo \
  -e POSTGRES_PASSWORD=change-me \
  postgres:16-alpine
```

Then start `tikeo-server` with `/config/tikeo.yml` pointing at `tikeo-postgres:5432`.

### Docker run with MySQL

Start MySQL first, then use the same Server and Web commands as above after changing `config/tikeo.yml` to `type: mysql` and `host: tikeo-mysql`.

```bash
docker network create tikeo-net

docker volume create tikeo-mysql-data

docker run -d --name tikeo-mysql \
  --network tikeo-net \
  --restart unless-stopped \
  -p 13306:3306 \
  -v tikeo-mysql-data:/var/lib/mysql \
  -e MYSQL_DATABASE=tikeo \
  -e MYSQL_USER=tikeo \
  -e MYSQL_PASSWORD=change-me \
  -e MYSQL_ROOT_PASSWORD=change-root \
  mysql:8.4 \
  --character-set-server=utf8mb4 \
  --collation-server=utf8mb4_0900_ai_ci
```

Then start `tikeo-server` with `/config/tikeo.yml` pointing at `tikeo-mysql:3306`.

### Docker run Prometheus

```bash
docker run -d --name tikeo-prometheus \
  --network tikeo-net \
  --restart unless-stopped \
  -p 9091:9090 \
  -v "$PWD/observability/prometheus/prometheus.yml:/etc/prometheus/prometheus.yml:ro" \
  -v "$PWD/observability/prometheus/tikeo-recording-rules.yml:/etc/prometheus/tikeo-recording-rules.yml:ro" \
  -v "$PWD/observability/prometheus/tikeo-alert-rules.yml:/etc/prometheus/tikeo-alert-rules.yml:ro" \
  prom/prometheus:v3.0.1 \
  --config.file=/etc/prometheus/prometheus.yml \
  --web.enable-lifecycle

curl -fsS http://127.0.0.1:9091/-/ready
```

## Worker connectivity

Workers dial out to the Server Worker Tunnel. In the default local setup they use:

```bash
TIKEO_WORKER_TUNNEL_PUBLIC_ENDPOINT=http://127.0.0.1:9998
```

For remote Workers, set the public endpoint to the externally reachable Worker Tunnel URL and keep firewall rules limited to the Server HTTP/Web ports and Worker Tunnel port that you actually use. Do not expose arbitrary business Worker ports.

If a Worker advertises script runners, install sandbox tools in that Worker image or mount a populated `TIKEO_SANDBOX_TOOLS_DIR`; see [Worker sandbox tools and Dockerfiles](./worker-sandbox-tools).

## Upgrade and rollback

1. Pin images in `.env`:

   ```bash
   TIKEO_IMAGE=yhyzgn/tikeo-server:v0.3.19
   TIKEO_WEB_IMAGE=yhyzgn/tikeo-web:v0.3.19
   ```

2. Pull and restart the selected stack:

   ```bash
   docker compose --env-file .env -f docker-compose.yml pull
   docker compose --env-file .env -f docker-compose.yml up -d
   ```

3. Verify business output before deleting old backup artifacts:

   ```bash
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/api/v1/metrics/summary
   curl -fsS http://127.0.0.1:${TIKEO_WEB_PORT:-8080}/
   ```

To roll back, set the two image variables to the previous working tags and run the same `pull` and `up -d` commands.

## Prerequisites

- Docker Engine and Docker Compose v2 are installed for Compose examples.
- `config/tikeo.yml` exists and has been reviewed for the selected database, TLS, logs, auth, and public console URL.
- Required host paths exist: `./config/tikeo.yml`, `./config/tls`, and `./observability/prometheus/*` when Prometheus is enabled.
- Named volumes are acceptable for the target backup policy.
- Host ports in `.env` do not conflict with existing services.

## Verify

After startup, verify actual service behavior rather than only checking that containers are running:

```bash
docker compose --env-file .env -f docker-compose.yml ps
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/healthz
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/api/v1/metrics/summary
curl -fsS http://127.0.0.1:${TIKEO_WEB_PORT:-8080}/
```

For database-backed stacks, also verify that the database container is healthy and that the selected database volume is covered by backups.

For Prometheus, verify readiness and the target state:

```bash
curl -fsS http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/-/ready
curl -fsS "http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/api/v1/targets?state=active"
```

## Troubleshooting

| Symptom | Check | Fix |
| --- | --- | --- |
| `tikeo-server` exits immediately | `docker compose logs tikeo-server` | Confirm `/config/tikeo.yml` is mounted and valid YAML. |
| PostgreSQL connection fails | `storage.database.host` and `.env` credentials | Use `host: tikeo-postgres`; keep username, password, and database aligned with `TIKEO_POSTGRES_*`. |
| MySQL connection fails | `storage.database.host` and `.env` credentials | Use `host: tikeo-mysql`; keep username, password, and database aligned with `TIKEO_MYSQL_*`. |
| Web returns 502 for API calls | nginx upstream and container names | Keep Server service/container name as `tikeo-server`, or update nginx consistently. |
| SSE pages update late or disconnect | outer proxy buffering | Disable buffering, cache, and gzip for `/api/v1/` on every proxy layer. |
| Prometheus has no active target | Prometheus target and network | The target must be `tikeo-server:9090` on the same Docker network. |
| `container name is already in use` | old containers remain | Run `docker rm tikeo-server tikeo-web tikeo-prometheus tikeo-postgres tikeo-mysql` only after confirming those stopped containers are not needed. |
| Port is already allocated | `.env` host ports | Change `TIKEO_HTTP_PORT`, `TIKEO_WEB_PORT`, `TIKEO_WORKER_TUNNEL_PORT`, or database host ports. |

## Production checklist

- [ ] `TIKEO_IMAGE` and `TIKEO_WEB_IMAGE` are pinned to release tags.
- [ ] `config/tikeo.yml` matches the chosen storage mode and public console URL.
- [ ] Database passwords are changed from examples and are not committed to source control.
- [ ] The active database volume or managed database is backed up and restore-tested.
- [ ] `/config/tls` contains the expected certificates when TLS or mTLS is enabled.
- [ ] The Web proxy and any outer proxy preserve SSE behavior for `/api/v1/`.
- [ ] Prometheus, if enabled, scrapes `tikeo-server:9090` and loads recording plus alert rules.
- [ ] Workers can reach the public Worker Tunnel endpoint and do not require inbound business ports.
- [ ] Upgrade and rollback image tags are documented for the environment.
