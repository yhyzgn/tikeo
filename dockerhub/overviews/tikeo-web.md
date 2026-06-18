# Tikeo Web

Tikeo Web is the browser console for operating Tikeo Server: jobs, workers, namespaces, execution instances, logs, notifications, alerts, scripts, and platform settings.

This image is a static web bundle served by nginx. It should normally run next to `yhyzgn/tikeo-server`.

## Mounts and persistent data

`tikeo-web` itself has no database, upload directory, or durable runtime state. It is an nginx image
serving static files, so normal deployments do not mount `config`, `log`, or `data` directories into
the Web container. nginx logs go to container stdout/stderr.

If you deploy Web together with Server, persist the **Server** storage instead:

| Component | Path | Mount guidance |
| --- | --- | --- |
| Server config | `/config/container.toml` or image default `/app/config/container.toml` | Mount read-only when you need environment-specific config. |
| Server SQLite data | `/data/tikeo.db` | Persist `/data` only when Server uses SQLite. |
| Server file logs | `/logs/tikeo.log` | Optional; set `TIKEO__OBSERVABILITY__LOGGING__LOG_DIR=/logs`. |
| Web static image | none | No persistent mount required. Mount custom nginx config only if you intentionally replace the default routing. |

## Image tags

- `latest` — latest stable release published by the Tikeo release pipeline.
- `v0.2.12` — exact Git release tag.
- `0.2.12` — semantic-version alias for the same release.

For production, pin an exact version such as `v0.2.12` or `0.2.12`. Use `latest` only for quick evaluation.

## Port

| Port | Purpose |
| --- | --- |
| `80` | nginx static web console. |

## Quick start with `docker run`

Start the server first:

```bash
docker network create tikeo 2>/dev/null || true
docker volume create tikeo-data

docker run -d \
  --name tikeo-server \
  --network tikeo \
  -p 9090:9090 \
  -p 9998:9998 \
  -v tikeo-data:/data \
  --restart unless-stopped \
  yhyzgn/tikeo-server:v0.2.12 \
  serve --config /app/config/container.toml
```

Start the web console:

```bash
docker run -d \
  --name tikeo-web \
  --network tikeo \
  -p 8080:80 \
  --restart unless-stopped \
  yhyzgn/tikeo-web:v0.2.12

open http://127.0.0.1:8080
```

The web app talks to the Tikeo API through its configured API base path. In production, put Web and Server behind the same reverse proxy origin or configure your ingress/proxy rules so browser API requests reach the Server.

## Docker Compose: web + server

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
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://127.0.0.1/ >/dev/null"]
      interval: 5s
      timeout: 5s
      retries: 30
      start_period: 5s
    restart: unless-stopped

volumes:
  tikeo-data:
    name: ${TIKEO_DATA_VOLUME:-tikeo-data}
```

Start it:

```bash
docker compose --env-file .env up -d
curl -fsS http://127.0.0.1:9090/readyz
open http://127.0.0.1:8080
```

## Production notes

- Use HTTPS at the edge proxy.
- Keep Web and Server versions aligned.
- Pin exact image tags for rollback.
- Configure public platform URLs in Tikeo Server settings so notification links and console deep links use absolute URLs.
- Do not expose the Worker Tunnel publicly unless worker hosts require it; prefer private networking.

## Related images

- Server: `yhyzgn/tikeo-server`
- Documentation site: `yhyzgn/tikeo-docs`

## Documentation

- https://github.com/yhyzgn/tikeo
- https://github.com/yhyzgn/tikeo/releases
