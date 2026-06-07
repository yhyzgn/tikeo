# Compose bootstrap

The root `docker-compose.yml` is the canonical SQLite Compose entrypoint. PostgreSQL and MySQL are provided as override files so the same server/web stack can be validated against each supported storage backend.

```bash
cp deploy/compose/tikeo.env.example .env

# SQLite, single-node default
DOCKER_BUILDKIT=1 docker compose --env-file .env up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz

# PostgreSQL override
DOCKER_BUILDKIT=1 docker compose --env-file .env -f docker-compose.yml -f docker-compose.postgres.yml up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz

# MySQL override
DOCKER_BUILDKIT=1 docker compose --env-file .env -f docker-compose.yml -f docker-compose.mysql.yml up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz

./deploy/smoke/worker-bootstrap-smoke.sh

# Optional Prometheus scrape + recording-rule smoke
DOCKER_BUILDKIT=1 docker compose --profile observability --env-file .env up -d --build
curl -fsS http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/-/ready
```

Notes:

- The default root Compose stack uses SQLite under a named volume for a single-node service.
- For shared environments, prefer an external PostgreSQL/MySQL URL via `TIKEO__STORAGE__DATABASE_URL`.
- Do not use host networking as a shortcut; Worker Tunnel behavior must be validated through normal container networking.
- Configure TLS/mTLS by mounting cert files and setting `TIKEO__TRANSPORT_SECURITY__*` environment overrides or a derived config file.
- Workers still initiate outbound gRPC to `${TIKEO_WORKER_TUNNEL_PUBLIC_ENDPOINT}`; do not expose business pod/container ports for scheduling.
- Use `deploy/worker/identity.env.example` for stable `client_instance_id` and worker pool labels.


## Database compatibility matrix

Use the dedicated compose asset when validating storage portability across supported database backends:

```bash
./scripts/db-compat-smoke.sh
```

It starts PostgreSQL 16 and MySQL 8.4 test services from `deploy/compose/database-compat-compose.yml` when Docker is available. For externally managed databases, set `TIKEO_DB_COMPAT_COMPOSE=false` plus `TIKEO_TEST_POSTGRES_URL` and/or `TIKEO_TEST_MYSQL_URL`.
