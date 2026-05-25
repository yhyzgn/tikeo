# Compose bootstrap

The root `docker-compose.yml` is the canonical Compose entrypoint. This directory keeps production-minded defaults and operator notes.

```bash
cp deploy/compose/tikee.env.example .env
DOCKER_BUILDKIT=1 docker compose --env-file .env up -d --build
curl -fsS http://127.0.0.1:${TIKEE_HTTP_PORT:-9090}/readyz

# Optional Prometheus scrape + recording-rule smoke
DOCKER_BUILDKIT=1 docker compose --profile observability --env-file .env up -d --build
curl -fsS http://127.0.0.1:${TIKEE_PROMETHEUS_PORT:-9091}/-/ready
```

Notes:

- The default root Compose stack uses SQLite under a named volume for a single-node service.
- For shared environments, prefer an external PostgreSQL/MySQL URL via `TIKEE__STORAGE__DATABASE_URL`.
- Do not use host networking as a shortcut; Worker Tunnel behavior must be validated through normal container networking.
- Configure TLS/mTLS by mounting cert files and setting `TIKEE__TRANSPORT_SECURITY__*` environment overrides or a derived config file.
