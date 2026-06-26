---
title: 配置 Cookbook
description: Tikeo 本地、Docker、PostgreSQL、MySQL、TLS、OIDC、观测、通知和排障配置示例。
---

# 配置 Cookbook

可部署的 Server 配置写在 `config/tikeo.yml`。Compose `.env` 只放 Docker 参数。

## 本地源码运行

```bash
cargo run --bin tikeo -- serve --config config/dev.yml
```

## Docker/Compose 默认模式

```bash
cp deploy/compose/tikeo.env.example .env
# Tikeo 服务配置改 config/tikeo.yml。
docker compose --env-file .env up -d
```

## PostgreSQL（密码含特殊字符）

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

## TLS/mTLS 路径

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

## 通知公开链接

```yaml
notification_delivery:
  enabled: true
  public_console_base_url: https://tikeo.example.com
  interval_seconds: 60
  batch_size: 50
  max_attempts: 3
  backoff_seconds: 300
```

## 观测

```yaml
observability:
  logging:
    root:
      level: INFO
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

OIDC 验证完成并确认有恢复入口前，保留本地登录。

## 排障

| 现象 | 检查 |
| --- | --- |
| 配置像是没生效 | 确认 `--config`、`TIKEO_CONFIG` 和文件路径。 |
| DB 连接失败 | 检查 `storage.database.type/host/port/username/password/database`；无需手动 URL encode。 |
| SQLite 数据丢失 | 持久化 `/data`，并确认 `storage.database.path=/data/tikeo.db`。 |
| TLS 启动失败 | 检查 `/config/tls` 挂载和证书/私钥/CA 路径。 |
| 通知卡片跳 localhost | 设置 `notification_delivery.public_console_base_url` 为公开 Web URL。 |
