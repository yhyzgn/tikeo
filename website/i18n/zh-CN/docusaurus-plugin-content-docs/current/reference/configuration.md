---
title: 配置参考
description: Tikeo 配置文件、环境变量覆盖、端口、存储 URL、TLS/mTLS、观测性、告警重试与脚本治理参数。
---

# 配置参考

Tikeo 通过 `tikeo serve --config <path>` 读取 TOML 配置。部署层可以用环境变量覆盖嵌套配置，例如 `TIKEO__STORAGE__DATABASE_URL`。非敏感默认值可以提交到 TOML；生产凭据必须放到 Secret store。

## 本地复制即跑

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
```

## 已提交配置文件

| 文件 | 用途 | 存储 |
|---|---|---|
| `config/dev.toml` | 本地源码评估 | SQLite `tikeo-dev.db` |
| `config/container.toml` | 容器默认 | SQLite `/data/tikeo.db` |
| `config/postgres.toml` | PostgreSQL/CockroachDB 示例 | `postgres://...` |
| `config/mysql.toml` | MySQL 示例 | `mysql://...` |
| `config/raft.toml` | 集群/raft 规划示例 | 见文件 |

## Server 端口

| 配置项 | 示例默认 | 含义 |
|---|---|---|
| `server.listen_addr` | `0.0.0.0:9090` | HTTP API、health、ready、metrics 与 gateway surface。 |
| `server.worker_tunnel_addr` | `0.0.0.0:9998` | Worker Tunnel gRPC/HTTP2 listener；Worker 主动出站连接。 |

Docker Compose 使用 `TIKEO_HTTP_PORT` / `TIKEO_WORKER_TUNNEL_PORT` 映射；Helm 使用 `server.httpPort` / `server.workerTunnelPort`。

## Storage URL

| 后端 | 示例 |
|---|---|
| SQLite dev | `sqlite://tikeo-dev.db?mode=rwc` |
| SQLite container | `sqlite:///data/tikeo.db?mode=rwc` |
| PostgreSQL | `postgres://tikeo:change-me@postgres.example:5432/tikeo?sslmode=require` |
| CockroachDB | `postgres://root@cockroach:26257/tikeo?sslmode=disable` |
| MySQL | `mysql://tikeo:change-me@mysql.example:3306/tikeo` |

环境变量覆盖：

```bash
TIKEO__STORAGE__DATABASE_URL='postgres://tikeo:change-me@postgres:5432/tikeo?sslmode=require' \
  ./target/release/tikeo serve --config config/container.toml
```

schema 变化必须通过显式 SeaORM migration，不要把手工改表写成支持路径。

## 认证与 API Token

```toml
[auth]
local_login_enabled = true

[auth.api_tokens]
default_ttl_seconds = 43200
min_ttl_seconds = 300
max_ttl_seconds = 2592000
```

开发环境可以使用本地登录；共享环境应配置 OIDC，并把 API-Key / Service Account 限制在 app scope 内。

## Transport security

```toml
[transport_security.http]
tls_enabled = false
mtls_required = false

[transport_security.worker_tunnel]
tls_enabled = false
mtls_required = false
```

直接暴露 API 时启用 HTTP TLS；Worker 跨主机、跨集群、跨 VPC 或跨信任边界时启用 Worker Tunnel TLS/mTLS。Helm 通过 Secret mount 生成对应配置。

## 观测性

```toml
[observability.logging]
level = "info"
# log_dir = "./logs"

[observability.tracing]
enabled = false
headers = []
# otlp_endpoint = "http://otel-collector:4318/v1/traces"
```

运维默认保持 `info`。VM/systemd 部署建议设置 `log_dir`。OTLP 只有在 collector 可达并被批准时开启。

## 告警重试与 Secret 引用

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

告警 channel JSON 可以通过 `env:NAME` 引用 SMTP、Webhook 或 API credential，不能把明文凭据提交到仓库。

## 脚本治理

```toml
[script_governance]
# release_signature_secret_ref = "env:TIKEO_SCRIPT_RELEASE_SECRET"
```

启用脚本发布签名时，把 secret 存在部署平台中，只把 reference 注入配置。


## SDK 与 Worker 配置

服务端配置只覆盖部署的一半。Worker 服务还需要 SDK 依赖选择、Worker Tunnel endpoint、身份 scope、sandbox 工具缓存路径，以及可选 management-client 凭证。Java/Spring 部署的 Maven Central artifact 矩阵、Spring Boot `application.yml` 模板、环境变量和默认值，见 [Java Spring Boot Starter](../sdks/java-spring-boot)。

生产 Worker 清单：

- 每个服务只添加一个 SDK 依赖，让包管理器解析传递的 Tikeo 模块。
- Worker SDK 应连接到能访问 `server.worker_tunnel_addr` 的 Service/LB/DNS 名称，而不一定是服务端 bind 地址。
- 在 worker 和 management client 中一致设置 namespace/app/cluster/region。
- 如果需要稳定身份或离线启动，持久化 SDK state/tool cache 目录，例如 `~/.tikeo/workers` 与 `~/.tikeo/sandbox-tools/*`。
- API key 与内部镜像 installer URL 应通过平台 Secret/config 注入，不要提交到配置文件。

## 环境变量覆盖规则

| 环境变量 | 配置项 |
|---|---|
| `TIKEO__STORAGE__DATABASE_URL` | `storage.database_url` |
| `TIKEO__ALERT_SECRETS__ALLOW_ENV_REFS` | `alert_secrets.allow_env_refs` |
| `TIKEO__ALERT_SECRETS__ENV_PREFIX` | `alert_secrets.env_prefix` |

非敏感默认值用 TOML；凭据、外部 endpoint、生产差异用环境变量或 Secret 注入。
