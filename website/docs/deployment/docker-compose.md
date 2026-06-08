---
title: Docker Compose
description: Local and production-shaped Docker Compose entry points for Tikeo.
---

# Docker Compose

Tikeo can run with Docker Compose for local evaluation and production-shaped smoke tests.

## SQLite development path

```bash
DOCKER_BUILDKIT=1 docker compose --env-file .env up -d --build
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
```

## External database overlays

The repository also includes PostgreSQL and MySQL Compose profiles. Use these when you need to verify schema behavior against a production-style database.

## Ports

| Port | Purpose |
|---|---|
| `9090` | HTTP API and Server/Web proxy target |
| `9998` | Worker Tunnel gRPC/HTTP2 listener |
| `80` | Web console container internal port |

## Cleanup

```bash
docker compose down --remove-orphans
```
