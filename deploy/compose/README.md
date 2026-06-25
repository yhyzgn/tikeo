# Docker Compose deployment 🐳

[🇨🇳 中文部署文档](../../README.zh-CN.md#运行-tikeo-服务)

The root `docker-compose.yml` is the canonical SQLite stack. `docker-compose.postgres.yml` and
`docker-compose.mysql.yml` are complete standalone server + web + database stacks for validating
each storage backend without layering multiple compose files.

All stacks use published Docker Hub images by default:

- `yhyzgn/tikeo-server:latest`
- `yhyzgn/tikeo-web:latest`

They do **not** build from the local `Dockerfile`. Pin `TIKEO_IMAGE` and `TIKEO_WEB_IMAGE` to a
release tag in `.env` for production rollback safety.

```bash
cp deploy/compose/tikeo.env.example .env
# Edit .env for Docker parameters; edit config/tikeo.yml for Tikeo service settings.

# SQLite default: mounts ./config/tikeo.yml, ./config/tls, tikeo-data:/data, and tikeo-logs:/logs.
docker compose --env-file .env pull
docker compose --env-file .env up -d
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz

# PostgreSQL: uses the same ./config/tikeo.yml after switching storage.database to postgres; mounts ./config/tls, tikeo-data:/data, tikeo-logs:/logs, and tikeo-postgres-data:/var/lib/postgresql/data.
docker compose --env-file .env -f docker-compose.postgres.yml pull
docker compose --env-file .env -f docker-compose.postgres.yml up -d

# MySQL: uses the same ./config/tikeo.yml after switching storage.database to mysql; mounts ./config/tls, tikeo-data:/data, tikeo-logs:/logs, and tikeo-mysql-data:/var/lib/mysql.
docker compose --env-file .env -f docker-compose.mysql.yml pull
docker compose --env-file .env -f docker-compose.mysql.yml up -d
```

## Required review before shared deployment

For PostgreSQL/MySQL, edit `config/tikeo.yml` before `up -d`: set `storage.database.type` to `postgres` or `mysql` and fill `host`, `port`, `username`, `password`, and `database`. Passwords can contain `@`, `/`, `:`, or `#`; Tikeo percent-encodes the generated internal URL.


- Set `notification_delivery.public_console_base_url` in the mounted `config/tikeo.yml` to the externally reachable Web URL.
- Use `TIKEO_WORKER_TUNNEL_PUBLIC_ENDPOINT` only as a helper for local worker-demo commands; Server listener config stays in `config/tikeo.yml`.
- Change database container passwords in `.env` before shared use, then edit `config/tikeo.yml` `storage.database` host/port/username/password/database to match.
- Keep `TIKEO_LOGS_VOLUME`, `TIKEO_DATA_VOLUME`, `TIKEO_POSTGRES_DATA_VOLUME`, and `TIKEO_MYSQL_DATA_VOLUME` explicit when multiple environments share one Docker host.
- Collect container stdout logs even when `/logs/tikeo.log` is enabled.

## Notes

- SQLite is for local/single-node validation.
- Use external PostgreSQL/MySQL for shared production deployments.
- Validate Worker Tunnel behavior through normal container networking.
- Web is a static nginx image and normally needs no config, log, or data mount.
