---
title: Configuration cookbook
description: Copy-paste Tikeo configuration recipes for local, Docker, PostgreSQL, MySQL, TLS, OIDC, observability, notifications, and troubleshooting.
---

# Configuration cookbook

Use `config/tikeo.yml` for deployable Server settings. Compose `.env` is only for Docker parameters.

## Local source run

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
```

## Docker/Compose default

```bash
cp deploy/compose/tikeo.env.example .env
# Edit config/tikeo.yml for Tikeo service settings.
docker compose --env-file .env up -d
```

## PostgreSQL with special-character password

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

## MySQL

```yaml
storage:
  database:
    type: mysql
    host: mysql
    port: 3306
    username: tikeo
    password: "p@ss/word:with#chars"
    database: tikeo
```

## TLS/mTLS paths

```yaml
transport_security:
  http:
    tls_enabled: true
    mtls_required: false
    cert_path: /config/tls/http.crt
    key_path: /config/tls/http.key
  worker_tunnel:
    tls_enabled: true
    mtls_required: true
    cert_path: /config/tls/worker.crt
    key_path: /config/tls/worker.key
    client_ca_path: /config/tls/ca.crt
```

## Notification public links

```yaml
notification_delivery:
  enabled: true
  public_console_base_url: https://tikeo.example.com
  interval_seconds: 60
  batch_size: 50
  max_attempts: 3
  backoff_seconds: 300
```

## Observability

```yaml
observability:
  logging:
    level: info
    log_dir: /logs
  tracing:
    enabled: true
    otlp_endpoint: http://otel-collector:4318/v1/traces
    headers: []
```

## OIDC

```yaml
auth:
  local_login_enabled: true
  oidc:
    enabled: true
    issuer_url: https://issuer.example.com
    client_id: tikeo
    client_secret: change-me
    scopes: ["openid", "profile", "email"]
```

Keep local login enabled until OIDC has been verified and recovery access exists.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Config appears ignored | Confirm `--config`, `TIKEO_CONFIG`, and file path. |
| DB connection fails | Check `storage.database.type/host/port/username/password/database`; no manual URL escaping is needed. |
| SQLite state disappears | Persist `/data` and ensure `storage.database.path=/data/tikeo.db`. |
| TLS startup fails | Verify `/config/tls` mount and referenced cert/key/CA paths. |
| Notification card links point to localhost | Set `notification_delivery.public_console_base_url` to the public Web URL. |
