# Docker Compose deployment 🐳

[🇨🇳 中文部署文档](../../README.zh-CN.md#运行-tikeo-服务)

The root `docker-compose.yml` is the canonical SQLite stack. `docker-compose.postgres.yml` and
`docker-compose.mysql.yml` are complete standalone server + web + database stacks for validating
each storage backend without layering multiple compose files.

```bash
cp deploy/compose/tikeo.env.example .env

# SQLite default
DOCKER_BUILDKIT=1 docker compose --env-file .env up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz

# PostgreSQL
DOCKER_BUILDKIT=1 docker compose --env-file .env -f docker-compose.postgres.yml up -d --build

# MySQL
DOCKER_BUILDKIT=1 docker compose --env-file .env -f docker-compose.mysql.yml up -d --build
```

## Notes

- SQLite is for local/single-node validation.
- Use external PostgreSQL/MySQL for shared production deployments.
- Validate Worker Tunnel behavior through normal container networking.
- Enable log directories and collect both server and web container logs from the platform.
