---
title: Logging configuration
description: Complete Tikeo Server logging configuration reference, including root filters, HTTP detail logs, SQL logs, console/file/ELK channels, defaults, performance impact, and security cautions.
---

# Logging configuration

Tikeo Server logging is configured under `observability.logging` in `config/tikeo.yml` or `config/dev.yml`. The same keys can be overridden with `TIKEO__...` environment variables, but the mounted YAML file should remain the normal source of truth for production.

The logging model has four layers:

1. `root` defines the default application log filter when `RUST_LOG` is not set.
2. `http` controls HTTP access summaries and optional request/response detail capture.
3. `sql` controls SQL driver/ORM logging.
4. `channels` controls output sinks. Disabled channels are not constructed, so Tikeo does not allocate their writer, formatter, or remote forwarder.

## Complete example

```yaml
observability:
  logging:
    root:
      level: INFO

    http:
      level: INFO
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
        enabled: false
        level: INFO
        path: "${TIKEO_LOG_PATH:/logs}"

      elk:
        enabled: ${ELK_ENABLED:false}
        servers: "${ELK_SERVERS:elk-server:8094}"
        topic: "${ELK_TOPIC:ivs-dev}"
        level: INFO
        sasl:
          enabled: ${ELK_SASL_ENABLED:false}
          username: "${ELK_USERNAME:}"
          password: "${ELK_PASSWORD:}"
```

`config/dev.yml` uses the same shape but sets the console channel to `DEBUG` and the file path default to `.dev/logs` for local development.

## Level and routing model

Tikeo uses `tracing` targets internally. The effective filter is the combination of:

| Layer | What it controls | Notes |
| --- | --- | --- |
| `root.level` | Default enabled level for Tikeo application targets and selected runtime targets. | Used only when `RUST_LOG` is absent. Invalid values fall back to `INFO`. |
| `http.level` | Detail log target `tikeo_server::http::detail`. | Does not change HTTP summary severity. It only controls the optional detailed request/response records. |
| `sql.enabled` and `sql.level` | `sqlx` and, when value logging is enabled, `sea_orm` SQL targets. | When `sql.enabled=false`, SQL targets are forced off even if the root level is broad. |
| `channels.*.level` | Minimum level accepted by each output sink. | A sink can be stricter than the root/detail filter, but it cannot output events filtered out upstream. |
| `RUST_LOG` | Emergency process-level override. | Prefer YAML config. If set, Tikeo still appends the configured SQL directives so disabled SQL logging remains off. |

Accepted level values are `TRACE`, `DEBUG`, `INFO`, `WARN`/`WARNING`, and `ERROR`.

## `root`

| Key | Default | Environment variable | Effect |
| --- | --- | --- | --- |
| `observability.logging.root.level` | `INFO` | `TIKEO__OBSERVABILITY__LOGGING__ROOT__LEVEL` | Default application log level for Server, storage, config, selected HTTP/runtime libraries, and ordinary application events. |

Operational notes:

- Keep production at `INFO` unless an incident needs more detail.
- `DEBUG` and `TRACE` increase event volume and string formatting cost.
- If `RUST_LOG` is set, it becomes the primary filter. Use it only for emergency overrides or one-off local debugging.

## HTTP logging

HTTP logging is split into **summary logs** and **detail logs**.

Summary logs are always emitted by outcome:

| Outcome | Level | Message |
| --- | --- | --- |
| request received | `INFO` | `HTTP request received` |
| success / non-error response | `INFO` | `HTTP request completed` |
| 4xx response | `WARN` | `HTTP request completed with client error` |
| 5xx response | `ERROR` | `HTTP request completed with server error` |

Summary records include `trace_id`, method, path, query, status, latency, and request/response size when available. The response log includes end-to-end interface latency.

Detail logs are emitted only when `include_headers` or `include_body` is enabled and the `tikeo_server::http::detail` target is enabled at `http.level`.

| Key | Default | Environment variable | Effect | Caution |
| --- | --- | --- | --- | --- |
| `observability.logging.http.level` | `INFO` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__LEVEL` | Level used for `HTTP request detail` and `HTTP response detail` events. | Does not suppress summary logs. |
| `observability.logging.http.include_headers` | `false` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__INCLUDE_HEADERS` | Captures request and response headers in detail logs. | `authorization`, `cookie`, `set-cookie`, `x-api-key`, and similar headers are redacted, but custom sensitive headers may still need upstream redaction. |
| `observability.logging.http.include_body` | `false` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__INCLUDE_BODY` | Captures request and response bodies in detail logs. | Can expose credentials, tokens, payload data, scripts, and business content. Enable only for short debugging windows. |
| `observability.logging.http.max_body_bytes` | `65536` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__MAX_BODY_BYTES` | Maximum bytes read from each request or response body for detail logging. | Higher values increase memory copying and latency for body-logged requests. |

Body capture is intentionally skipped for streaming or binary-like traffic:

- `text/event-stream` SSE responses
- `application/grpc`
- `multipart/*`
- `application/octet-stream`
- chunked bodies without a content length
- bodies that fail to read within the configured capture limit

Recommended use:

```yaml
# Short local debugging window only.
observability:
  logging:
    http:
      level: DEBUG
      include_headers: true
      include_body: true
      max_body_bytes: 16384
```

Return `include_body` to `false` after the incident. For production, keep headers and bodies disabled unless a tightly controlled debug window requires them.

## SQL logging

SQL logs are disabled by default because high-volume database logging can be noisy, expensive, and sensitive.

| Key | Default | Environment variable | Effect | Caution |
| --- | --- | --- | --- | --- |
| `observability.logging.sql.enabled` | `false` | `TIKEO__OBSERVABILITY__LOGGING__SQL__ENABLED` | Enables SQL driver/ORM execution logs. | Leave disabled during normal production operation. |
| `observability.logging.sql.level` | `DEBUG` | `TIKEO__OBSERVABILITY__LOGGING__SQL__LEVEL` | Log level for enabled SQL events. | Use `DEBUG` for incident diagnosis; `TRACE` can be very noisy. |
| `observability.logging.sql.include_values` | `false` | `TIKEO__OBSERVABILITY__LOGGING__SQL__INCLUDE_VALUES` | Enables bound-value logging where the driver/ORM supports it. | May expose credentials, tokens, tenant/scope names, payloads, and business data. Use only for the shortest possible window. |
| `observability.logging.sql.slow_threshold_ms` | `250` | `TIKEO__OBSERVABILITY__LOGGING__SQL__SLOW_THRESHOLD_MS` | Slow statement threshold used by the storage driver options. | Lower thresholds increase warning volume. |

Recommended production diagnosis profile:

```yaml
observability:
  logging:
    sql:
      enabled: true
      level: DEBUG
      include_values: false
      slow_threshold_ms: 250
```

Enable `include_values=true` only when you have an approved, time-boxed need and a safe destination for the resulting logs.

## Output channels

### Console channel

| Key | Default | Environment variable | Effect |
| --- | --- | --- | --- |
| `observability.logging.channels.console.enabled` | `true` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__CONSOLE__ENABLED` | Enables stdout/stderr console output. |
| `observability.logging.channels.console.level` | `INFO` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__CONSOLE__LEVEL` | Minimum level written to console. |

Console output is the recommended container and Kubernetes path. It uses ANSI color when enabled and labels each event category so operators can visually distinguish `[HTTP]`, `[SQL ]`, and `[APP ]` logs.

Impact:

- Best default for containers because the platform log collector owns persistence and rotation.
- `DEBUG` or `TRACE` can still produce significant stdout volume.
- If disabled, no console formatter/writer layer is installed.

### File channel

| Key | Default | Environment variable | Effect |
| --- | --- | --- | --- |
| `observability.logging.channels.file.enabled` | `false` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__ENABLED` | Enables the JSON file sink. |
| `observability.logging.channels.file.level` | `INFO` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__LEVEL` | Minimum level written to the file sink. |
| `observability.logging.channels.file.path` | `/logs` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__PATH` or template variable `TIKEO_LOG_PATH` | Directory or file path for file logs. Directory paths write `tikeo.log`; file paths use the provided file name. |

File logs are newline-delimited JSON records with `timestamp`, `level`, `target`, `message`, and optional `fields`.

Impact and operations:

- The sink uses a non-blocking writer so request paths enqueue formatted events instead of performing direct blocking file I/O.
- Enable it only when a durable log volume is mounted. In containers, mount `/logs` or set `TIKEO_LOG_PATH` to the mounted directory.
- Rotation/retention is an operator responsibility. Use the platform log agent, logrotate, or a sidecar policy.
- If disabled, Tikeo does not create the directory and does not install the file writer.

### ELK / remote collector channel

| Key | Default | Environment variable | Effect |
| --- | --- | --- | --- |
| `observability.logging.channels.elk.enabled` | `false` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__ENABLED` or template variable `ELK_ENABLED` | Enables remote JSON-lines forwarding. |
| `observability.logging.channels.elk.servers` | `elk-server:8094` in `config/tikeo.yml` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SERVERS` or `ELK_SERVERS` | Comma-separated `host:port` collector list. |
| `observability.logging.channels.elk.topic` | `ivs-dev` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__TOPIC` or `ELK_TOPIC` | Logical collector topic/index metadata. |
| `observability.logging.channels.elk.level` | `INFO` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__LEVEL` | Minimum level forwarded remotely. |
| `observability.logging.channels.elk.sasl.enabled` | `false` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SASL__ENABLED` or `ELK_SASL_ENABLED` | SASL metadata switch for compatible collector environments. |
| `observability.logging.channels.elk.sasl.username` | empty | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SASL__USERNAME` or `ELK_USERNAME` | Optional SASL username metadata. |
| `observability.logging.channels.elk.sasl.password` | empty | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SASL__PASSWORD` or `ELK_PASSWORD` | Optional SASL password metadata. Keep in Secret-managed env, not in Git. |

ELK output uses flat JSON lines shaped for log collectors:

```json
{
  "app": "tikeo-server",
  "ip": null,
  "hostname": "tikeo-0",
  "class": "tikeo_server::http::trace",
  "file": "crates/tikeo-server/src/http/trace.rs",
  "method": "tikeo_server::http::trace",
  "line": "150",
  "datetime": "2026-06-26T00:00:00Z",
  "thread": "tokio-runtime-worker",
  "level": "INFO",
  "trace_id": "trc-...",
  "msg": "HTTP request completed | method=GET | path=/readyz | status=200 | latency_ms=1.2",
  "exception": ""
}
```

Impact and operations:

- Forwarding is non-blocking and batched. Application threads enqueue log frames; a dedicated `tikeo-elk-log-forwarder` thread flushes to collectors.
- If the bounded queue is full or disconnected, log frames can be dropped instead of blocking business traffic.
- Configure multiple collectors in `servers` for failover.
- `sasl.*` values are configuration metadata for compatible collector environments; they are not a replacement for network-level TLS, firewalling, or collector-side access control.
- If disabled, no remote forwarder thread or remote formatter is installed.

## Environment override quick reference

| YAML key | Primary environment override | Template shortcut |
| --- | --- | --- |
| `observability.logging.root.level` | `TIKEO__OBSERVABILITY__LOGGING__ROOT__LEVEL` | — |
| `observability.logging.http.level` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__LEVEL` | — |
| `observability.logging.http.include_headers` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__INCLUDE_HEADERS` | — |
| `observability.logging.http.include_body` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__INCLUDE_BODY` | — |
| `observability.logging.http.max_body_bytes` | `TIKEO__OBSERVABILITY__LOGGING__HTTP__MAX_BODY_BYTES` | — |
| `observability.logging.sql.enabled` | `TIKEO__OBSERVABILITY__LOGGING__SQL__ENABLED` | — |
| `observability.logging.sql.level` | `TIKEO__OBSERVABILITY__LOGGING__SQL__LEVEL` | — |
| `observability.logging.sql.include_values` | `TIKEO__OBSERVABILITY__LOGGING__SQL__INCLUDE_VALUES` | — |
| `observability.logging.sql.slow_threshold_ms` | `TIKEO__OBSERVABILITY__LOGGING__SQL__SLOW_THRESHOLD_MS` | — |
| `observability.logging.channels.console.enabled` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__CONSOLE__ENABLED` | — |
| `observability.logging.channels.console.level` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__CONSOLE__LEVEL` | — |
| `observability.logging.channels.file.enabled` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__ENABLED` | — |
| `observability.logging.channels.file.level` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__LEVEL` | — |
| `observability.logging.channels.file.path` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__FILE__PATH` | `TIKEO_LOG_PATH` in committed templates |
| `observability.logging.channels.elk.enabled` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__ENABLED` | `ELK_ENABLED` in committed templates |
| `observability.logging.channels.elk.servers` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SERVERS` | `ELK_SERVERS` in committed templates |
| `observability.logging.channels.elk.topic` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__TOPIC` | `ELK_TOPIC` in committed templates |
| `observability.logging.channels.elk.level` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__LEVEL` | — |
| `observability.logging.channels.elk.sasl.enabled` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SASL__ENABLED` | `ELK_SASL_ENABLED` in committed templates |
| `observability.logging.channels.elk.sasl.username` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SASL__USERNAME` | `ELK_USERNAME` in committed templates |
| `observability.logging.channels.elk.sasl.password` | `TIKEO__OBSERVABILITY__LOGGING__CHANNELS__ELK__SASL__PASSWORD` | `ELK_PASSWORD` in committed templates |

## Recommended profiles

### Local development

```yaml
observability:
  logging:
    root:
      level: DEBUG
    channels:
      console:
        enabled: true
        level: DEBUG
      file:
        enabled: false
      elk:
        enabled: false
```

Use this when iterating locally. Add `http.include_body=true` or `sql.enabled=true` only for the specific debugging window.

### Production container default

```yaml
observability:
  logging:
    root:
      level: INFO
    http:
      level: INFO
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
        enabled: false
      elk:
        enabled: false
```

Use console output and let Kubernetes, Docker, or the host collector own persistence.

### Production with file logs

```yaml
observability:
  logging:
    channels:
      console:
        enabled: true
        level: INFO
      file:
        enabled: true
        level: INFO
        path: /logs
```

Mount `/logs` as a durable writable volume and configure external rotation.

### Production with remote collector

```yaml
observability:
  logging:
    channels:
      console:
        enabled: true
        level: INFO
      elk:
        enabled: true
        servers: "elk-a:8094,elk-b:8094"
        topic: "tikeo-prod"
        level: INFO
        sasl:
          enabled: false
```

Keep HTTP body logging and SQL value logging disabled unless an incident explicitly requires them.

## Operational cautions

- Do not run broad `TRACE` logging in production unless the incident requires it and the window is short.
- Do not enable full HTTP body logging or SQL value logging for normal operation. Both can expose sensitive business data.
- Use durable mounts before enabling file logs; otherwise logs may disappear on container restart.
- Prefer console logging in container platforms and collect stdout with the platform log agent.
- Disabled output channels are intentionally not loaded. This is the lowest-overhead mode for unused sinks.
- Use `RUST_LOG` only as an emergency override. Persist intended behavior in `observability.logging` instead.
