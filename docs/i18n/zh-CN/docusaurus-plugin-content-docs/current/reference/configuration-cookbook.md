---
title: 配置 Cookbook
description: 本地开发、生产数据库、TLS/mTLS、OIDC、观测、通知和 Worker SDK 设置的场景化配置示例。
---

# 配置 Cookbook

需要完整默认值时读 [配置参考](./configuration)。已经知道场景、需要能工作的配置形状时读本页。

## 配置如何加载

Tikeo 读取 TOML 文件后，再应用环境变量覆盖。覆盖格式：

```text
TIKEO__SECTION__KEY=value
TIKEO__SECTION__SUBSECTION__KEY=value
```

示例：

```bash
export TIKEO__STORAGE__DATABASE_URL='postgres://tikeo:secret@postgres:5432/tikeo'
export TIKEO__SERVER__LISTEN_ADDR='0.0.0.0:9090'
export TIKEO__NOTIFICATION_DELIVERY__ENABLED='true'
```

## Recipe：本地开发

使用 `config/dev.toml`：

```toml
[server]
listen_addr = "0.0.0.0:9090"
worker_tunnel_addr = "0.0.0.0:9998"

[storage]
database_url = "sqlite://.dev/tikeo-dev.db?mode=rwc"
timestamp_offset = "+08:00"
```

启动：

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://127.0.0.1:9090/readyz
```

这只适合本地。客户端 URL 用 `127.0.0.1`，不要用监听地址。

## Recipe：PostgreSQL 生产

```toml
[storage]
database_url = "postgres://tikeo:tikeo@postgres:5432/tikeo"
```

推荐用环境变量覆盖：

```bash
export TIKEO__STORAGE__DATABASE_URL="postgres://tikeo:${TIKEO_DB_PASSWORD}@postgres:5432/tikeo"
```

检查清单：数据库已创建；用户可执行迁移；需要 TLS 时连接串满足 provider 要求；备份恢复由数据库运维负责。

## Recipe：MySQL 生产

```toml
[storage]
database_url = "mysql://tikeo:tikeo@mysql:3306/tikeo"
timestamp_offset = "+08:00"
```

请使用 `utf8mb4` 支持完整 Unicode payload/log。新的 MySQL 版本进入团队环境前，先跑仓库数据库兼容测试。

## Recipe：HTTP TLS 在反向代理终止

Tikeo HTTP 在私网明文，TLS 在 ingress/proxy 终止：

```toml
[server]
listen_addr = "0.0.0.0:9090"
```

代理要求：转发 `/api/*` 和 `/api-docs/openapi.json` 到 Server；Web 静态路由到 Web nginx；SSE 禁用 buffering 并设置长 read timeout；如果用到转发头和安全 cookie，要统一配置。

## Recipe：Worker Tunnel TLS 或 mTLS

本地明文：

```toml
[transport_security.worker_tunnel]
tls_enabled = false
mtls_required = false
```

跨网络生产建议 TLS 或 mTLS。文件路径由 Secret/config 管理挂载。完整 key 见 [配置参考](./configuration)；Helm 中是 `server.tls.workerTunnel.mtlsRequired` 等 values。

mTLS 发布顺序：签发 CA 和 server cert；配置 Server Worker Tunnel TLS；配置一个测试 Worker 信任 CA；给该 Worker 启用 client cert；最后再全局要求 mTLS。

## Recipe：本地登录与 OIDC 准备

```toml
[auth]
local_login_enabled = true

[auth.oidc]
enabled = false
scopes = ["openid", "profile", "email"]
```

启用 OIDC 时，通过环境变量或 Secret 设置 issuer/client；先在 staging 验证登录，再考虑关闭本地 fallback。

## Recipe：观测

```toml
[observability.logging]
level = "info"
log_dir = "/var/log/tikeo"

[observability.tracing]
enabled = true
otlp_endpoint = "http://otel-collector:4318/v1/traces"
headers = []
```

文件日志用于事故复盘，OTel 用于跨服务 trace。自定义 tracing headers 不要放 bearer token 或 provider secret。

## Recipe：通知中心投递 worker

```toml
[notification_delivery]
enabled = true
# public_console_base_url = "https://tikeo.example.com"
interval_seconds = 60
batch_size = 50
max_attempts = 3
backoff_seconds = 300
```

含义：`interval_seconds` 是扫描间隔；`batch_size` 是每轮最大 attempt；`max_attempts` 是进入 DLQ 前尝试次数；`backoff_seconds` 是失败后的重试延迟。通过通知中心队列视图或 `notification-delivery-attempts:queue-status` 查看 retry/DLQ。

## Recipe：Worker SDK 默认值

| 概念 | 本地典型值 | 生产值 |
| --- | --- | --- |
| endpoint | `http://127.0.0.1:9998` | 带 TLS/mTLS 的 Worker Tunnel URL |
| namespace | service namespace | 平台批准的 namespace |
| app | app name | 平台批准的 app |
| workerPool | `default` | 按容量/SLO/安全划分的 pool |
| heartbeat interval | SDK 默认 | 观察 lease 和网络后再调 |
| processor names | demo 名称 | 服务拥有的稳定名称 |

## Recipe：发布镜像固定

使用明确 tag 或 digest：

```bash
docker pull yhyzgn/tikeo-server:v${TIKEO_VERSION}
docker run --rm yhyzgn/tikeo-server:v${TIKEO_VERSION} --version
```

除非故意测试混合版本，否则 Server、Web、Docs 镜像 tag 应与 release version 对齐。

## 前置条件

- 已知道要配置的部署模式。
- 已有数据库 URL 和 Secret 管理路径。
- 已判断 Worker Tunnel 是否跨信任边界。
- 已决定该环境是否运行通知中心投递 worker。

## 验收

任意 recipe 至少验证：

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

然后运行对应部署路径的 smoke。

## 故障排查

| 现象 | 检查 |
| --- | --- |
| 配置像是没生效 | 确认 `--config`、`TIKEO_CONFIG`、`TIKEO__...` 拼写。 |
| 数据库连接失败 | URL scheme、凭据、DNS、TLS 要求、数据库是否存在。 |
| Worker TLS 失败 | CA 路径、SNI/hostname、client cert、ingress 协议。 |
| OIDC 登录循环 | issuer URL、redirect URI、cookie security、时钟偏移。 |
| 通知一直 retry | provider 出站网络、channel 凭据、retry worker enabled。 |

## 生产检查清单

- [ ] 所有 secret 通过环境变量/平台 Secret 注入，未提交 TOML。
- [ ] 数据库 URL、TLS/mTLS、OIDC、通知投递设置已记录。
- [ ] 配置变更包含验收命令和回滚说明。
- [ ] 本地 bind address 没有被当作客户端 URL。
