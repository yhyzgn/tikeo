---
title: 配置参考
description: Tikeo 的完整默认值、环境变量覆盖、示例 TOML、安全、观测、存储和 Worker SDK 默认值。
---

# 配置参考

Tikeo Server 配置由 `crates/tikeo-config/src/lib.rs` 中的类型加载。运行 `tikeo serve --config <path>` 或设置 `TIKEO_CONFIG` 指定 TOML 文件，然后用 `TIKEO` 前缀和双下划线覆盖嵌套字段，例如 `storage.database_url` 对应 `TIKEO__STORAGE__DATABASE_URL`。

## 加载顺序

加载顺序是 Rust 默认值、可选 TOML 文件、环境变量覆盖。生产建议把非敏感默认值写入 TOML，把 DB URL、证书路径、OIDC secret、OTel 凭证和集群 token 放到平台 Secret 或环境变量中。

```bash
TIKEO__SERVER__LISTEN_ADDR=0.0.0.0:19090 \
TIKEO__SERVER__WORKER_TUNNEL_ADDR=0.0.0.0:19998 \
TIKEO__STORAGE__DATABASE_URL='sqlite:///tmp/tikeo-smoke.db?mode=rwc' \
cargo run --bin tikeo -- serve --config config/dev.toml
```

## 配置文件用途

| 文件 | 用途 | 关键值 |
| --- | --- | --- |
| `config/dev.toml` | 本地源码评估 | HTTP `0.0.0.0:9090`、Worker Tunnel `0.0.0.0:9998`、SQLite `.dev/tikeo-dev.db`、`timestamp_offset="+08:00"`、OIDC/TLS/OTel 关闭。 |
| `config/container.toml` | root Dockerfile 默认 | SQLite `/data/tikeo.db`、日志 info、alert retry 开启、alert env refs 开启。 |
| `config/postgres.toml` | PostgreSQL/Cockroach 示例 | `postgres://tikeo:tikeo@postgres:5432/tikeo`，注释说明 `TIKEO__STORAGE__DATABASE_URL`。 |
| `config/mysql.toml` | MySQL 示例 | `mysql://tikeo:tikeo@mysql:3306/tikeo`，提醒使用 `utf8mb4`。 |
| `config/raft.toml` | raft shape 示例 | `mode="raft"`、静态 peers、`transport_token` 从 Secret 注入。 |

## 完整默认值表

| Config key | 默认值 | 环境变量 | 说明 |
| --- | --- | --- | --- |
| `server.listen_addr` | `0.0.0.0:9090` | `TIKEO__SERVER__LISTEN_ADDR` | HTTP API、health、ready、metrics、OpenAPI。 |
| `server.worker_tunnel_addr` | `0.0.0.0:9998` | `TIKEO__SERVER__WORKER_TUNNEL_ADDR` | gRPC/HTTP2 Worker Tunnel，Worker 主动连接。 |
| `storage.database_url` | `sqlite://.dev/tikeo-dev.db?mode=rwc` | `TIKEO__STORAGE__DATABASE_URL` | SeaORM/sqlx URL；生产用 PostgreSQL/MySQL。 |
| `storage.timestamp_offset` | `+00:00` | `TIKEO__STORAGE__TIMESTAMP_OFFSET` | 启动时解析；dev/mysql 示例为 `+08:00`。 |
| `cluster.mode` | `standalone` | `TIKEO__CLUSTER__MODE` | `standalone` 或 `raft`；K8s 生产多 Pod HA 使用 Helm Raft StatefulSet/headless peers，只有 Leader 调度。 |
| `cluster.node_id` | `standalone` | `TIKEO__CLUSTER__NODE_ID` | 集群状态和 raft 元数据节点 ID。 |
| `cluster.peers` | `[]` | `TIKEO__CLUSTER__PEERS` | peer 数组建议用 TOML/Helm 表达。 |
| `cluster.transport_token` | 未设置 | `TIKEO__CLUSTER__TRANSPORT_TOKEN` | 内部 raft HTTP token，不要提交真实值。 |
| `auth.local_login_enabled` | `true` | `TIKEO__AUTH__LOCAL_LOGIN_ENABLED` | 本地用户名密码登录开关。 |
| `auth.api_tokens.default_ttl_seconds` | `43200` | `TIKEO__AUTH__API_TOKENS__DEFAULT_TTL_SECONDS` | 默认 12 小时。 |
| `auth.api_tokens.min_ttl_seconds` | `300` | `TIKEO__AUTH__API_TOKENS__MIN_TTL_SECONDS` | 最小 5 分钟。 |
| `auth.api_tokens.max_ttl_seconds` | `2592000` | `TIKEO__AUTH__API_TOKENS__MAX_TTL_SECONDS` | 最大 30 天。 |
| `auth.oidc.enabled` | `false` | `TIKEO__AUTH__OIDC__ENABLED` | OIDC 默认关闭。 |
| `auth.oidc.issuer_url` | 未设置 | `TIKEO__AUTH__OIDC__ISSUER_URL` | OIDC issuer。 |
| `auth.oidc.client_id` | 未设置 | `TIKEO__AUTH__OIDC__CLIENT_ID` | OIDC client id。 |
| `auth.oidc.client_secret` | 未设置 | `TIKEO__AUTH__OIDC__CLIENT_SECRET` | Secret 存平台 Secret。 |
| `auth.oidc.scopes` | `openid, profile, email` | `TIKEO__AUTH__OIDC__SCOPES` | 列表形状不确定时用 TOML。 |
| `transport_security.http.tls_enabled` | `false` | `TIKEO__TRANSPORT_SECURITY__HTTP__TLS_ENABLED` | HTTP listener 进程内 TLS。 |
| `transport_security.http.mtls_required` | `false` | `TIKEO__TRANSPORT_SECURITY__HTTP__MTLS_REQUIRED` | HTTP mTLS。 |
| `transport_security.http.cert_path` | 未设置 | `TIKEO__TRANSPORT_SECURITY__HTTP__CERT_PATH` | 证书路径。 |
| `transport_security.http.key_path` | 未设置 | `TIKEO__TRANSPORT_SECURITY__HTTP__KEY_PATH` | 私钥路径。 |
| `transport_security.http.client_ca_path` | 未设置 | `TIKEO__TRANSPORT_SECURITY__HTTP__CLIENT_CA_PATH` | mTLS 客户端 CA。 |
| `transport_security.worker_tunnel.tls_enabled` | `false` | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__TLS_ENABLED` | Worker Tunnel TLS。 |
| `transport_security.worker_tunnel.mtls_required` | `false` | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__MTLS_REQUIRED` | Worker 客户端证书。 |
| `transport_security.worker_tunnel.cert_path` | 未设置 | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__CERT_PATH` | Tunnel 证书。 |
| `transport_security.worker_tunnel.key_path` | 未设置 | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__KEY_PATH` | Tunnel 私钥。 |
| `transport_security.worker_tunnel.client_ca_path` | 未设置 | `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__CLIENT_CA_PATH` | Worker 客户端 CA。 |
| `observability.logging.level` | `info` | `TIKEO__OBSERVABILITY__LOGGING__LEVEL` | `RUST_LOG` 未设置时使用。 |
| `observability.logging.log_dir` | 未设置 | `TIKEO__OBSERVABILITY__LOGGING__LOG_DIR` | 设置后写 `tikeo.log`。 |
| `observability.tracing.enabled` | `false` | `TIKEO__OBSERVABILITY__TRACING__ENABLED` | OTel trace export 开关。 |
| `observability.tracing.otlp_endpoint` | 未设置 | `TIKEO__OBSERVABILITY__TRACING__OTLP_ENDPOINT` | 开启 tracing 时必须设置。 |
| `observability.tracing.headers` | `[]` | `TIKEO__OBSERVABILITY__TRACING__HEADERS` | header 名称，值不进 status API。 |
| `alert_retry.enabled` | `true` | `TIKEO__ALERT_RETRY__ENABLED` | 告警投递重试 worker。 |
| `alert_retry.interval_seconds` | `60` | `TIKEO__ALERT_RETRY__INTERVAL_SECONDS` | 扫描间隔。 |
| `alert_retry.batch_size` | `50` | `TIKEO__ALERT_RETRY__BATCH_SIZE` | 每轮最大数量。 |
| `alert_retry.max_attempts` | `3` | `TIKEO__ALERT_RETRY__MAX_ATTEMPTS` | 死信前最大次数。 |
| `alert_retry.backoff_seconds` | `300` | `TIKEO__ALERT_RETRY__BACKOFF_SECONDS` | 重试退避。 |
| `alert_secrets.allow_env_refs` | `true` | `TIKEO__ALERT_SECRETS__ALLOW_ENV_REFS` | 允许 `env:NAME` 引用。 |
| `alert_secrets.env_prefix` | `TIKEO_ALERT_SECRET_` | `TIKEO__ALERT_SECRETS__ENV_PREFIX` | 告警 Secret 前缀。 |
| `script_governance.release_signature_secret_ref` | 未设置 | `TIKEO__SCRIPT_GOVERNANCE__RELEASE_SIGNATURE_SECRET_REF` | 脚本发布签名 Secret 引用。 |

## 存储、认证和安全

SQLite 适合本地，容器里要持久化 `/data`。生产建议使用 PostgreSQL 或 MySQL，并通过 `TIKEO__STORAGE__DATABASE_URL` 从 Secret 注入。SQLite 启动会设置 WAL、busy timeout、foreign keys，但 schema 变更仍必须走显式 SeaORM migration。

本地登录默认开启。首个部署 Owner 通过 `/api/v1/auth/bootstrap/register` 创建。OIDC 默认关闭；开启后需要 issuer、client_id、client_secret，并且外部身份需要先映射到本地用户。SDK API Key 与人类 bearer token 不同，SDK 使用 `x-tikeo-api-key`，通常来自 `TIKEO_MANAGEMENT_API_KEY`。

TLS/mTLS 默认关闭。跨主机、跨集群、跨 VPC 或不可信网络时，应给 HTTP 和 Worker Tunnel 分别配置 TLS。mTLS 需要 `tls_enabled=true`、`cert_path`、`key_path` 和 `client_ca_path`。Ingress TLS 与 Tikeo 进程内 TLS 是两层不同配置。

## 观测和集群

日志默认 console + `info`。设置 `observability.logging.log_dir` 后会写 `tikeo.log`。开启 OTel 时必须设置 `observability.tracing.otlp_endpoint`，并把凭证放环境变量或 Secret。

`standalone` 是单 Server 安装的默认模式。`raft` 是生产多 Pod Server HA 模式，但必须配合稳定 node id、静态 peers、外部数据库和内部 transport token 部署。调度模型是 active-passive：只有已选出的 Raft Leader 在持久化 fencing token 后报告 `canSchedule=true` 并运行 schedule/dispatch/retry 所有权循环；Follower 会跳过这些循环。配置了 `cluster.transport_token` 时，内部 raft append 流量需要 `x-tikeo-raft-token`。核心调度所有权不要引入 Redis/Dragonfly 分布式锁；未来多活调度应走 Raft/fencing shard ownership。

## Worker SDK 默认值

| 字段 | 默认值 | 说明 |
| --- | --- | --- |
| `endpoint` | 调用方提供，demo 多用 `http://127.0.0.1:9998` | 可达的 Worker Tunnel endpoint。 |
| `clientInstanceId` / `client_instance_id` | 必填或由 Java Boot 生成 | 稳定客户端提示；Server 分配权威 `worker_id`。 |
| `namespace` | SDK helper 通常是 `default` | 调度和管理 scope。 |
| `app` | SDK helper 通常是 `default` | 应用 scope。 |
| `cluster` | Rust/Go/Python/Node 多为 `local`，Java Boot 默认为 `default` | Worker 集群元数据。 |
| `region` | Rust/Go/Python/Node 多为 `local`，Java Boot 默认为 `default` | 区域元数据。 |
| `heartbeatEvery` | 10 秒或 10000 ms | lease 续约频率。 |
| `capabilities` | 空 | legacy metadata，路由应使用 structured。 |
| `structuredCapabilities` | 空 | SDK processors、script runners、plugin processors、tags。 |
| `labels` | 空 | 如 `worker_pool`。 |
| `election.enabled` | true | Worker 集群选主元数据。 |
| `election.priority` | 100 | 数值越小优先级越高。 |

只广告真实可执行能力。缺少 SRT、Deno、Docker、Podman、SQL 或 plugin 工具时要 fail-closed，不要为了展示而广告能力。

## 部署检查清单

选择存储并确认备份；用 Secret 注入 DB URL；决定 API 和 Worker Tunnel 的 TLS/mTLS；业务 Worker 不放进 Helm chart，而是独立部署并出站连接；初始化 Owner 后创建 service account 和 SDK API Key；确认日志/OTel 目标存在；运行对应 smoke 并保存 `.dev/reports` 或 CI artifact。

## 环境变量覆盖 runbook

现场改配置时优先遵循“可公开默认值进 TOML、敏感值进 Secret、临时排障值进环境变量”的顺序。示例：数据库 URL 用 `TIKEO__STORAGE__DATABASE_URL` 覆盖；OIDC issuer 用 `TIKEO__AUTH__OIDC__ISSUER_URL`，client secret 用 `TIKEO__AUTH__OIDC__CLIENT_SECRET`；Worker Tunnel 客户端 CA 用 `TIKEO__TRANSPORT_SECURITY__WORKER_TUNNEL__CLIENT_CA_PATH`。如果数组或 map 的环境变量表达不清楚，例如 `cluster.peers`、`observability.tracing.headers`，应写入 TOML 或 Helm values，避免 shell 转义导致启动成功但运行语义错误。

每次变更后至少执行三类检查：`/healthz` 证明进程存活，`/readyz` 证明存储和运行依赖可用，`/api-docs/openapi.json` 证明 HTTP router 已加载；Worker 侧再用一个 outbound demo Worker 连接 `server.worker_tunnel_addr`，确认注册、心跳、`DispatchTask`、任务日志和结果回传。若启用 mTLS，先用短期证书在 staging 验证证书链，再切生产 Secret；若启用 OTel，确认 collector 收到 span 后再把 tracing 当作上线证据。

## 通知中心投递

通知中心有独立的通用投递 worker，与 `alert_retry` 分离。它扫描由 notification policies 产生的 `notification_delivery_attempts`，并更新关联的 `notification_messages`。配置形状来自 `crates/tikeo-config/src/lib.rs` 中的 `NotificationDeliveryConfig`，并已出现在 `config/dev.toml` 与 `config/container.toml`。

| Config key | 默认值 | 环境变量 | 说明 |
| --- | --- | --- | --- |
| `notification_delivery.enabled` | `true` | `TIKEO__NOTIFICATION_DELIVERY__ENABLED` | 启用通用通知中心投递 worker。 |
| `notification_delivery.public_console_base_url` | 未设置 | `TIKEO__NOTIFICATION_DELIVERY__PUBLIC_CONSOLE_BASE_URL` | 可选的外部可访问 Web 基地址，用于 provider 卡片中的公开执行控制台链接。 |
| `notification_delivery.interval_seconds` | `60` | `TIKEO__NOTIFICATION_DELIVERY__INTERVAL_SECONDS` | due-attempt 扫描间隔。 |
| `notification_delivery.batch_size` | `50` | `TIKEO__NOTIFICATION_DELIVERY__BATCH_SIZE` | 每轮最大扫描数量。 |
| `notification_delivery.max_attempts` | `3` | `TIKEO__NOTIFICATION_DELIVERY__MAX_ATTEMPTS` | 进入 dead-letter 前最大尝试次数。 |
| `notification_delivery.backoff_seconds` | `300` | `TIKEO__NOTIFICATION_DELIVERY__BACKOFF_SECONDS` | 下一次通用投递重试前的延迟。 |

示例覆盖：

```bash
TIKEO__NOTIFICATION_DELIVERY__ENABLED=true \
TIKEO__NOTIFICATION_DELIVERY__INTERVAL_SECONDS=30 \
TIKEO__NOTIFICATION_DELIVERY__BATCH_SIZE=100 \
TIKEO__NOTIFICATION_DELIVERY__MAX_ATTEMPTS=5 \
TIKEO__NOTIFICATION_DELIVERY__BACKOFF_SECONDS=120 \
cargo run --bin tikeo -- serve --config config/dev.toml
```

`alert_retry` 只影响兼容告警投递尝试；`notification_delivery` 影响 Notification Center messages。不要调整一个队列却期望改变另一个队列。渠道、策略、重试、DLQ 与脱敏行为见 [通知中心参考](./notification-center)。

## 前置条件

执行本页命令前，请先满足页面列出的安装、认证和权限要求。本地示例默认 Server 使用 `config/dev.toml`，客户端访问 `127.0.0.1`，令牌保存在 shell 变量中，不写入文件或截图。

## 验收

完成本页步骤后，用对应 API、UI、构建、smoke 或部署检查验证结果。有效验收至少包含执行的命令、检查的路由或文件，以及观察到的状态或产物。

## 故障排查

步骤失败时，先保留完整命令、响应状态和 Server 日志时间窗口，再检查认证、namespace/app scope、Worker 匹配、存储 readiness 和代理行为，不要直接修改生产配置。

## 生产检查清单

- [ ] 密钥通过环境变量或平台 Secret 引用管理，不写入示例。
- [ ] 已把本地 `127.0.0.1` 命令替换成真实域名、TLS 和认证方式。
- [ ] 已记录变更面的回滚和证据采集方式。
- [ ] 运维人员可以在没有隐藏 shell 历史或隐式状态的情况下复现验收。
