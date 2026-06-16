# Tikeo Docs

Tikeo Docs is the versioned documentation site for deploying, configuring, integrating, and operating Tikeo.

This image is a static Docusaurus build served by nginx. It is useful when your production or intranet environment needs the same documentation version as the deployed Server/Web release.

## Image tags

- `latest` — latest stable release published by the Tikeo release pipeline.
- `v0.2.12` — exact Git release tag.
- `0.2.12` — semantic-version alias for the same release.

For production, pin an exact version such as `v0.2.12` or `0.2.12`. Use `latest` only for quick evaluation.

## Port

| Port | Purpose |
| --- | --- |
| `80` | nginx static documentation site. |

## Quick start with `docker run`

```bash
docker run -d \
  --name tikeo-docs \
  -p 8081:80 \
  --restart unless-stopped \
  yhyzgn/tikeo-docs:v0.2.12

open http://127.0.0.1:8081
```

Health check:

```bash
curl -fsS http://127.0.0.1:8081/healthz
```

## Docker Compose: docs only

```yaml
services:
  docs:
    image: yhyzgn/tikeo-docs:v0.2.12
    ports:
      - "8081:80"
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://127.0.0.1/healthz >/dev/null"]
      interval: 5s
      timeout: 5s
      retries: 30
      start_period: 5s
    restart: unless-stopped
```

Run it:

```bash
docker compose up -d
open http://127.0.0.1:8081
```

## Docker Compose: server + web + docs

```yaml
services:
  tikeo:
    image: yhyzgn/tikeo-server:v0.2.12
    command: ["serve", "--config", "/app/config/container.toml"]
    ports:
      - "9090:9090"
      - "9998:9998"
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
    image: yhyzgn/tikeo-web:v0.2.12
    depends_on:
      tikeo:
        condition: service_healthy
    ports:
      - "8080:80"
    restart: unless-stopped

  docs:
    image: yhyzgn/tikeo-docs:v0.2.12
    ports:
      - "8081:80"
    restart: unless-stopped

volumes:
  tikeo-data:
```

Start it:

```bash
docker compose up -d
open http://127.0.0.1:8080
open http://127.0.0.1:8081
```

## What the docs cover

- Local and production deployment.
- Docker Compose, Kubernetes, Helm, GitOps, Terraform, and bare-metal operations.
- Server configuration and defaults.
- SDK/API integration across Java, Node.js, Python, Go, and Rust.
- Worker development, Worker Tunnel behavior, task execution, logs, notifications, and troubleshooting.

## Related images

- Server: `yhyzgn/tikeo-server`
- Web console: `yhyzgn/tikeo-web`

## Documentation source

- https://github.com/yhyzgn/tikeo/tree/main/docs
- https://github.com/yhyzgn/tikeo/releases
