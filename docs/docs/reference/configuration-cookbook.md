---
title: Configuration cookbook
description: Scenario-based configuration recipes for local development, production databases, TLS/mTLS, OIDC, observability, notifications, and Worker SDK settings.
keywords: [tikeo configuration cookbook, environment variables, tls, oidc, observability]
---

# Configuration cookbook

Use [Configuration reference](./configuration) when you need every default. Use this cookbook when you know the scenario and need a working configuration shape.

## How config is loaded

Tikeo reads a TOML file and then applies environment overrides. The override format is:

```text
TIKEO__SECTION__KEY=value
TIKEO__SECTION__SUBSECTION__KEY=value
```

Examples:

```bash
export TIKEO__STORAGE__DATABASE_URL='postgres://tikeo:secret@postgres:5432/tikeo'
export TIKEO__SERVER__LISTEN_ADDR='0.0.0.0:9090'
export TIKEO__NOTIFICATION_DELIVERY__ENABLED='true'
```

## Recipe: local development

Use `config/dev.toml`:

```toml
[server]
listen_addr = "0.0.0.0:9090"
worker_tunnel_addr = "0.0.0.0:9998"

[storage]
database_url = "sqlite://tikeo-dev.db?mode=rwc"
timestamp_offset = "+08:00"
```

Run:

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://127.0.0.1:9090/readyz
```

Use this only for local work. Client URLs should use `127.0.0.1`, not the bind address.

## Recipe: PostgreSQL production

```toml
[storage]
database_url = "postgres://tikeo:tikeo@postgres:5432/tikeo"
```

Recommended environment override:

```bash
export TIKEO__STORAGE__DATABASE_URL="postgres://tikeo:${TIKEO_DB_PASSWORD}@postgres:5432/tikeo"
```

Checklist:

- Database exists before Server start.
- User can create/update migration tables.
- Connection uses TLS if required by your database provider.
- Backups and restore tests are owned by database operations.

## Recipe: MySQL production

```toml
[storage]
database_url = "mysql://tikeo:tikeo@mysql:3306/tikeo"
timestamp_offset = "+08:00"
```

Use `utf8mb4` for full Unicode payload/log support. Run repository database compatibility tests before declaring a new MySQL version supported in your environment.

## Recipe: HTTP TLS at reverse proxy

Keep Tikeo HTTP plaintext inside the private network and terminate TLS at ingress/proxy:

```toml
[server]
listen_addr = "0.0.0.0:9090"
```

Proxy requirements:

- Forward `/api/*` and `/api-docs/openapi.json` to Server.
- Forward Web static routes to Web nginx.
- Preserve SSE behavior for dashboard/instance streams; disable buffering and use long read timeouts.
- Set secure cookies and normal `X-Forwarded-*` headers if your deployment uses them.

## Recipe: Worker Tunnel TLS or mTLS

For local plaintext:

```toml
[transport_security.worker_tunnel]
tls_enabled = false
mtls_required = false
```

For cross-network production, enable TLS or mTLS with files mounted from Secret/config management. The exact key names are documented in [Configuration reference](./configuration); Helm exposes the same intent with values such as `server.tls.workerTunnel.mtlsRequired`.

mTLS rollout order:

1. Issue CA and server certificate.
2. Configure Server Worker Tunnel TLS.
3. Configure one test Worker with CA trust.
4. Enable client certificates for that Worker.
5. Only then require mTLS globally.

## Recipe: local login plus OIDC readiness

Local login is useful for bootstrap and small internal deployments:

```toml
[auth]
local_login_enabled = true

[auth.oidc]
enabled = false
scopes = ["openid", "profile", "email"]
```

When enabling OIDC, set issuer/client values through environment or secret management, then verify login in a staging environment before disabling local fallback.

## Recipe: Observability

```toml
[observability.logging]
level = "info"
log_dir = "/var/log/tikeo"

[observability.tracing]
enabled = true
otlp_endpoint = "http://otel-collector:4318/v1/traces"
headers = []
```

Use file logs for incident reconstruction and OTel for distributed trace correlation. If you add custom tracing headers, do not store bearer tokens or provider secrets in plain config.

## Recipe: Notification Center delivery worker

```toml
[notification_delivery]
enabled = true
interval_seconds = 60
batch_size = 50
max_attempts = 3
backoff_seconds = 300
```

Operational meaning:

- `interval_seconds`: how often the worker scans due attempts.
- `batch_size`: max attempts per scan.
- `max_attempts`: attempts before DLQ.
- `backoff_seconds`: delay before retry.

Use the Web Notification Center queue view or `notification-delivery-attempts:queue-status` endpoint to inspect retry/DLQ state.

## Recipe: Worker SDK defaults

Worker SDKs use language-specific config objects, but the same concepts apply:

| Concept | Typical local value | Production value |
| --- | --- | --- |
| endpoint | `http://127.0.0.1:9998` | Worker Tunnel URL with TLS/mTLS if enabled |
| namespace | service namespace | platform-approved namespace |
| app | app name | platform-approved app |
| workerPool | `default` | pool name by capacity/SLO/security |
| heartbeat interval | SDK default | tune only after observing leases and network |
| processor names | demo names | stable service-owned names |

## Recipe: release image pinning

Use explicit tags or digests:

```bash
docker pull yhyzgn/tikeo-server:v0.2.9
docker run --rm yhyzgn/tikeo-server:v0.2.9 --version
```

Keep Server, Web, and Docs image tags aligned with the release version unless you intentionally test a mixed version.

## Prerequisites

- You know which deployment mode you are configuring.
- You have a database URL and secret-management path.
- You know whether Worker Tunnel crosses trust boundaries.
- You know whether Notification Center delivery should run in this environment.

## Verify

For any recipe, verify at least:

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

Then run the specific smoke for your deployment path.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Config appears ignored | Confirm `--config`, `TIKEO_CONFIG`, and `TIKEO__...` env spelling. |
| Database connection fails | URL scheme, credentials, DNS, TLS requirements, database exists. |
| Worker TLS fails | CA path, SNI/hostname, client cert, ingress protocol. |
| OIDC login loops | issuer URL, redirect URI, cookie security, clock skew. |
| Notifications stay retrying | provider network egress, channel credentials, retry worker enabled. |

## Production checklist

- [ ] All secrets are injected through environment/platform Secret, not committed TOML.
- [ ] Database URL, TLS/mTLS, OIDC, and notification delivery settings are documented.
- [ ] Config changes include verification commands and rollback notes.
- [ ] Local bind addresses are not used as client URLs.
