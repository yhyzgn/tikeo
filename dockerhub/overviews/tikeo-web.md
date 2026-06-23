# Tikeo Web

Tikeo Web is the browser console for operating Tikeo Server: jobs, workers, namespaces, execution instances, logs, notifications, alerts, scripts, and platform settings.

This image is a static web bundle served by nginx. It normally runs next to `yhyzgn/tikeo-server`.

## Image tags

- `latest` — latest stable release.
- `v${TIKEO_VERSION}` — exact Git release tag placeholder.
- `${TIKEO_VERSION}` — semantic-version alias when published.

## Mounts and persistent data

`tikeo-web` has no database, upload directory, or durable runtime state. nginx logs go to stdout/stderr. Persist Server runtime files instead:

| Component | Path | Mount guidance |
| --- | --- | --- |
| Server config | `/config/tikeo.yml` | Mount read-only when environment-specific config should live outside the image. |
| Server TLS files | `/config/tls` | Mount read-only when process-level TLS/mTLS is enabled. |
| Server SQLite data | `/data/tikeo.db` | Persist `/data` only when Server uses SQLite. |
| Server file logs | `/logs/tikeo.log` | Enable in `config/tikeo.yml` with `observability.logging.log_dir: /logs`. |
| Web static image | none | No persistent mount required. |

## Port

| Port | Purpose |
| --- | --- |
| `80` | nginx static web console. |

## Compose quick start

```bash
cp deploy/compose/tikeo.env.example .env
# Edit .env for Docker parameters; edit config/tikeo.yml for Tikeo service settings.
docker compose --env-file .env pull
docker compose --env-file .env up -d
curl -fsS http://127.0.0.1:9090/readyz
open http://127.0.0.1:8080
```

## Production notes

- Use HTTPS at the edge proxy.
- Keep Web and Server versions aligned.
- Pin exact image tags for rollback.
- Configure `notification_delivery.public_console_base_url` in `config/tikeo.yml` so notification links use the public Web URL.
- Treat Worker Tunnel separately from browser traffic; it needs gRPC/HTTP2 support.
