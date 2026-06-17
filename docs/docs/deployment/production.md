---
title: Production deployment guide
description: A human runbook for deploying Tikeo Server, Web, Docs, databases, Worker Tunnel networking, TLS, observability, backup, rollback, and smoke verification.
keywords: [tikeo production deployment, docker compose, helm, worker tunnel, postgres, mysql]
---

# Production deployment guide

This page is the deployment playbook you should read before copying any YAML. It explains what to deploy, what to keep outside the cluster, how traffic should flow, which configuration keys matter, and how to prove the installation works. Use the narrower pages for exact manifests: [Docker Compose](./docker-compose), [Kubernetes and Helm](./kubernetes), [single binary](./single-binary), [SSE realtime](./sse-realtime), and [management trigger smoke](./management-trigger-smoke-runbook).

## Deployment goal

A production Tikeo environment has these responsibilities:

| Component | Runs where | Responsibility | Exposed to |
| --- | --- | --- | --- |
| Tikeo Server | container, VM, or Kubernetes Deployment | HTTP API, Web API, scheduler, storage migrations, Worker Tunnel listener, Notification Center delivery worker | Operators, SDK Management clients, outbound Workers |
| Tikeo Web | static nginx container or any static host | Browser console for jobs, workers, workflows, scripts, notifications, audit, RBAC | Human operators |
| Database | managed PostgreSQL/MySQL, or SQLite only for local/small single-node | Durable jobs, instances, logs, RBAC, notification records, audit | Server only |
| Worker processes | app clusters, private VPCs, VMs, sidecars, or external networks | Execute SDK processors/scripts/plugins and stream logs/results | Outbound to Worker Tunnel only |
| Notification providers | SaaS/webhook/email/PagerDuty/office bots | Receive rendered notification payloads | Server outbound only |

The most important boundary: **business Workers do not expose inbound task ports**. They dial out to `server.worker_tunnel_addr`. Do not create a Worker Service just so the scheduler can call it.

## Choose an installation path

| Situation | Recommended path | Why |
| --- | --- | --- |
| Laptop evaluation | `config/dev.toml` + Web dev server or Compose | Fast, disposable SQLite, easy logs. |
| Small internal VM | Single binary + systemd + PostgreSQL/MySQL | Simple operations, good for one Server node. |
| Team shared environment | Docker Compose with PostgreSQL/MySQL | Reproducible, close to release images, easy smoke. |
| Kubernetes production | Helm chart + external database + ingress/gateway | Separates Server/Web, supports TLS/mTLS and platform Secret management. |
| Air-gapped or strict change control | Pin Docker image digests and release assets | Repeatable rollout and rollback evidence. |

Use SQLite only when you accept single-node local durability. For production, prefer PostgreSQL or MySQL and back it up with your normal database tooling.

## Server HA deployment choice

For Kubernetes multi-pod Server HA, use the **Raft FSOD Cluster** runbook, [Server HA and Raft FSOD Cluster](./server-ha). The short version is:

- `standalone` is for one Server process/pod.
- `raft` is the production multi-pod HA mode and renders a StatefulSet/headless peer topology in Helm.
- Raft mode uses one fenced Leader for global timer/retry loops and shard ownership projection; dispatch is multi-owner by shard for pods that hold active ownership rows.
- Extra Server pods improve failover, API availability, Worker Tunnel gateway capacity, and Raft membership; dispatch throughput can spread across active shard owners, while global timer/retry loops remain Leader-fenced.
- Do not add Redis/Dragonfly locks for core scheduler ownership; future multi-active scheduling must be Raft shard ownership with fencing.


## Network model

Plan four network paths separately:

1. **Human Web traffic**: browser → Web nginx → Server API path. Configure reverse proxy timeouts for long API calls and SSE.
2. **Management API traffic**: app/service → Server HTTP API with `x-tikeo-api-key` for app-scoped SDK clients, or bearer session token for human routes.
3. **Worker Tunnel traffic**: Worker → Server Worker Tunnel endpoint (`9998` by default). This must support gRPC/HTTP2 if TLS/gateway is enabled.
4. **Provider traffic**: Server → Slack/DingTalk/Feishu/WeCom/PagerDuty/email/webhook. Secrets live in channel config or environment-compatible refs.

Do not use `0.0.0.0` as a client URL. It is a bind address only. Clients should use `127.0.0.1`, a service DNS name, or a real hostname.

## Baseline configuration

Start from one of the committed config files:

```bash
cp config/postgres.toml /etc/tikeo/tikeo.toml
# or
cp config/mysql.toml /etc/tikeo/tikeo.toml
```

Set database credentials through environment variables or platform Secrets:

```bash
export TIKEO__STORAGE__DATABASE_URL='postgres://tikeo:${PASSWORD}@postgres:5432/tikeo'
export TIKEO__SERVER__LISTEN_ADDR='0.0.0.0:9090'
export TIKEO__SERVER__WORKER_TUNNEL_ADDR='0.0.0.0:9998'
export TIKEO__OBSERVABILITY__LOGGING__LEVEL='info'
```

The environment variable convention maps nested keys to `TIKEO__SECTION__KEY`, for example `storage.database_url` becomes `TIKEO__STORAGE__DATABASE_URL`. See [Configuration reference](../reference/configuration) and [Configuration cookbook](../reference/configuration-cookbook) for complete defaults and recipes.

## Docker Compose production-shaped path

For a shared non-Kubernetes environment:

```bash
cp deploy/compose/tikeo.env.example .env
# Edit .env: image tags, ports, database password, timezone.
docker compose --env-file .env up -d --build
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:8080/ >/dev/null
```

Use `docker-compose.postgres.yml` or `docker-compose.mysql.yml` when you want the database in the same stack. Use managed database endpoints for production. Keep volume names, image tags, and port mappings explicit in `.env` so rollback is repeatable.

## Kubernetes production-shaped path

For Kubernetes, use Helm and an external database Secret:

```bash
kubectl create namespace tikeo
kubectl -n tikeo create secret generic tikeo-database \
  --from-literal=database-url='postgres://tikeo:${PASSWORD}@postgres:5432/tikeo'

helm upgrade --install tikeo deploy/helm/tikeo \
  --namespace tikeo \
  --set server.envFromSecret=tikeo-database \
  --set server.service.type=ClusterIP \
  --set web.service.type=ClusterIP
```

Expose Web and Server API through your normal ingress. Treat Worker Tunnel separately: use a controller path that supports gRPC/HTTP2, or expose a dedicated LoadBalancer/service for Workers. For concrete Nginx Ingress, Envoy Gateway, Traefik, and Gateway API settings, use [Kubernetes controller runbook](./kubernetes-controller-runbook).

## Docs site image and operator access

The docs site is also a release artifact. It is not required for the Server to schedule work, but production teams should publish it next to Web so operators can read the exact runbooks for the deployed version. The Docker Hub repository is `yhyzgn/tikeo-docs`, built from `docs/Dockerfile` and served by nginx with `/healthz`.

```bash
docker pull yhyzgn/tikeo-docs:v0.2.9
docker run --rm -p 8081:80 yhyzgn/tikeo-docs:v0.2.9
curl -fsS http://127.0.0.1:8081/healthz
curl -fsS http://127.0.0.1:8081/docs/ >/dev/null
```

In Kubernetes, expose Docs as a separate static site or internal route. Do not proxy Docs traffic through the Server API container; keeping Web and Docs static images separate makes cache, rollback, and access-control policy easier to reason about.

## TLS and mTLS decisions

| Traffic | Minimum | Production recommendation |
| --- | --- | --- |
| Web/API browser traffic | TLS at ingress/proxy | TLS at edge, secure cookies, WAF/rate limit if public. |
| SDK Management API | TLS | TLS plus app-scoped API keys and scoped RBAC. |
| Worker Tunnel | Plaintext only for local | TLS or mTLS when crossing networks. |
| Provider outbound | Provider HTTPS/SMTP TLS | Secret rotation and provider test-send before enabling policy. |

The relevant config namespaces are `transport_security.http` and `transport_security.worker_tunnel`. Helm exposes Worker Tunnel mTLS through values such as `server.tls.workerTunnel.mtlsRequired`.

## Bootstrap and access setup

After the Server is reachable, bootstrap the first Owner once:

```bash
curl -fsS http://127.0.0.1:9090/api/v1/auth/bootstrap | jq .data.registrationOpen
: "${TIKEO_OWNER_USERNAME:?set the production owner username}"
: "${TIKEO_OWNER_EMAIL:?set the production owner email}"
: "${TIKEO_OWNER_PASSWORD:?set the production owner password from your secret manager}"
TOKEN="$(jq -n \
  --arg username "$TIKEO_OWNER_USERNAME" \
  --arg email "$TIKEO_OWNER_EMAIL" \
  --arg password "$TIKEO_OWNER_PASSWORD" \
  '{username:$username,email:$email,password:$password,confirmPassword:$password}' \
  | curl -fsS -X POST http://127.0.0.1:9090/api/v1/auth/bootstrap/register \
      -H 'content-type: application/json' \
      -d @- | jq -r .data.token)"
```

Then create namespace/app scope, service account, and SDK API keys from the Web console or Management API before connecting application Workers. Do not run production Workers with human session tokens.

## Worker rollout pattern

For each service team:

1. Pick the language SDK and read its page under [SDKs](../sdks/rust).
2. Give the Worker a stable `namespace`, `app`, `workerPool`, and processor names.
3. Configure `TIKEO_WORKER_ENDPOINT` or language-specific `WorkerConfig.endpoint` to the Worker Tunnel URL.
4. Start one Worker and verify it appears online in **Workers**.
5. Trigger a test job with `triggerType=api` and verify `executionMode=single` or broadcast behavior.
6. Scale the Worker deployment only after logs and result evidence are correct.

## Observability and evidence

Before production traffic:

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

Enable file logs and OpenTelemetry where required:

```toml
[observability.logging]
level = "info"
log_dir = "/var/log/tikeo"

[observability.tracing]
enabled = true
otlp_endpoint = "http://otel-collector:4318/v1/traces"
headers = []
```

Operator evidence should include: health/ready checks, bootstrap result, Worker online snapshot, triggered instance, task logs, audit event, and Notification Center delivery attempt if notifications are enabled.

## Backup and restore

Back up the database, not only container volumes. A useful backup runbook includes:

- Database dump schedule and restore test.
- Configuration file or Helm values version.
- Docker image tag/digest or release asset checksum.
- Secret names and rotation procedure, without printing secret values.
- Smoke command to run after restore.

SQLite backups are file copies only when the Server is stopped or the database is safely checkpointed. PostgreSQL/MySQL should use native backup tools.

## Upgrade and rollback

1. Read the release notes and image tags.
2. Apply the new Server image in staging.
3. Run health/ready checks and the management trigger smoke.
4. Verify Web static bundle loads and Worker Tunnel accepts one Worker.
5. Promote to production.
6. Roll back by restoring the previous image tag and config/Helm values. Database migrations may not always be reversible; test rollback before a production upgrade.

For release images, verify the binary version inside the Server container:

```bash
docker run --rm yhyzgn/tikeo-server:v0.2.9 --version
```

## Prerequisites

- A reachable database endpoint or a local SQLite-only evaluation path.
- Docker or Kubernetes access, depending on chosen path.
- DNS/TLS plan for Web/API and Worker Tunnel.
- Owner bootstrap plan and at least one app-scoped SDK API key.
- Secret storage for database URL and notification provider credentials.

## Verify

A production-ready verification should pass:

```bash
curl -fsS https://tikeo.example.com/readyz
curl -fsS https://tikeo.example.com/api-docs/openapi.json >/tmp/openapi.json
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```

For Kubernetes also check:

```bash
kubectl -n tikeo get pods,svc,ingress
kubectl -n tikeo logs deploy/tikeo-server --tail=120
```

## Troubleshooting

| Symptom | First checks |
| --- | --- |
| Web loads but API calls fail | Reverse proxy API path, CORS/origin, Server service DNS, auth token. |
| Worker never appears online | Worker Tunnel URL, gRPC/HTTP2 proxy support, TLS/mTLS CA/client certs, firewall egress. |
| Jobs stay pending | Worker capability mismatch, disabled Worker, namespace/app mismatch, queue/lease logs. |
| Notification test fails | Channel enabled, target configured, secret refs resolved, provider network egress. |
| SSE dashboard stale | Proxy buffering/timeouts; see [SSE realtime](./sse-realtime). |

## Production checklist

- [ ] Database uses PostgreSQL/MySQL or an explicitly accepted SQLite single-node path.
- [ ] Server/Web/Docs images are pinned by tag or digest.
- [ ] Worker Tunnel is reachable from Worker networks and does not require inbound Worker ports.
- [ ] TLS/mTLS decisions are documented for API and Worker Tunnel.
- [ ] Owner bootstrap is complete and closed.
- [ ] App-scoped SDK API keys are used by automation instead of human session tokens.
- [ ] Health/ready/OpenAPI/Worker/instance/log/audit smoke evidence is captured.
- [ ] Backup and rollback are tested before production traffic.
