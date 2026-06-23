---
title: Configuration reference
description: Complete Server and Worker configuration reference, environment variables, defaults, mount paths, structured database settings, and deployment guidance.
---

# Configuration reference

Tikeo has two configuration surfaces:

1. **Server configuration** controls the control plane, storage, auth, cluster mode, notification delivery, TLS, and observability. Docker/Compose uses `config/tikeo.yml` mounted as `/config/tikeo.yml`.
2. **Worker configuration** lives in SDKs or application configuration. Java Spring Boot exposes `tikeo.worker.*`; other SDKs expose equivalent `WorkerConfig` fields and demo environment variables.

Server config load order is:

1. Rust defaults from `crates/tikeo-config/src/lib.rs`.
2. Optional file passed by `tikeo serve --config <path>` or `TIKEO_CONFIG`.
3. Environment overrides with prefix `TIKEO` and double underscores, for example `storage.database.host` → `TIKEO__STORAGE__DATABASE__HOST`.

Prefer the mounted config file for normal deployment. Use environment overrides for Kubernetes Secrets, emergency overrides, or platforms where mounting a file is inconvenient.

## Runtime files and mount paths

| Path | Used by | Meaning | Mount guidance |
| --- | --- | --- | --- |
| `/config/tikeo.yml` | Dockerfile, Compose, Kubernetes, Helm | Server config selected by `serve --config /config/tikeo.yml`. | Mount read-only from host path, ConfigMap, or Secret. |
| `/config/tls` | TLS/mTLS config | Certificate, private key, and CA files referenced by `transport_security.*`. | Mount read-only. Never bake private keys into images. |
| `/data/tikeo.db` | SQLite mode | SQLite database file from `storage.database.path=/data/tikeo.db`. | Persist `/data` when SQLite data must survive restarts. |
| `/logs/tikeo.log` | Optional file logging | File log created when `observability.logging.log_dir=/logs`. | Optional; stdout logging is always emitted. |
| `/etc/tikeo/tikeo.yml` | systemd/bare metal | Conventional host config file. | Own by root/deployment automation; readable by the process. |
| `/var/lib/tikeo` | systemd/bare metal | Durable local state, usually SQLite on a VM. | Own by the `tikeo` user; include in backups if SQLite is used. |
| `/var/log/tikeo` | systemd/bare metal | Host file logs. | Create before startup and rotate with host policy. |

## Committed config files

| File | Purpose | Notes |
| --- | --- | --- |
| `config/tikeo.yml` | Production/container template | Single formal deployment entry. Defaults to SQLite `/data/tikeo.db`, includes commented PostgreSQL/MySQL/Raft/TLS examples. |
| `config/dev.toml` | Local source development | Keeps the fast dev path with `.dev/tikeo-dev.db`. |

## Structured database configuration

Tikeo uses structured `storage.database.*` fields for Server persistence. Passwords with `@`, `/`, `:`, or `#` can be written as normal values; Tikeo builds the internal sqlx/SeaORM connection URL and percent-encodes credentials automatically.

```yaml
storage:
  database:
    type: postgres
    host: postgres
    port: 5432
    username: tikeo
    password: "p@ss/word:with#chars"
    database: tikeo
    params:
      sslmode: disable
```


## Server configuration table

The following is the Complete default-value table for Server settings: config key, environment variable, requirement level, default, and operational meaning.

| Config key | Environment variable | Required? | Default | Meaning |
| --- | --- | --- | --- | --- |
| `server.listen_addr` | `TIKEO__SERVER__LISTEN_ADDR` | No | `0.0.0.0:9090` | HTTP API, health, readiness, metrics, OpenAPI, and Web API target bind address. |
| `server.worker_tunnel_addr` | `TIKEO__SERVER__WORKER_TUNNEL_ADDR` | No | `0.0.0.0:9998` | gRPC/HTTP2 Worker Tunnel bind address. Workers dial this endpoint outbound. |
| `storage.database.type` | `TIKEO__STORAGE__DATABASE__TYPE` | No | `sqlite` | `sqlite`, `postgres`, `mysql`, or `cockroachdb`. |
| `storage.database.path` | `TIKEO__STORAGE__DATABASE__PATH` | SQLite mode | `.dev/tikeo-dev.db`; production template `/data/tikeo.db` | SQLite file path. Persist `/data` in containers. |
| `storage.database.host` | `TIKEO__STORAGE__DATABASE__HOST` | Network DB | `127.0.0.1` if omitted | Host for PostgreSQL/MySQL/CockroachDB. |
| `storage.database.port` | `TIKEO__STORAGE__DATABASE__PORT` | No | Postgres `5432`, MySQL `3306` | Network database port. |
| `storage.database.username` | `TIKEO__STORAGE__DATABASE__USERNAME` | Usually yes for network DB | unset | Database username. |
| `storage.database.password` | `TIKEO__STORAGE__DATABASE__PASSWORD` | Usually yes for network DB | unset | Database password; special characters are supported without manual URL escaping. |
| `storage.database.database` | `TIKEO__STORAGE__DATABASE__DATABASE` | Network DB | `tikeo` if omitted | Database/schema name. |
| `storage.database.params.*` | Prefer file config | No | SQLite uses `mode=rwc` when params are empty | Query parameters such as `sslmode=disable`. |
| `storage.timestamp_offset` | `TIKEO__STORAGE__TIMESTAMP_OFFSET` | No | `+00:00` | Offset used when writing DB timestamps. |
| `cluster.mode` | `TIKEO__CLUSTER__MODE` | No | `standalone` | `standalone` or `raft`. Use raft for multi-pod Server HA. |
| `cluster.node_id` | `TIKEO__CLUSTER__NODE_ID` | Raft: yes | `standalone` | Stable node id; in Kubernetes use the pod name. |
| `cluster.peers` | `TIKEO__CLUSTER__PEERS` | Raft: yes | `[]` | Static peer list; arrays are clearer in file/Helm values. |
| `cluster.transport_token` | `TIKEO__CLUSTER__TRANSPORT_TOKEN` | Raft: yes | unset | Shared token for internal Raft/relay traffic; store in a Secret. |
| `cluster.scheduler_shard_map_version` | `TIKEO__CLUSTER__SCHEDULER_SHARD_MAP_VERSION` | No | `1` | Monotonic scheduler shard-map version. |
| `cluster.scheduler_shard_count` | `TIKEO__CLUSTER__SCHEDULER_SHARD_COUNT` | No | `64` | Logical scheduler shard count; keep stable per map version. |
| `auth.local_login_enabled` | `TIKEO__AUTH__LOCAL_LOGIN_ENABLED` | No | `true` | Local username/password login toggle. |
| `auth.api_tokens.default_ttl_seconds` | `TIKEO__AUTH__API_TOKENS__DEFAULT_TTL_SECONDS` | No | `43200` | Default API token TTL. |
| `auth.api_tokens.min_ttl_seconds` | `TIKEO__AUTH__API_TOKENS__MIN_TTL_SECONDS` | No | `300` | Minimum requested token TTL. |
| `auth.api_tokens.max_ttl_seconds` | `TIKEO__AUTH__API_TOKENS__MAX_TTL_SECONDS` | No | `2592000` | Maximum requested token TTL. |
| `auth.oidc.enabled` | `TIKEO__AUTH__OIDC__ENABLED` | No | `false` | Enable OIDC login. |
| `auth.oidc.issuer_url` | `TIKEO__AUTH__OIDC__ISSUER_URL` | If OIDC enabled | unset | OIDC issuer URL. |
| `auth.oidc.client_id` | `TIKEO__AUTH__OIDC__CLIENT_ID` | If OIDC enabled | unset | OIDC client id. |
| `auth.oidc.client_secret` | `TIKEO__AUTH__OIDC__CLIENT_SECRET` | If OIDC enabled | unset | OIDC client secret. |
| `auth.oidc.scopes` | `TIKEO__AUTH__OIDC__SCOPES` | No | `openid`, `profile`, `email` | Prefer config file for list shape. |
| `transport_security.http.tls_enabled` | `TIKEO__TRANSPORT_SECURITY__HTTP__TLS_ENABLED` | No | `false` | Enable TLS on the HTTP listener itself. |
| `transport_security.http.mtls_required` | `TIKEO__TRANSPORT_SECURITY__HTTP__MTLS_REQUIRED` | No | `false` | Require HTTP client certs; also needs TLS and client CA. |
| `transport_security.http.cert_path` | `TIKEO__TRANSPORT_SECURITY__HTTP__CERT_PATH` | If TLS enabled | unset | HTTP listener certificate path. |
| `transport_security.http.key_path` | `TIKEO__TRANSPORT_SECURITY__HTTP__KEY_PATH` | If TLS enabled | unset | HTTP listener private key path. |
| `transport_security.http.client_ca_path` | `TIKEO__TRANSPORT_SECURITY__HTTP__CLIENT_CA_PATH` | If mTLS required | unset | HTTP client CA bundle. |
| `transport_security.worker_tunnel.tls_enabled` | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__TLS_ENABLED` | No | `false` | Enable TLS on the Worker Tunnel. |
| `transport_security.worker_tunnel.mtls_required` | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__MTLS_REQUIRED` | No | `false` | Require Worker client certs. |
| `transport_security.worker_tunnel.cert_path` | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__CERT_PATH` | If TLS enabled | unset | Worker Tunnel certificate path. |
| `transport_security.worker_tunnel.key_path` | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__KEY_PATH` | If TLS enabled | unset | Worker Tunnel private key path. |
| `transport_security.worker_tunnel.client_ca_path` | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__CLIENT_CA_PATH` | If mTLS required | unset | Worker client CA bundle. |
| `observability.logging.level` | `TIKEO__OBSERVABILITY__LOGGING__LEVEL` | No | `info` | Default log level when `RUST_LOG` is not set. |
| `observability.logging.log_dir` | `TIKEO__OBSERVABILITY__LOGGING__LOG_DIR` | No | unset; production template `/logs` | Writes `tikeo.log` in addition to stdout. |
| `observability.tracing.enabled` | `TIKEO__OBSERVABILITY__TRACING__ENABLED` | No | `false` | Enable OTLP trace export. |
| `observability.tracing.otlp_endpoint` | `TIKEO__OBSERVABILITY__TRACING__OTLP_ENDPOINT` | If tracing enabled | unset | OTLP collector endpoint. |
| `observability.tracing.headers` | `TIKEO__OBSERVABILITY__TRACING__HEADERS` | No | `[]` | Exporter auth/tenant header names; values live outside status APIs. |
| `alert_retry.enabled` | `TIKEO__ALERT_RETRY__ENABLED` | No | `true` | Alert retry worker switch. |
| `alert_retry.interval_seconds` | `TIKEO__ALERT_RETRY__INTERVAL_SECONDS` | No | `60` | Due-attempt scan interval. |
| `alert_retry.batch_size` | `TIKEO__ALERT_RETRY__BATCH_SIZE` | No | `50` | Max due attempts scanned per iteration. |
| `alert_retry.max_attempts` | `TIKEO__ALERT_RETRY__MAX_ATTEMPTS` | No | `3` | Attempts before dead-lettering. |
| `alert_retry.backoff_seconds` | `TIKEO__ALERT_RETRY__BACKOFF_SECONDS` | No | `300` | Retry backoff. |
| `notification_delivery.enabled` | `TIKEO__NOTIFICATION_DELIVERY__ENABLED` | No | `true` | Notification Center delivery worker switch. |
| `notification_delivery.public_console_base_url` | `TIKEO__NOTIFICATION_DELIVERY__PUBLIC_CONSOLE_BASE_URL` | No | unset; template `http://127.0.0.1:8080` | External Web base URL for notification card links. |
| `notification_delivery.interval_seconds` | `TIKEO__NOTIFICATION_DELIVERY__INTERVAL_SECONDS` | No | `60` | Due-attempt scan interval. |
| `notification_delivery.batch_size` | `TIKEO__NOTIFICATION_DELIVERY__BATCH_SIZE` | No | `50` | Max due attempts scanned per iteration. |
| `notification_delivery.max_attempts` | `TIKEO__NOTIFICATION_DELIVERY__MAX_ATTEMPTS` | No | `3` | Attempts before dead-lettering. |
| `notification_delivery.backoff_seconds` | `TIKEO__NOTIFICATION_DELIVERY__BACKOFF_SECONDS` | No | `300` | Retry backoff. |
| `alert_secrets.allow_env_refs` | `TIKEO__ALERT_SECRETS__ALLOW_ENV_REFS` | No | `true` | Allows `env:NAME` references in alert/channel secrets. |
| `alert_secrets.env_prefix` | `TIKEO__ALERT_SECRETS__ENV_PREFIX` | No | `TIKEO_ALERT_SECRET_` | Expected env secret prefix. |
| `script_governance.release_signature_secret_ref` | `TIKEO__SCRIPT_GOVERNANCE__RELEASE_SIGNATURE_SECRET_REF` | Only when signature gate is enabled | unset | `env:NAME` secret ref for script release signature verification. |

## Notification Center delivery

`notification_delivery.*` controls the generic Notification Center delivery worker. Set `notification_delivery.public_console_base_url` to the externally reachable Web URL when provider cards should link back to the console. Provider credentials live on each channel row, while this Server config only controls the background delivery loop and public link base.

## Worker configuration table

| Config key / SDK field | Environment variable | Required? | Default | Meaning |
| --- | --- | --- | --- | --- |
| `tikeo.worker.enabled` | `TIKEO_WORKER_ENABLED` | No | `true` | Spring Boot auto-configuration switch. |
| `tikeo.worker.auto-startup` | `TIKEO_WORKER_AUTO_STARTUP` | No | `true` | Spring Boot lifecycle auto-start switch. |
| `endpoint` / `tikeo.worker.endpoint` | `TIKEO_WORKER_ENDPOINT` | Yes for live workers | demos use `http://127.0.0.1:9998` | Worker Tunnel endpoint reachable from the worker process. |
| `dry-run` | `TIKEO_WORKER_DRY_RUN` | No | `false` | Avoids opening a live Worker Tunnel. |
| `heartbeatEvery` / `heartbeat-interval-millis` | `TIKEO_WORKER_HEARTBEAT_INTERVAL_MILLIS` | No | `10000` ms / `10s` | Worker lease renewal cadence. |
| `clientInstanceId` / `client-instance-id` | `TIKEO_WORKER_CLIENT_INSTANCE_ID` | Core SDKs: yes; Boot: no | Boot generates/persists when blank | Stable client-side hint; Server assigns authoritative `worker_id`. |
| `state-dir` | `TIKEO_WORKER_STATE_DIR` | No | `~/.tikeo/workers` in Boot helper | Client instance id and sandbox tool cache directory. |
| `namespace` | `TIKEO_WORKER_NAMESPACE` | No | `default` | Tenant/environment namespace. |
| `app` | `TIKEO_WORKER_APP` | No | `default` | Application scope. |
| `cluster` | `TIKEO_WORKER_CLUSTER` | No | Java Boot `default`; other helpers `local` | Worker cluster/environment shard. |
| `region` | `TIKEO_WORKER_REGION` | No | Java Boot `default`; other helpers `local` | Worker region/zone. |
| `name` | `TIKEO_WORKER_NAME` | No | usually client instance id | Operator-facing worker name. |
| `version` | `TIKEO_WORKER_VERSION` | No | `dev` in Go/Python/Node helpers | Worker/application build version. |
| `capabilities` | `TIKEO_WORKER_CAPABILITIES` | No | `[]` | Legacy/operator metadata. |
| `labels` | `TIKEO_WORKER_LABELS` | No | `{}` | Comma-separated `key=value` in demos; maps in Boot. |
| `structured.sdkProcessors` | `TIKEO_WORKER_SDK_PROCESSORS` | No | demo-dependent | SDK processor names advertised for dispatch. |
| `structured.scriptRunners` | `TIKEO_WORKER_SCRIPT_LANGUAGES` / SDK API | No | demo-dependent | Script languages and sandbox backends. |
| `election.enabled` | `TIKEO_WORKER_ELECTION_ENABLED` | No | `true` | Worker-cluster master election flag. |
| `election.domain` | `TIKEO_WORKER_ELECTION_DOMAIN` | No | blank | Blank means `namespace/app/cluster/region`. |
| `election.priority` | `TIKEO_WORKER_ELECTION_PRIORITY` | No | `100` | Lower values win. |
| `wasm.auto-install` | `TIKEO_WORKER_WASM_AUTO_INSTALL` | No | `true` | Auto-install Wasmtime when unavailable. |
| `wasm.install-version` | `TIKEO_WORKER_WASM_INSTALL_VERSION` | No | `latest` | Wasmtime installer version. |
| `wasm.install-dir` | `TIKEO_WORKER_WASM_INSTALL_DIR` | No | `~/.tikeo/sandbox-tools/wasmtime` | Optional install directory. |
| `wasm.installer-url` | `TIKEO_WORKER_WASM_INSTALLER_URL` | No | `https://wasmtime.dev/install.sh` | Wasmtime installer URL. |
| `wasm.install-timeout-millis` | `TIKEO_WORKER_WASM_INSTALL_TIMEOUT_MILLIS` | No | `120000` | Installer timeout. |
| `scripts.enabled` | `TIKEO_WORKER_SCRIPTS_ENABLED` | No | `true` | Enable dynamic script execution. |
| `scripts.container-enabled` | `TIKEO_WORKER_SCRIPTS_CONTAINER_ENABLED` | No | `false` | Enable container-backed script runners. |
| `scripts.availability-check` | `TIKEO_WORKER_SCRIPTS_AVAILABILITY_CHECK` | No | `true` | Probe runtime before advertising non-WASM script capabilities. |
| `scripts.runtime-command` | `TIKEO_WORKER_SCRIPTS_RUNTIME_COMMAND` | No | blank | Explicit Docker-compatible runtime command. |
| `scripts.runtime-args` | `TIKEO_WORKER_SCRIPTS_RUNTIME_ARGS` | No | `[]` | Extra runtime args before image. |
| `scripts.auto-install-tools` | `TIKEO_WORKER_SCRIPTS_AUTO_INSTALL_TOOLS` | No | `true` | Auto-install local development script tools. |
| `scripts.*-install-version` | `TIKEO_WORKER_SCRIPT_*_INSTALL_VERSION` | No | `latest` / blank by tool | SRT, ripgrep, Deno, Rhai, PowerShell, WasmEdge, V8 versions. |
| `scripts.*-install-dir` | `TIKEO_WORKER_SCRIPT_*_INSTALL_DIR` | No | `~/.tikeo/sandbox-tools/<tool>` | Tool install/cache directories. |
| `scripts.*-installer-url` | `TIKEO_WORKER_SCRIPT_*_INSTALLER_URL` | No | tool default | Deno/WasmEdge and similar installer URLs. |
| `scripts.tool-install-timeout-millis` | `TIKEO_WORKER_SCRIPT_TOOL_INSTALL_TIMEOUT_MILLIS` | No | `120000` | Script tool installer timeout. |
| `scripts.images.*` | `TIKEO_WORKER_SCRIPT_IMAGE_*` | No | blank | Optional per-language container images; blank disables that runner. |

## Example run

```bash
cp config/tikeo.yml ./tikeo.yml
./target/release/tikeo serve --config ./tikeo.yml
```

For Docker Compose, edit `config/tikeo.yml`, not Compose `environment`, for Tikeo service behavior.
## Prerequisites

- Decide whether this process is a Server or Worker; the tables below are separate on purpose.
- For Server deployments, choose SQLite, PostgreSQL, MySQL, or CockroachDB and prepare the matching `storage.database.*` fields.
- For Worker deployments, keep SDK settings in the application configuration rather than Server `config/tikeo.yml`.

## Verify

After editing configuration, start the process and check `/readyz` for Server or Worker registration for SDK clients. Confirm that the effective storage, TLS, log, and notification settings match the expected environment.

## Troubleshooting

If configuration does not apply, check load order first: defaults, config file, then `TIKEO__...` environment overrides. For arrays and maps such as `cluster.peers` and `storage.database.params`, prefer file configuration to avoid shell escaping mistakes.

## Production checklist

- [ ] Sensitive values are in platform Secrets or secret references, not copied into public examples.
- [ ] Structured database fields are used instead of hand-built credential URLs.
- [ ] TLS/mTLS paths refer to mounted files under `/config/tls` when enabled.
- [ ] Worker SDK defaults are reviewed for endpoint, namespace, app, state-dir, and advertised capabilities.
