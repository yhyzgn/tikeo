---
title: Docker Compose
description: Complete copy-paste Docker Compose deployment files for SQLite, PostgreSQL, MySQL, Web, Worker Tunnel, and Prometheus.
---

# Docker Compose

Use Docker Compose when you want a reproducible local or VM smoke environment with packaged Server and Web containers. This page includes the **complete** committed Compose files so users can copy the full YAML without jumping back to GitHub.

## Quick start

```bash
cp deploy/compose/tikeo.env.example .env
DOCKER_BUILDKIT=1 docker compose --env-file .env up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
curl -fsS http://127.0.0.1:${TIKEO_WEB_PORT:-8080}/ >/dev/null
```

Open the Web console at `http://127.0.0.1:${TIKEO_WEB_PORT:-8080}`.

## Full `docker-compose.yml`

SQLite is the fastest local evaluation path. It persists `/data/tikeo.db` in the `tikeo-data` named volume.

```yaml
services:
  tikeo:
    build:
      context: .
      dockerfile: Dockerfile
    image: ${TIKEO_IMAGE:-yhyzgn/tikeo-server:local}
    command: ["serve", "--config", "/app/config/container.toml"]
    environment:
      TZ: ${TZ:-Asia/Shanghai}
      MIMALLOC_PURGE_DELAY: ${MIMALLOC_PURGE_DELAY:-0}
      MIMALLOC_PURGE_DECOMMITS: ${MIMALLOC_PURGE_DECOMMITS:-1}
      MIMALLOC_ABANDONED_PAGE_PURGE: ${MIMALLOC_ABANDONED_PAGE_PURGE:-1}
    ports:
      - "${TIKEO_HTTP_PORT:-9090}:9090"
      - "${TIKEO_WORKER_TUNNEL_PORT:-9998}:9998"
    volumes:
      - tikeo-data:/data
    healthcheck:
      test: ["CMD-SHELL", "curl -fsS http://127.0.0.1:9090/readyz >/dev/null"]
      interval: 5s
      timeout: 5s
      retries: 30
      start_period: 10s
    restart: unless-stopped

  web:
    build:
      context: ./web
      dockerfile: Dockerfile
    image: ${TIKEO_WEB_IMAGE:-yhyzgn/tikeo-web:local}
    depends_on:
      tikeo:
        condition: service_healthy
    ports:
      - "${TIKEO_WEB_PORT:-8080}:80"
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://127.0.0.1/ >/dev/null"]
      interval: 5s
      timeout: 5s
      retries: 30
      start_period: 5s
    restart: unless-stopped

  prometheus:
    image: prom/prometheus:v3.0.1
    profiles: ["observability"]
    depends_on:
      tikeo:
        condition: service_healthy
    command:
      - "--config.file=/etc/prometheus/prometheus.yml"
      - "--web.enable-lifecycle"
    volumes:
      - ./observability/prometheus/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - ./observability/prometheus/tikeo-recording-rules.yml:/etc/prometheus/tikeo-recording-rules.yml:ro
    ports:
      - "${TIKEO_PROMETHEUS_PORT:-9091}:9090"
    restart: unless-stopped

volumes:
  tikeo-data:
    name: ${TIKEO_DATA_VOLUME:-tikeo-data}
```

## PostgreSQL stack

```bash
DOCKER_BUILDKIT=1 docker compose --env-file .env -f docker-compose.postgres.yml up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
docker compose --env-file .env -f docker-compose.postgres.yml ps
```

Useful `.env` overrides:

```dotenv
TIKEO_POSTGRES_PORT=15432
TIKEO_POSTGRES_DB=tikeo
TIKEO_POSTGRES_USER=tikeo
TIKEO_POSTGRES_PASSWORD=change-me
TIKEO_POSTGRES_DATA_VOLUME=tikeo-postgres-data
```

## Full `docker-compose.postgres.yml`

```yaml
services:
  tikeo:
    build:
      context: .
      dockerfile: Dockerfile
    image: ${TIKEO_IMAGE:-yhyzgn/tikeo-server:local}
    command: ["serve", "--config", "/app/config/postgres.toml"]
    depends_on:
      postgres:
        condition: service_healthy
    environment:
      TZ: ${TZ:-Asia/Shanghai}
      MIMALLOC_PURGE_DELAY: ${MIMALLOC_PURGE_DELAY:-0}
      MIMALLOC_PURGE_DECOMMITS: ${MIMALLOC_PURGE_DECOMMITS:-1}
      MIMALLOC_ABANDONED_PAGE_PURGE: ${MIMALLOC_ABANDONED_PAGE_PURGE:-1}
    ports:
      - "${TIKEO_HTTP_PORT:-9090}:9090"
      - "${TIKEO_WORKER_TUNNEL_PORT:-9998}:9998"
    healthcheck:
      test: ["CMD-SHELL", "curl -fsS http://127.0.0.1:9090/readyz >/dev/null"]
      interval: 5s
      timeout: 5s
      retries: 30
      start_period: 10s
    restart: unless-stopped

  web:
    build:
      context: ./web
      dockerfile: Dockerfile
    image: ${TIKEO_WEB_IMAGE:-yhyzgn/tikeo-web:local}
    depends_on:
      tikeo:
        condition: service_healthy
    ports:
      - "${TIKEO_WEB_PORT:-8080}:80"
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://127.0.0.1/ >/dev/null"]
      interval: 5s
      timeout: 5s
      retries: 30
      start_period: 5s
    restart: unless-stopped

  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: ${TIKEO_POSTGRES_DB:-tikeo}
      POSTGRES_USER: ${TIKEO_POSTGRES_USER:-tikeo}
      POSTGRES_PASSWORD: ${TIKEO_POSTGRES_PASSWORD:-tikeo}
    ports:
      - "${TIKEO_POSTGRES_PORT:-15432}:5432"
    volumes:
      - tikeo-postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U $${POSTGRES_USER} -d $${POSTGRES_DB}"]
      interval: 5s
      timeout: 5s
      retries: 30
      start_period: 10s
    restart: unless-stopped

  prometheus:
    image: prom/prometheus:v3.0.1
    profiles: ["observability"]
    depends_on:
      tikeo:
        condition: service_healthy
    command:
      - "--config.file=/etc/prometheus/prometheus.yml"
      - "--web.enable-lifecycle"
    volumes:
      - ./observability/prometheus/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - ./observability/prometheus/tikeo-recording-rules.yml:/etc/prometheus/tikeo-recording-rules.yml:ro
    ports:
      - "${TIKEO_PROMETHEUS_PORT:-9091}:9090"
    restart: unless-stopped

volumes:
  tikeo-postgres-data:
    name: ${TIKEO_POSTGRES_DATA_VOLUME:-tikeo-postgres-data}
```

## MySQL stack

```bash
DOCKER_BUILDKIT=1 docker compose --env-file .env -f docker-compose.mysql.yml up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
docker compose --env-file .env -f docker-compose.mysql.yml ps
```

Useful `.env` overrides:

```dotenv
TIKEO_MYSQL_PORT=13306
TIKEO_MYSQL_DATABASE=tikeo
TIKEO_MYSQL_USER=tikeo
TIKEO_MYSQL_PASSWORD=change-me
TIKEO_MYSQL_ROOT_PASSWORD=change-root
TIKEO_MYSQL_DATA_VOLUME=tikeo-mysql-data
```

## Full `docker-compose.mysql.yml`

```yaml
services:
  tikeo:
    build:
      context: .
      dockerfile: Dockerfile
    image: ${TIKEO_IMAGE:-yhyzgn/tikeo-server:local}
    command: ["serve", "--config", "/app/config/mysql.toml"]
    depends_on:
      mysql:
        condition: service_healthy
    environment:
      TZ: ${TZ:-Asia/Shanghai}
      MIMALLOC_PURGE_DELAY: ${MIMALLOC_PURGE_DELAY:-0}
      MIMALLOC_PURGE_DECOMMITS: ${MIMALLOC_PURGE_DECOMMITS:-1}
      MIMALLOC_ABANDONED_PAGE_PURGE: ${MIMALLOC_ABANDONED_PAGE_PURGE:-1}
    ports:
      - "${TIKEO_HTTP_PORT:-9090}:9090"
      - "${TIKEO_WORKER_TUNNEL_PORT:-9998}:9998"
    healthcheck:
      test: ["CMD-SHELL", "curl -fsS http://127.0.0.1:9090/readyz >/dev/null"]
      interval: 5s
      timeout: 5s
      retries: 30
      start_period: 10s
    restart: unless-stopped

  web:
    build:
      context: ./web
      dockerfile: Dockerfile
    image: ${TIKEO_WEB_IMAGE:-yhyzgn/tikeo-web:local}
    depends_on:
      tikeo:
        condition: service_healthy
    ports:
      - "${TIKEO_WEB_PORT:-8080}:80"
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://127.0.0.1/ >/dev/null"]
      interval: 5s
      timeout: 5s
      retries: 30
      start_period: 5s
    restart: unless-stopped

  mysql:
    image: mysql:8.4
    environment:
      MYSQL_DATABASE: ${TIKEO_MYSQL_DATABASE:-tikeo}
      MYSQL_USER: ${TIKEO_MYSQL_USER:-tikeo}
      MYSQL_PASSWORD: ${TIKEO_MYSQL_PASSWORD:-tikeo}
      MYSQL_ROOT_PASSWORD: ${TIKEO_MYSQL_ROOT_PASSWORD:-root}
    command:
      - "--character-set-server=utf8mb4"
      - "--collation-server=utf8mb4_0900_ai_ci"
    ports:
      - "${TIKEO_MYSQL_PORT:-13306}:3306"
    volumes:
      - tikeo-mysql-data:/var/lib/mysql
    healthcheck:
      test: ["CMD-SHELL", "mysqladmin ping -h 127.0.0.1 -uroot -p$${MYSQL_ROOT_PASSWORD} --silent"]
      interval: 5s
      timeout: 5s
      retries: 60
      start_period: 20s
    restart: unless-stopped

  prometheus:
    image: prom/prometheus:v3.0.1
    profiles: ["observability"]
    depends_on:
      tikeo:
        condition: service_healthy
    command:
      - "--config.file=/etc/prometheus/prometheus.yml"
      - "--web.enable-lifecycle"
    volumes:
      - ./observability/prometheus/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - ./observability/prometheus/tikeo-recording-rules.yml:/etc/prometheus/tikeo-recording-rules.yml:ro
    ports:
      - "${TIKEO_PROMETHEUS_PORT:-9091}:9090"
    restart: unless-stopped

volumes:
  tikeo-mysql-data:
    name: ${TIKEO_MYSQL_DATA_VOLUME:-tikeo-mysql-data}
```

## Optional Prometheus

The three Compose files include a `prometheus` service behind the `observability` profile.

```bash
docker compose --env-file .env --profile observability up -d prometheus
curl -fsS http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/-/ready
```

Prometheus reads committed files under `observability/prometheus/`.

## Compose parameter reference

| Variable | Default | Used by | Meaning |
|---|---:|---|---|
| `TIKEO_IMAGE` | `yhyzgn/tikeo-server:dev` | Server | Server image tag to build/use. |
| `TIKEO_WEB_IMAGE` | `yhyzgn/tikeo-web:dev` | Web | Web image tag to build/use. |
| `TIKEO_HTTP_PORT` | `9090` | Server | Host port for HTTP API and health checks. |
| `TIKEO_WORKER_TUNNEL_PORT` | `9998` | Server | Host port for outbound Worker Tunnel clients. |
| `TIKEO_WEB_PORT` | `8080` | Web | Host port for browser UI. |
| `TIKEO_PROMETHEUS_PORT` | `9091` | Prometheus | Host port for optional Prometheus. |
| `TIKEO_DATA_VOLUME` | `tikeo-data` | SQLite | Named volume for SQLite data. |
| `TIKEO_WORKER_TUNNEL_PUBLIC_ENDPOINT` | `http://127.0.0.1:9998` | Workers | Endpoint external demo workers should dial. |
| `TIKEO__STORAGE__DATABASE_URL` | unset | Server | Optional config override for external DB URLs. |

## Worker connectivity

Workers still dial out to the Server Worker Tunnel. For a local Rust demo:

```bash
TIKEO_WORKER_TUNNEL_ENDPOINT=${TIKEO_WORKER_TUNNEL_PUBLIC_ENDPOINT:-http://127.0.0.1:9998}   cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Do not expose arbitrary business Worker ports. The only public Worker-facing endpoint should be Tikeo's Server tunnel.

## Cleanup and reset

Stop containers but keep data:

```bash
docker compose --env-file .env down --remove-orphans
```

Delete local SQLite data volume:

```bash
docker compose --env-file .env down --remove-orphans -v
```

For PostgreSQL/MySQL stacks, include the same `-f` file you used for startup.
