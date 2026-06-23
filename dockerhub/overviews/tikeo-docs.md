# Tikeo Docs

Tikeo Docs is the versioned documentation site for deploying, configuring, integrating, and operating Tikeo.

This image is a static Docusaurus build served by nginx. It stores no product data and normally needs no persistent mount.

## Image tags

- `latest` — latest stable release.
- `v${TIKEO_VERSION}` — exact Git release tag placeholder.
- `${TIKEO_VERSION}` — semantic-version alias when published.

## Mounts and persistent data

`tikeo-docs` has no database, upload directory, or runtime config. nginx logs go to stdout/stderr. If deployed with Server, persist Server runtime files instead:

| Component | Path | Mount guidance |
| --- | --- | --- |
| Server config | `/config/tikeo.yml` | Mount read-only when config should live outside the image. |
| Server TLS files | `/config/tls` | Mount read-only for process-level TLS/mTLS. |
| Server SQLite data | `/data/tikeo.db` | Persist `/data` only for SQLite mode. |
| Server file logs | `/logs/tikeo.log` | Enable in `config/tikeo.yml` with `observability.logging.log_dir: /logs`. |
| Docs static image | none | No persistent mount required. |

## Port

| Port | Purpose |
| --- | --- |
| `80` | nginx static documentation site. |

## Quick start

```bash
docker run -d \
  --name tikeo-docs \
  -p 8081:80 \
  --restart unless-stopped \
  yhyzgn/tikeo-docs:latest

open http://127.0.0.1:8081
```
