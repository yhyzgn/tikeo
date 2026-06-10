---
title: Configuration reference
description: Tikeo configuration files, environment overrides, ports, storage URLs, TLS/mTLS, observability, alert retry, and script governance parameters.
---

# Configuration reference

Tikeo reads a TOML config passed to `tikeo serve --config <path>`. Deployment layers may override nested keys with environment variables such as `TIKEO__STORAGE__DATABASE_URL`. Keep committed config examples small and move production secrets into platform Secret stores.

## Copy-paste local config command

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
```

## Committed config files

| File | Purpose | Storage |
|---|---|---|
| `config/dev.toml` | Local source evaluation | SQLite file `tikeo-dev.db` |
| `config/container.toml` | Container default | SQLite `/data/tikeo.db` |
| `config/postgres.toml` | PostgreSQL/CockroachDB example | `postgres://...` |
| `config/mysql.toml` | MySQL example | `mysql://...` |
| `config/raft.toml` | Cluster/raft planning example | see file |

## Server ports

| Config key | Default examples | Meaning |
|---|---|---|
| `server.listen_addr` | `0.0.0.0:9090` | HTTP API, health, readiness, metrics, and embedded/API gateway surface. |
| `server.worker_tunnel_addr` | `0.0.0.0:9998` | Worker Tunnel gRPC/HTTP2 listener. Workers connect outbound to this endpoint. |

In Docker Compose these map to `TIKEO_HTTP_PORT` and `TIKEO_WORKER_TUNNEL_PORT`. In Helm they map to `server.httpPort` and `server.workerTunnelPort`.

## Storage URLs

| Backend | Example |
|---|---|
| SQLite dev | `sqlite://tikeo-dev.db?mode=rwc` |
| SQLite container | `sqlite:///data/tikeo.db?mode=rwc` |
| PostgreSQL | `postgres://tikeo:change-me@postgres.example:5432/tikeo?sslmode=require` |
| CockroachDB | `postgres://root@cockroach:26257/tikeo?sslmode=disable` |
| MySQL | `mysql://tikeo:change-me@mysql.example:3306/tikeo` |

Environment override:

```bash
TIKEO__STORAGE__DATABASE_URL='postgres://tikeo:change-me@postgres:5432/tikeo?sslmode=require' \
  ./target/release/tikeo serve --config config/container.toml
```

Schema changes must go through explicit SeaORM migrations. Do not document manual database mutation as a supported configuration path.

## Authentication and API tokens

```toml
[auth]
local_login_enabled = true

[auth.api_tokens]
default_ttl_seconds = 43200
min_ttl_seconds = 300
max_ttl_seconds = 2592000
```

Use local login for development. For shared environments, configure OIDC and keep API-key/service-account credentials scoped to the app boundary.

## Transport security

```toml
[transport_security.http]
tls_enabled = false
mtls_required = false

[transport_security.worker_tunnel]
tls_enabled = false
mtls_required = false
```

Enable HTTP TLS when exposing the API directly. Enable Worker Tunnel TLS/mTLS when workers cross hosts, clusters, VPCs, or trust boundaries. In Helm, certificate files are mounted from Secrets and referenced by generated transport-security config.

## Observability

```toml
[observability.logging]
level = "info"
# log_dir = "./logs"

[observability.tracing]
enabled = false
headers = []
# otlp_endpoint = "http://otel-collector:4318/v1/traces"
```

Keep `info` as the default log level for operations. Set `log_dir` for VM/systemd deployments. Use OTLP only when a collector is reachable and approved for the environment.

## Alert retry and alert secrets

```toml
[alert_retry]
enabled = true
interval_seconds = 60
batch_size = 50
max_attempts = 3
backoff_seconds = 300

[alert_secrets]
allow_env_refs = true
env_prefix = "TIKEO_ALERT_SECRET_"
```

Alert channel JSON may reference secrets through `env:NAME` indirection. Do not commit SMTP, webhook, or API credentials.

## Script governance

```toml
[script_governance]
# release_signature_secret_ref = "env:TIKEO_SCRIPT_RELEASE_SECRET"
```

If script release signing is enabled, store the secret in the deployment platform and pass only a reference into config.


## SDK and worker configuration

Server configuration is only half of a deployment. Worker services also need SDK dependency selection, Worker Tunnel endpoint wiring, identity scope, capabilities, labels, sandbox tool cache paths, and optional management-client credentials.

For Java-specific Boot, plain Java, and non-Boot Spring examples, see [Java SDK and Spring Boot Starter](../sdks/java-spring-boot).

### Worker runtime fields shared by SDKs

These fields are common to Java, Rust, Go, Python, and Node.js Worker SDKs. Language-specific wrappers expose them as Java records/properties, Rust structs, Go structs, Python dataclasses, TypeScript classes, or Spring Boot configuration properties.

| Field | Default in SDK helpers | Meaning |
| --- | --- | --- |
| `endpoint` | usually `http://127.0.0.1:9998` in demos | Worker Tunnel endpoint reachable from the worker process. Use a Service/LB/DNS name in real deployments, not necessarily the server bind address. |
| `clientInstanceId` / `client_instance_id` | required for core SDK helpers; Java Boot can generate/persist it | Stable client-side hint. The server still assigns the authoritative `worker_id`. |
| `namespace` | `default` | Tenant/environment namespace used for dispatch and management scoping. |
| `app` | `default` | Application scope used for routing and management operations. |
| `cluster` | `local` in Rust/Go/Python/Node helpers; Java Boot default is `default` | Worker cluster or environment shard. |
| `region` | `local` in Rust/Go/Python/Node helpers; Java Boot default is `default` | Worker region/zone. |
| `name` | usually the client instance id | Operator-facing worker name when the SDK exposes it. |
| `version` | `dev` in Go/Python/Node helpers | Worker/application build version when the SDK exposes it. |
| `heartbeatEvery` / `heartbeat-interval-millis` | `10s` / `10000` | Worker lease renewal cadence. |
| `capabilities` | `[]` | Legacy/operator metadata. Prefer structured capabilities for dispatch routing when available. |
| `structuredCapabilities` | empty | SDK processors, script runners, plugin processors, and structured tags used for routing. |
| `labels` | `{}` | Free-form operational metadata such as `worker_pool`, `runtime`, `team`, or `tier`. |
| `election.enabled` | `true` | Worker-cluster master election flag in registration. |
| `election.domain` | blank | Blank means `namespace/app/cluster/region`. |
| `election.priority` | `100` | Deterministic election priority; lower values win. |

### Worker deployment checklist

- Add one SDK dependency per service and let the package manager resolve transitive Tikeo modules.
- Point worker SDKs at `server.worker_tunnel_addr` through the reachable Service/LB/DNS name, not necessarily the server bind address.
- Set namespace/app/cluster/region consistently across workers and management clients.
- Advertise only capabilities backed by real runtime support; missing tools should fail closed instead of being advertised.
- Persist SDK state/tool cache directories such as `~/.tikeo/workers` and `~/.tikeo/sandbox-tools/*` when stable identity or offline startup matters.
- Inject API keys and mirrored installer URLs from platform Secrets/config, not committed files.

## Environment override rule

Nested config keys use double underscores:

| Environment variable | Config key |
|---|---|
| `TIKEO__STORAGE__DATABASE_URL` | `storage.database_url` |
| `TIKEO__ALERT_SECRETS__ALLOW_ENV_REFS` | `alert_secrets.allow_env_refs` |
| `TIKEO__ALERT_SECRETS__ENV_PREFIX` | `alert_secrets.env_prefix` |

Prefer committed TOML for non-secret defaults and environment/Secret injection for credentials and deployment-specific endpoints.
