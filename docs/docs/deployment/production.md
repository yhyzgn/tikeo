---
title: Production deployment guide
description: Production runbook for Tikeo Server, Web, database, Worker Tunnel, mounts, TLS, observability, backup, rollback, and smoke verification.
keywords: [tikeo production deployment, docker compose, helm, worker tunnel, postgres, mysql]
---

# Production deployment guide

A production Tikeo environment has five responsibilities:

| Component | Responsibility | Exposed to |
| --- | --- | --- |
| Tikeo Server | HTTP API, scheduler, migrations, Worker Tunnel, notifications, audit. | Operators, SDK Management clients, outbound Workers. |
| Tikeo Web | Browser console for jobs, workers, workflows, scripts, notifications, audit, RBAC. | Human operators. |
| Database | Durable jobs, instances, logs, RBAC, notifications, audit, cluster ownership, outbox rows. | Server only. |
| Worker processes | Execute normal processors/scripts/plugins and stream logs/results. | Outbound to Worker Tunnel only. |
| Notification providers | Receive rendered messages. | Server outbound only. |

Workers do not expose inbound task ports. They dial `server.worker_tunnel_addr`.

## Choose an installation path

| Situation | Recommended path | Notes |
| --- | --- | --- |
| Laptop evaluation | `config/dev.yml` or Compose SQLite | Fast and disposable. |
| Small VM | Single binary + systemd + PostgreSQL/MySQL | Simple operations. |
| Shared non-Kubernetes environment | Docker Compose + PostgreSQL/MySQL | Uses release images and explicit mounts. |
| Kubernetes production | Helm + external database + ingress/gateway | Supports HA, Secrets, TLS/mTLS, platform observability. |
| Air-gapped/change-controlled | Pin image digests and release assets | Repeatable rollback evidence. |

Use SQLite only when you accept single-node local durability. Production should use PostgreSQL/MySQL/CockroachDB-compatible PostgreSQL wire storage.

## Baseline Server configuration

Start from the single formal production template:

```bash
cp config/tikeo.yml /etc/tikeo/tikeo.yml
```

Set production values in the mounted YAML file. Prefer structured database fields; passwords with `@`, `/`, `:`, or `#` do not need manual URL escaping.

```yaml
server:
  listen_addr: "0.0.0.0:9090"
  worker_tunnel_addr: "0.0.0.0:9998"

storage:
  database:
    type: postgres
    host: postgres.example
    port: 5432
    username: tikeo
    password: "p@ss/word:with#chars"
    database: tikeo
    params:
      sslmode: require

observability:
  logging:
    root:
      level: INFO
    http:
      include_headers: false
      include_body: false
      max_body_bytes: 65536
    sql:
      enabled: false
      level: DEBUG
      include_values: false
      slow_threshold_ms: 250
    channels:
      console:
        enabled: true
        level: INFO
      file:
        enabled: true
        level: INFO
        path: /logs
      error-file:
        enabled: true
        level: ERROR
        path: /logs
```

See [Configuration reference](../reference/configuration) for the complete Server and Worker tables.

## Mounts and persistent directories

| Deployment surface | Config path | Data/db path | Log path | Mount guidance |
| --- | --- | --- | --- | --- |
| Docker image default | `/config/tikeo.yml` | `/data/tikeo.db` for SQLite | stdout unless file logging is enabled | Fine for quick evaluation; mount `/data` if SQLite is not disposable. |
| Docker with external config | `/config/tikeo.yml` | `/data/tikeo.db` for SQLite | `/logs/tikeo.log` when `file.path=/logs` | Bind-mount config read-only, mount `/config/tls`, `/data`, and `/logs`. |
| Docker Compose SQLite | `/config/tikeo.yml` | `tikeo-data:/data` | `tikeo-logs:/logs` | Compose mounts config, TLS, data, and logs explicitly. |
| Docker Compose PostgreSQL | `/config/tikeo.yml` | `tikeo-postgres-data:/var/lib/postgresql/data` on DB service | `tikeo-logs:/logs` | Server `/data` is only a uniform runtime mount; DB state is in the DB service. |
| Docker Compose MySQL | `/config/tikeo.yml` | `tikeo-mysql-data:/var/lib/mysql` on DB service | `tikeo-logs:/logs` | Back up the MySQL volume or managed DB. |
| Kubernetes raw manifest | `/config/tikeo.yml` from ConfigMap | `/data` PVC in SQLite manifest | stdout by default | Add a log PVC only if file logs are enabled. |
| Kubernetes Raft/HA | `/config/tikeo.yml` from ConfigMap + structured DB Secret | external DB | stdout by default | Use StatefulSet/headless peers and Secret-backed DB fields. |
| Helm SQLite | `/config/tikeo.yml` from chart ConfigMap | `/data` PVC | stdout by default | Dev/small single-node only. |
| Helm external DB | `/config/tikeo.yml` + structured DB Secret keys | managed/self-hosted DB | stdout by default | Create Secret keys `type`, `host`, `port`, `username`, `password`, `database`. |
| Binary/systemd | `/etc/tikeo/tikeo.yml` | `/var/lib/tikeo` for local SQLite | `/var/log/tikeo/tikeo.log` if enabled | Own dirs by the `tikeo` user. |
| Web/Docs static images | none | none | nginx stdout | No persistent data. |

## Docker run shape

```bash
mkdir -p ./tikeo/config/tls ./tikeo/data ./tikeo/logs
cp config/tikeo.yml ./tikeo/config/tikeo.yml

docker run -d --name tikeo-server \
  -p 9090:9090 -p 9998:9998 \
  -v "$PWD/tikeo/config/tikeo.yml:/config/tikeo.yml:ro" \
  -v "$PWD/tikeo/config/tls:/config/tls:ro" \
  -v "$PWD/tikeo/data:/data" \
  -v "$PWD/tikeo/logs:/logs" \
  yhyzgn/tikeo-server:latest \
  serve --config /config/tikeo.yml
```

## Docker Compose shape

```bash
cp deploy/compose/tikeo.env.example .env
# Edit .env for Docker parameters; edit config/tikeo.yml for Tikeo service settings.
docker compose --env-file .env pull
docker compose --env-file .env up -d
curl -fsS http://127.0.0.1:9090/readyz
```

Use `docker-compose.postgres.yml` or `docker-compose.mysql.yml` after switching `config/tikeo.yml` `storage.database.type` and host/user/password/database to the matching database service.

## Kubernetes and Helm shape

For Helm external database mode, create structured Secret keys:

```bash
kubectl -n tikeo create secret generic tikeo-database \
  --from-literal=type=postgres \
  --from-literal=host=postgres.example \
  --from-literal=port=5432 \
  --from-literal=username=tikeo \
  --from-literal=password='p@ss/word:with#chars' \
  --from-literal=database=tikeo

helm upgrade --install tikeo deploy/helm/tikeo \
  --namespace tikeo \
  --set server.storage.mode=external \
  --set server.storage.type=postgres \
  --set server.storage.existingSecret=tikeo-database
```

Expose Web and HTTP API through normal ingress. Treat Worker Tunnel separately: use a controller path that supports gRPC/HTTP2, or expose a dedicated LoadBalancer/service for Workers.

## TLS/mTLS and SSE

- Enable `transport_security.http.*` and/or `transport_security.worker_tunnel.*` only after mounting `/config/tls` or Kubernetes Secrets.
- SSE endpoints require proxy buffering disabled and long read/idle timeouts. See [SSE realtime deployment](./sse-realtime).
- Do not use `0.0.0.0` as a client URL; it is a bind address only.

## Smoke verification

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:8080/ >/dev/null
```

Then connect at least one Worker, trigger a test job, inspect instance logs, and verify notification delivery if notification policies are enabled.

For Worker-side script sandbox tools, do not rely on startup downloads in production. Preinstall the tools in the Worker host/image and follow [Worker sandbox tools and Dockerfiles](./worker-sandbox-tools).
