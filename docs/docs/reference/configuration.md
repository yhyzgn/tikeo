---
title: Configuration reference
description: Complete Tikeo configuration defaults, environment override names, example TOML files, security, observability, storage, and Worker SDK defaults.
---

# Configuration reference

Tikeo Server configuration is a typed TOML + environment overlay loaded by `crates/tikeo-config/src/lib.rs`. The binary reads a file from `tikeo serve --config <path>` or `TIKEO_CONFIG`, then applies environment overrides using prefix `TIKEO` and double underscore separators. For example, `storage.database_url` becomes `TIKEO__STORAGE__DATABASE_URL`.

This page is operator-verified. Do not add keys here unless they exist in `TikeoConfig`, a committed config example, Helm values, or an SDK config type.

## Load order

The loader applies values in this order:

1. Rust defaults from `TikeoConfig::default()` and `Config::builder().set_default(...)`.
2. Optional TOML file passed by `--config` or `TIKEO_CONFIG`.
3. Environment variables using `Environment::with_prefix("TIKEO").separator("__")`.

Example:

```bash
TIKEO__SERVER__LISTEN_ADDR=0.0.0.0:19090 \
TIKEO__SERVER__WORKER_TUNNEL_ADDR=0.0.0.0:19998 \
TIKEO__STORAGE__DATABASE_URL='sqlite:///tmp/tikeo-quickstart.db?mode=rwc' \
cargo run --bin tikeo -- serve --config config/dev.toml
```

Use TOML for non-secret defaults and environment/Secret injection for secrets, DB URLs, certificate paths, and environment-specific endpoints.

## Committed config files

| File | Purpose | Notable values |
| --- | --- | --- |
| `config/dev.toml` | Local source evaluation | HTTP `0.0.0.0:9090`, Worker Tunnel `0.0.0.0:9998`, SQLite `sqlite://.dev/tikeo-dev.db?mode=rwc`, `timestamp_offset="+08:00"`, OIDC off, TLS off. |
| `config/container.toml` | Container default used by root `Dockerfile` | SQLite `sqlite:///data/tikeo.db?mode=rwc`, logging `info`, alert retry on, alert env refs enabled. |
| `config/postgres.toml` | PostgreSQL/CockroachDB example | `postgres://tikeo:tikeo@postgres:5432/tikeo`, comment for `TIKEO__STORAGE__DATABASE_URL`. |
| `config/mysql.toml` | MySQL example | `mysql://tikeo:tikeo@mysql:3306/tikeo`, `timestamp_offset="+08:00"`, `utf8mb4` reminder. |
| `config/raft.toml` | Raft-shape cluster metadata example | `mode="raft"`, static peers, secret-injected `transport_token` comment. |

## Complete default-value table

These defaults come from `crates/tikeo-config/src/lib.rs` unless a committed example overrides them.

| Config key | Default | Environment variable | Notes |
| --- | --- | --- | --- |
| `server.listen_addr` | `0.0.0.0:9090` | `TIKEO__SERVER__LISTEN_ADDR` | HTTP API, health, readiness, metrics, OpenAPI, and gateway surface. |
| `server.worker_tunnel_addr` | `0.0.0.0:9998` | `TIKEO__SERVER__WORKER_TUNNEL_ADDR` | gRPC/HTTP2 Worker Tunnel. Workers dial this endpoint outbound. |
| `storage.database_url` | `sqlite://.dev/tikeo-dev.db?mode=rwc` | `TIKEO__STORAGE__DATABASE_URL` | SeaORM/sqlx URL. Container example uses `/data/tikeo.db`; production should use PostgreSQL or MySQL. |
| `storage.timestamp_offset` | `+00:00` | `TIKEO__STORAGE__TIMESTAMP_OFFSET` | Parsed at startup. `config/dev.toml` and `config/mysql.toml` use `+08:00`; account for this in timestamp comparisons. |
| `cluster.mode` | `standalone` | `TIKEO__CLUSTER__MODE` | Accepted values are `standalone` and `raft`. In Kubernetes production HA, use Helm Raft mode with StatefulSet/headless peers; only the elected Leader schedules. |
| `cluster.node_id` | `standalone` | `TIKEO__CLUSTER__NODE_ID` | Stable node ID in cluster status and raft metadata. |
| `cluster.peers` | `[]` | `TIKEO__CLUSTER__PEERS` | Static peer list shape with `node_id` and `endpoint`. Prefer TOML/Helm for arrays. |
| `cluster.transport_token` | unset | `TIKEO__CLUSTER__TRANSPORT_TOKEN` | Optional shared token for internal raft HTTP transport; never commit real values. |
| `auth.local_login_enabled` | `true` | `TIKEO__AUTH__LOCAL_LOGIN_ENABLED` | Local username/password login toggle. |
| `auth.api_tokens.default_ttl_seconds` | `43200` | `TIKEO__AUTH__API_TOKENS__DEFAULT_TTL_SECONDS` | 12 hours. |
| `auth.api_tokens.min_ttl_seconds` | `300` | `TIKEO__AUTH__API_TOKENS__MIN_TTL_SECONDS` | 5 minutes. |
| `auth.api_tokens.max_ttl_seconds` | `2592000` | `TIKEO__AUTH__API_TOKENS__MAX_TTL_SECONDS` | 30 days. |
| `auth.oidc.enabled` | `false` | `TIKEO__AUTH__OIDC__ENABLED` | When enabled, authorize/callback require issuer/client credentials and mapped identities. |
| `auth.oidc.issuer_url` | unset | `TIKEO__AUTH__OIDC__ISSUER_URL` | OIDC issuer URL. |
| `auth.oidc.client_id` | unset | `TIKEO__AUTH__OIDC__CLIENT_ID` | OAuth/OIDC client ID. |
| `auth.oidc.client_secret` | unset | `TIKEO__AUTH__OIDC__CLIENT_SECRET` | Secret; keep in platform Secret store. |
| `auth.oidc.scopes` | `openid`, `profile`, `email` | `TIKEO__AUTH__OIDC__SCOPES` | Use TOML for list shape if env parsing is ambiguous. |
| `transport_security.http.tls_enabled` | `false` | `TIKEO__TRANSPORT_SECURITY__HTTP__TLS_ENABLED` | Enables TLS on the HTTP listener itself. |
| `transport_security.http.mtls_required` | `false` | `TIKEO__TRANSPORT_SECURITY__HTTP__MTLS_REQUIRED` | Requires HTTP client certs; also requires TLS and client CA. |
| `transport_security.http.cert_path` | unset | `TIKEO__TRANSPORT_SECURITY__HTTP__CERT_PATH` | Readable server certificate path. |
| `transport_security.http.key_path` | unset | `TIKEO__TRANSPORT_SECURITY__HTTP__KEY_PATH` | Readable server private key path. |
| `transport_security.http.client_ca_path` | unset | `TIKEO__TRANSPORT_SECURITY__HTTP__CLIENT_CA_PATH` | Client CA bundle for mTLS. |
| `transport_security.worker_tunnel.tls_enabled` | `false` | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__TLS_ENABLED` | Enables TLS on the Worker Tunnel listener. |
| `transport_security.worker_tunnel.mtls_required` | `false` | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__MTLS_REQUIRED` | Requires Worker client certificates. |
| `transport_security.worker_tunnel.cert_path` | unset | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__CERT_PATH` | Worker Tunnel server certificate. |
| `transport_security.worker_tunnel.key_path` | unset | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__KEY_PATH` | Worker Tunnel private key. |
| `transport_security.worker_tunnel.client_ca_path` | unset | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__CLIENT_CA_PATH` | Worker client CA bundle for mTLS. |
| `observability.logging.level` | `info` | `TIKEO__OBSERVABILITY__LOGGING__LEVEL` | Used when `RUST_LOG` is not provided. Supported practical levels: `debug`, `info`, `warn`, `error`. |
| `observability.logging.log_dir` | unset | `TIKEO__OBSERVABILITY__LOGGING__LOG_DIR` | When set, writes `tikeo.log` in addition to console output. |
| `observability.tracing.enabled` | `false` | `TIKEO__OBSERVABILITY__TRACING__ENABLED` | Enables OTLP export beyond local spans. |
| `observability.tracing.otlp_endpoint` | unset | `TIKEO__OBSERVABILITY__TRACING__OTLP_ENDPOINT` | Required when tracing export is enabled. Example: `http://otel-collector:4318/v1/traces`. |
| `observability.tracing.headers` | `[]` | `TIKEO__OBSERVABILITY__TRACING__HEADERS` | Header names for exporter auth/tenancy. Keep values out of status APIs. |
| `alert_retry.enabled` | `true` | `TIKEO__ALERT_RETRY__ENABLED` | Background retry worker for alert delivery attempts. |
| `alert_retry.interval_seconds` | `60` | `TIKEO__ALERT_RETRY__INTERVAL_SECONDS` | Due-attempt scan interval. |
| `alert_retry.batch_size` | `50` | `TIKEO__ALERT_RETRY__BATCH_SIZE` | Max due attempts scanned per iteration. |
| `alert_retry.max_attempts` | `3` | `TIKEO__ALERT_RETRY__MAX_ATTEMPTS` | Attempts before dead-lettering. |
| `alert_retry.backoff_seconds` | `300` | `TIKEO__ALERT_RETRY__BACKOFF_SECONDS` | Delay before retry. |
| `alert_secrets.allow_env_refs` | `true` | `TIKEO__ALERT_SECRETS__ALLOW_ENV_REFS` | Allows alert provider secrets to reference environment variables. |
| `alert_secrets.env_prefix` | `TIKEO_ALERT_SECRET_` | `TIKEO__ALERT_SECRETS__ENV_PREFIX` | Required prefix for env secret names in production. |
| `script_governance.release_signature_secret_ref` | unset | `TIKEO__SCRIPT_GOVERNANCE__RELEASE_SIGNATURE_SECRET_REF` | Optional `env:NAME` secret reference for script release signature verification. |

## Copy-paste local config command

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
```

For isolated tests, override ports and DB path instead of editing `config/dev.toml`:

```bash
TIKEO__SERVER__LISTEN_ADDR=127.0.0.1:19090 \
TIKEO__SERVER__WORKER_TUNNEL_ADDR=127.0.0.1:19998 \
TIKEO__STORAGE__DATABASE_URL='sqlite:///tmp/tikeo-smoke.db?mode=rwc' \
./target/debug/tikeo serve --config config/dev.toml
```

## Storage URLs

| Backend | Example | Operational note |
| --- | --- | --- |
| SQLite dev | `sqlite://.dev/tikeo-dev.db?mode=rwc` | Fast local path; avoid sharing one file across concurrent Server processes. |
| SQLite container | `sqlite:///data/tikeo.db?mode=rwc` | Persist `/data` with a volume/PVC. |
| PostgreSQL | `postgres://tikeo:change-me@postgres.example:5432/tikeo?sslmode=require` | Preferred production default when you need shared, managed storage. |
| CockroachDB | `postgres://root@cockroach:26257/tikeo?sslmode=disable` | Uses PostgreSQL wire protocol shape. |
| MySQL | `mysql://tikeo:change-me@mysql.example:3306/tikeo` | Use MySQL 8.0+/8.4 LTS and `utf8mb4` for Unicode payload/log support. |

SQLite startup sets compatibility pragmas such as WAL, busy timeout, and foreign keys. New schema changes must still be represented as explicit SeaORM migrations, not ad-hoc post-connect patches.

## Authentication and session configuration

Local login is enabled by default. First deployment creates a bootstrap Owner through `/api/v1/auth/bootstrap/register`. Human sessions use local opaque bearer sessions. OIDC provider tokens are used only to fetch external identity and issue a local session; they are not stored or reused as the local login state.

OIDC gotchas:

- `auth.oidc.enabled=true` requires issuer URL, client ID, and client secret for authorize/callback.
- An external identity must be mapped in Tikeo before OIDC callback can issue a local session.
- Keep `auth.local_login_enabled=true` until you have verified OIDC login and recovery access.

API-key gotchas:

- Human bearer tokens and SDK `x-tikeo-api-key` credentials are separate.
- SDK API keys are created under `/api/v1/management/api-keys` and should be scoped to namespace/app/service-account permissions.
- SDK clients should load keys from `TIKEO_MANAGEMENT_API_KEY` or platform Secrets, not repository files.

## Transport security

Default local mode is plaintext for both listeners. Enable TLS/mTLS when traffic crosses a host, cluster, VPC, customer network, public LB, or untrusted path.

```toml
[transport_security.http]
tls_enabled = true
mtls_required = false
cert_path = "/etc/tikeo/tls/http.crt"
key_path = "/etc/tikeo/tls/http.key"

[transport_security.worker_tunnel]
tls_enabled = true
mtls_required = true
cert_path = "/etc/tikeo/tls/worker-tunnel.crt"
key_path = "/etc/tikeo/tls/worker-tunnel.key"
client_ca_path = "/etc/tikeo/tls/worker-client-ca.crt"
```

TLS requires readable `cert_path` and `key_path`. mTLS additionally requires `tls_enabled=true` and readable `client_ca_path`. The HTTP and Worker Tunnel listeners are independent; terminating HTTPS at ingress does not automatically enable TLS inside the Tikeo process.

## Observability

Process logs always go to the console. `observability.logging.level` sets the default filter when `RUST_LOG` is not provided. Set `observability.logging.log_dir` for VM/systemd installs where you also want `tikeo.log` on disk.

```toml
[observability.logging]
level = "info"
log_dir = "/var/log/tikeo"

[observability.tracing]
enabled = true
otlp_endpoint = "http://otel-collector:4318/v1/traces"
headers = ["x-otlp-tenant"]
```

If `observability.tracing.enabled=true`, configure `observability.tracing.otlp_endpoint`. Keep collector credentials in the environment or platform Secret store. Verify observability status through the Web console or API before assuming traces are exported.

## Cluster mode

`standalone` is the operational default for single-server installs. `raft` is the production multi-pod Server HA mode when deployed with stable node IDs, a static peer list, external database storage, and an internal transport token. The scheduling model is active-passive: only the elected Raft Leader with a persisted fencing token reports `canSchedule=true` and runs schedule/dispatch/retry ownership loops; followers skip those loops. In raft mode, internal append traffic can require `x-tikeo-raft-token` when `cluster.transport_token` is configured. Do not add Redis/Dragonfly distributed locks for core scheduler ownership; future multi-active scheduling should use Raft/fencing shard ownership.

Example shape:

```toml
[cluster]
mode = "raft"
node_id = "tikeo-0"
# transport_token = "inject-from-secret"
peers = [
  { node_id = "tikeo-0", endpoint = "http://tikeo-0.tikeo-headless:9090" },
  { node_id = "tikeo-1", endpoint = "http://tikeo-1.tikeo-headless:9090" },
  { node_id = "tikeo-2", endpoint = "http://tikeo-2.tikeo-headless:9090" },
]
```

## Docker Compose environment matrix

Compose uses user-facing `TIKEO_*` variables from `deploy/compose/tikeo.env.example`, while the Server itself consumes `TIKEO__...` nested overrides.

| Compose variable | Default | Meaning |
| --- | --- | --- |
| `TIKEO_IMAGE` | `yhyzgn/tikeo-server:local` | Server image tag. |
| `TIKEO_WEB_IMAGE` | `yhyzgn/tikeo-web:local` | Web image tag. |
| `TIKEO_HTTP_PORT` | `9090` | Host port mapped to container `9090`. |
| `TIKEO_WORKER_TUNNEL_PORT` | `9998` | Host port mapped to container `9998`. |
| `TIKEO_WEB_PORT` | `8080` | Host port mapped to Web nginx `80`. |
| `TIKEO_PROMETHEUS_PORT` | `9091` | Optional observability profile. |
| `TIKEO_POSTGRES_*` | see `docker-compose.postgres.yml` | DB name/user/password/port/volume for PostgreSQL stack. |
| `TIKEO_MYSQL_*` | see `docker-compose.mysql.yml` | DB name/user/password/root password/port/volume for MySQL stack. |

## Helm mapping

The chart writes Server config through a ConfigMap and injects database URL secrets for external storage. Important values:

| Helm value | Default | Maps to |
| --- | --- | --- |
| `server.httpPort` | `9090` | `server.listen_addr` container port. |
| `server.workerTunnelPort` | `9998` | `server.worker_tunnel_addr` container port. |
| `server.storage.mode` | `sqlite` | SQLite PVC or external DB Secret mode. |
| `server.storage.existingSecret` | empty | Secret containing DB URL. |
| `server.storage.databaseUrlSecretKey` | `database-url` | Injects `TIKEO__STORAGE__DATABASE_URL`. |
| `server.tls.http.enabled` | `false` | HTTP listener TLS config. |
| `server.tls.workerTunnel.enabled` | `false` | Worker Tunnel TLS config. |
| `server.tls.workerTunnel.mtlsRequired` | `false` | Worker mTLS. |
| `networkPolicy.enabled` | `false` | Renders NetworkPolicy while preserving outbound-only Worker model. |
| `gatewayApi.enabled` | `false` | Renders Gateway API resources for Worker Tunnel. |

## Worker SDK defaults

Server configuration is only half the deployment. Worker services also carry SDK-specific config. Common defaults across Rust, Go, Python, and Node helpers are:

| Field | Default | Meaning |
| --- | --- | --- |
| `endpoint` | provided by caller; demos use `http://127.0.0.1:9998` | Reachable Worker Tunnel endpoint, not necessarily the Server bind address. |
| `clientInstanceId` / `client_instance_id` | required; demos choose language-specific IDs | Stable client-side hint. Server assigns authoritative `worker_id`. |
| `namespace` | `default` in SDK helpers; demos often use `dev-alpha` or smoke-specific namespace | Dispatch and management scope. |
| `app` | `default` in SDK helpers; demos often use `orders` or `management` | Dispatch and management app scope. |
| `name` | client instance id where exposed | Operator-facing name. |
| `cluster` | `local` in Rust/Go/Python/Node helpers; Java Boot default can be `default` | Worker cluster or domain. |
| `region` | `local` in Rust/Go/Python/Node helpers; Java Boot default can be `default` | Region/zone metadata. |
| `version` | `dev` where exposed | Worker build version. |
| `heartbeatEvery` | `10s` or `10000 ms` | Lease renewal cadence. |
| `capabilities` | `[]` | Legacy metadata; routing should use structured capabilities. |
| `structuredCapabilities.tags` | `[]` | Operator tags. |
| `structuredCapabilities.sdkProcessors` | `[]` | Processor names used for dispatch routing. |
| `structuredCapabilities.scriptRunners` | `[]` | Language + sandbox backend declarations. |
| `structuredCapabilities.pluginProcessors` | `[]` | Plugin type + processor names. |
| `labels` | `{}` | Operational metadata such as `worker_pool`. |
| `election.enabled` | `true` in current registration helpers | Worker-cluster master-election metadata. |
| `election.domain` | blank | Blank means namespace/app/cluster/region domain. |
| `election.priority` | `100` | Lower values win deterministic election. |

Advertise only capabilities backed by real runtime support. Missing SRT, Deno, Docker, Podman, SQL, or plugin tooling should fail closed and should not be advertised just to make a demo look bigger.

## Deployment checklist

Before moving from local evaluation to shared deployment:

- Choose storage backend and confirm backups.
- Set `TIKEO__STORAGE__DATABASE_URL` from a Secret for PostgreSQL/MySQL.
- Decide whether the API listener terminates TLS itself or sits behind ingress TLS.
- Enable Worker Tunnel TLS/mTLS when workers cross trust boundaries.
- Keep business Workers separate from the Helm chart and connect them outbound.
- Bootstrap the first Owner, then create service accounts and SDK API keys for automation.
- Configure logging/OTel only after the collector/log path exists.
- Run the relevant smoke script and preserve evidence under `.dev/reports` or CI artifacts.

## Notification Center delivery

Notification Center has its own generic delivery worker, separate from `alert_retry`. It scans generic `notification_delivery_attempts` produced by notification policies and updates the associated `notification_messages`. The config shape is defined by `NotificationDeliveryConfig` in `crates/tikeo-config/src/lib.rs` and is present in both `config/dev.toml` and `config/container.toml`.

| Config key | Default | Environment variable | Notes |
| --- | --- | --- | --- |
| `notification_delivery.enabled` | `true` | `TIKEO__NOTIFICATION_DELIVERY__ENABLED` | Enables the generic Notification Center delivery worker. |
| `notification_delivery.public_console_base_url` | unset | `TIKEO__NOTIFICATION_DELIVERY__PUBLIC_CONSOLE_BASE_URL` | Optional externally reachable Web base URL for public execution console links in provider cards. |
| `notification_delivery.interval_seconds` | `60` | `TIKEO__NOTIFICATION_DELIVERY__INTERVAL_SECONDS` | Interval between due-attempt scans. |
| `notification_delivery.batch_size` | `50` | `TIKEO__NOTIFICATION_DELIVERY__BATCH_SIZE` | Maximum due attempts scanned per worker iteration. |
| `notification_delivery.max_attempts` | `3` | `TIKEO__NOTIFICATION_DELIVERY__MAX_ATTEMPTS` | Attempts before the generic delivery attempt is dead-lettered. |
| `notification_delivery.backoff_seconds` | `300` | `TIKEO__NOTIFICATION_DELIVERY__BACKOFF_SECONDS` | Delay before the next generic delivery retry. |

Example override:

```bash
TIKEO__NOTIFICATION_DELIVERY__ENABLED=true \
TIKEO__NOTIFICATION_DELIVERY__INTERVAL_SECONDS=30 \
TIKEO__NOTIFICATION_DELIVERY__BATCH_SIZE=100 \
TIKEO__NOTIFICATION_DELIVERY__MAX_ATTEMPTS=5 \
TIKEO__NOTIFICATION_DELIVERY__BACKOFF_SECONDS=120 \
cargo run --bin tikeo -- serve --config config/dev.toml
```

Use `alert_retry` for compatibility alert delivery attempts and `notification_delivery` for Notification Center messages. Do not tune one queue expecting it to change the other. See [Notification Center reference](./notification-center) for channel, policy, retry, DLQ, and redaction behavior.

## Prerequisites

Use the setup, authentication, and access requirements described in this page before running any command. For local examples, start the Server with `config/dev.toml`, use `127.0.0.1` as the client host, and keep tokens in shell variables rather than pasted into files.

## Verify

After following the page, verify the result with the documented API, UI, build, smoke, or deployment checks. A valid verification includes the command that was run, the route or file that was inspected, and the observed status or artifact.

## Troubleshooting

When a step fails, first capture the exact command, response status, and Server log window. Then check authentication, namespace/app scope, Worker eligibility, storage readiness, and proxy behavior before changing production configuration.

## Production checklist

- [ ] Secrets are referenced through environment or platform secret mechanisms and are not written into examples.
- [ ] Commands have been adapted from local `127.0.0.1` to the real host, TLS, and authentication model.
- [ ] Rollback and evidence collection are documented for the changed surface.
- [ ] Operators can repeat the verification without private shell history or hidden state.
