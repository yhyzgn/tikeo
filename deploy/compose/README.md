# Docker Compose deployment 🐳

[🇨🇳 中文部署文档](../../docs/zh-CN/deployment.md)

The root `docker-compose.yml` is the canonical SQLite stack. PostgreSQL and MySQL are provided as
override files so the same server/web images can be validated against each storage backend.

```bash
cp deploy/compose/tikeo.env.example .env

# SQLite default
DOCKER_BUILDKIT=1 docker compose --env-file .env up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz

# PostgreSQL
DOCKER_BUILDKIT=1 docker compose --env-file .env -f docker-compose.yml -f docker-compose.postgres.yml up -d --build

# MySQL
DOCKER_BUILDKIT=1 docker compose --env-file .env -f docker-compose.yml -f docker-compose.mysql.yml up -d --build
```

## Notes

- SQLite is for local/single-node validation.
- Use external PostgreSQL/MySQL for shared production deployments.
- Validate Worker Tunnel behavior through normal container networking.
- Enable log directories and collect both server and web container logs from the platform.
