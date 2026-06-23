# Tikeo Server

Tikeo Server is the control-plane runtime for scheduled jobs, workflow dispatch, Worker Tunnel coordination, execution logs, notification delivery, audit evidence, and management APIs.

## Image tags

- `latest` — latest stable release.
- `v${TIKEO_VERSION}` — exact Git release tag placeholder. Replace `${TIKEO_VERSION}` with the release badge version.
- `${TIKEO_VERSION}` — semantic-version alias when published.

Pin an exact version in production; use `latest` for quick evaluation.

## Ports

| Port | Purpose |
| --- | --- |
| `9090` | HTTP API, health checks, management APIs, OpenAPI, metrics. |
| `9998` | Worker Tunnel endpoint for outbound Worker connections. |

## Configuration, mounts, and runtime files

| What | Container path | How to use it | Required? |
| --- | --- | --- | --- |
| Server config | `/config/tikeo.yml` | Image contains a default file; production should bind-mount or ConfigMap-mount it read-only and run `serve --config /config/tikeo.yml`. | Recommended for repeatable operations. |
| TLS/mTLS files | `/config/tls` | Mount cert/key/CA files read-only and reference them from `transport_security.*.*_path`. | Only when process-level TLS/mTLS is enabled. |
| SQLite data/db | `/data/tikeo.db` | Persist `/data` when `storage.database.type=sqlite`. | Required for non-disposable SQLite. |
| File logs | `/logs/tikeo.log` | Set `observability.logging.log_dir: /logs` in `config/tikeo.yml`. | Optional; stdout logs are always emitted. |
| PostgreSQL/MySQL data | DB container or managed DB | Persist the DB service volume or use managed backups. | Required when self-hosting DB. |

Use structured database config in `config/tikeo.yml` instead of a single URL. Passwords with `@`, `/`, `:`, or `#` do not need manual escaping.

## Quick start with `docker run`

```bash
docker network create tikeo 2>/dev/null || true
docker volume create tikeo-data
docker volume create tikeo-logs
mkdir -p ./tikeo/config/tls
cp config/tikeo.yml ./tikeo/config/tikeo.yml

docker run -d \
  --name tikeo-server \
  --network tikeo \
  -p 9090:9090 \
  -p 9998:9998 \
  -v "$PWD/tikeo/config/tikeo.yml:/config/tikeo.yml:ro" \
  -v "$PWD/tikeo/config/tls:/config/tls:ro" \
  -v tikeo-data:/data \
  -v tikeo-logs:/logs \
  --restart unless-stopped \
  yhyzgn/tikeo-server:latest \
  serve --config /config/tikeo.yml

curl -fsS http://127.0.0.1:9090/readyz
```

## Docker Compose

Use the repository Compose files:

```bash
cp deploy/compose/tikeo.env.example .env
# Edit .env for Docker parameters; edit config/tikeo.yml for Tikeo service settings.
docker compose --env-file .env pull
docker compose --env-file .env up -d
```

For PostgreSQL/MySQL stacks, set `storage.database.type`, `host`, `port`, `username`, `password`, and `database` in `config/tikeo.yml`, then use `docker-compose.postgres.yml` or `docker-compose.mysql.yml`.

## Health checks

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

## Related images

- Web console: `yhyzgn/tikeo-web`
- Documentation site: `yhyzgn/tikeo-docs`
