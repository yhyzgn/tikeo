# Tikeo Server

Tikeo Server is the control-plane runtime for scheduled jobs, workflow dispatch, Worker Tunnel coordination, execution logs, notification delivery, audit evidence, and management APIs.

Use this image when you want to run the backend API and Worker Tunnel from Docker, Docker Compose, Kubernetes, or another container platform.

## Image tags

- `latest` — latest stable release published by the Tikeo release pipeline.
- `v0.2.12` — exact Git release tag.
- `0.2.12` — semantic-version alias for the same release.

For production, pin an exact version such as `v0.2.12` or `0.2.12`. Use `latest` only for quick evaluation.

## Ports

| Port | Purpose |
| --- | --- |
| `9090` | HTTP API, health checks, management APIs, OpenAPI, metrics endpoints. |
| `9998` | Worker Tunnel endpoint for outbound worker connections. |

## Persistent data

The default container config stores local SQLite data under `/data`. Mount a named volume or bind mount for any non-disposable environment.

## Mounts and runtime files

| What | Container path | How to use it | Required? |
| --- | --- | --- | --- |
| Built-in config | `/app/config/container.toml` | Image default used by `serve --config /app/config/container.toml`. | Enough for quick evaluation. |
| External config | `/config/container.toml` | Bind-mount or ConfigMap-mount read-only, then run `serve --config /config/container.toml`. | Recommended for production and repeatable operations. |
| SQLite data/db | `/data/tikeo.db` | Persist `/data` when `storage.database_url=sqlite:///data/tikeo.db?mode=rwc`. | Required for non-disposable SQLite. |
| File logs | `/logs/tikeo.log` | Set `TIKEO__OBSERVABILITY__LOGGING__LOG_DIR=/logs` or `observability.logging.log_dir="/logs"`. | Optional; stdout logs are always emitted. |
| TLS/mTLS files | `/config/tls` or `/etc/tikeo/tls` | Mount cert/key/CA files read-only and reference them from `transport_security.*.*_path`. | Only when process-level TLS/mTLS is enabled. |

The Server accepts `tikeo serve --config <path>` and `TIKEO_CONFIG`. Environment overrides use the
`TIKEO__SECTION__KEY` shape: `TIKEO__STORAGE__DATABASE_URL`,
`TIKEO__OBSERVABILITY__LOGGING__LOG_DIR`, `TIKEO__SERVER__LISTEN_ADDR`, and so on.

When using PostgreSQL or MySQL, durable state lives in the database, not in the Server `/data` volume.
Persist `/var/lib/postgresql/data` for a self-hosted PostgreSQL container or `/var/lib/mysql` for a
self-hosted MySQL container, or use your managed database backup/snapshot policy.

## Quick start with `docker run`

```bash
docker network create tikeo 2>/dev/null || true

docker volume create tikeo-data
docker volume create tikeo-logs
mkdir -p ./tikeo/config
cat > ./tikeo/config/container.toml <<'TOML'
[server]
listen_addr = "0.0.0.0:9090"
worker_tunnel_addr = "0.0.0.0:9998"

[storage]
database_url = "sqlite:///data/tikeo.db?mode=rwc"

[observability.logging]
level = "info"
log_dir = "/logs"
TOML

docker run -d \
  --name tikeo-server \
  --network tikeo \
  -p 9090:9090 \
  -p 9998:9998 \
  -v "$PWD/tikeo/config/container.toml:/config/container.toml:ro" \
  -v tikeo-data:/data \
  -v tikeo-logs:/logs \
  -e TIKEO__STORAGE__DATABASE_URL='sqlite:///data/tikeo.db?mode=rwc' \
  -e TIKEO__OBSERVABILITY__LOGGING__LOG_DIR=/logs \
  --restart unless-stopped \
  yhyzgn/tikeo-server:v0.2.12 \
  serve --config /config/container.toml

curl -fsS http://127.0.0.1:9090/readyz
```

After the server is ready, open the Web console image or use the HTTP API/SDKs. Bootstrap the first owner through the product bootstrap flow; do not rely on default administrator credentials.

## Docker Compose: server + web + SQLite

Create `.env`:

```bash
cat > .env <<'ENV'
TIKEO_IMAGE=yhyzgn/tikeo-server:v0.2.12
TIKEO_WEB_IMAGE=yhyzgn/tikeo-web:v0.2.12
TIKEO_HTTP_PORT=9090
TIKEO_WORKER_TUNNEL_PORT=9998
TIKEO_WEB_PORT=8080
TIKEO_DATA_VOLUME=tikeo-data
ENV
```

Create `compose.yml`:

```yaml
services:
  tikeo:
    image: ${TIKEO_IMAGE}
    command: ["serve", "--config", "/app/config/container.toml"]
    ports:
      - "${TIKEO_HTTP_PORT:-9090}:9090"
      - "${TIKEO_WORKER_TUNNEL_PORT:-9998}:9998"
    volumes:
      - tikeo-data:/data
    # For externalized config and file logs, switch the command to /config/container.toml
    # and add read-only config plus log mounts:
    # command: ["serve", "--config", "/config/container.toml"]
    # environment:
    #   TIKEO__OBSERVABILITY__LOGGING__LOG_DIR: /logs
    # volumes:
    #   - ./tikeo/config/container.toml:/config/container.toml:ro
    #   - tikeo-data:/data
    #   - tikeo-logs:/logs
    healthcheck:
      test: ["CMD-SHELL", "curl -fsS http://127.0.0.1:9090/readyz >/dev/null"]
      interval: 5s
      timeout: 5s
      retries: 30
      start_period: 10s
    restart: unless-stopped

  web:
    image: ${TIKEO_WEB_IMAGE}
    depends_on:
      tikeo:
        condition: service_healthy
    ports:
      - "${TIKEO_WEB_PORT:-8080}:80"
    restart: unless-stopped

volumes:
  tikeo-data:
    name: ${TIKEO_DATA_VOLUME:-tikeo-data}
  # tikeo-logs:
  #   name: tikeo-logs
```

Start it:

```bash
docker compose --env-file .env up -d
curl -fsS http://127.0.0.1:9090/readyz
open http://127.0.0.1:8080
```

## PostgreSQL or MySQL

For shared production environments, prefer PostgreSQL or MySQL instead of local SQLite. Use the release Compose assets from GitHub Releases or the repository files:

```bash
# PostgreSQL stack
docker compose --env-file .env -f docker-compose.postgres.yml up -d

# MySQL stack
docker compose --env-file .env -f docker-compose.mysql.yml up -d
```

Set the image variables to the released tags before starting:

```bash
TIKEO_IMAGE=yhyzgn/tikeo-server:v0.2.12
TIKEO_WEB_IMAGE=yhyzgn/tikeo-web:v0.2.12
```

## Health checks

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

## Related images

- Web console: `yhyzgn/tikeo-web`
- Documentation site: `yhyzgn/tikeo-docs`

## Documentation

Full deployment, configuration, SDK, worker, notification, and troubleshooting guides are in the docs site and GitHub repository:

- https://github.com/yhyzgn/tikeo
- https://github.com/yhyzgn/tikeo/releases
