---
title: 生产部署指南
description: 面向人的 Tikeo 生产部署 runbook，覆盖 Server、Web、Docs、数据库、Worker Tunnel 网络、TLS、观测、备份、回滚和 smoke 验收。
---

# 生产部署指南

这页是复制 YAML 之前应该先读的部署手册。它解释要部署哪些组件、哪些东西应该留在集群外、流量如何流动、哪些配置项最关键，以及怎样证明安装真的可用。精确 manifest 请继续阅读 [Docker Compose](./docker-compose)、[Kubernetes 与 Helm](./kubernetes)、[单二进制](./single-binary)、[SSE 实时通道](./sse-realtime) 和 [Management trigger smoke](./management-trigger-smoke-runbook)。

## 部署目标

一个生产 Tikeo 环境包含以下职责：

| 组件 | 运行位置 | 职责 | 暴露给谁 |
| --- | --- | --- | --- |
| Tikeo Server | 容器、VM 或 Kubernetes Deployment | HTTP API、调度器、迁移、Worker Tunnel、通知投递 worker | 运维、SDK Management 客户端、出站 Worker |
| Tikeo Web | 静态 nginx 容器或静态托管 | 作业、Worker、工作流、脚本、通知、审计、RBAC 控制台 | 人类运维 |
| 数据库 | 托管 PostgreSQL/MySQL；SQLite 仅限本地/小型单节点 | 持久化任务、实例、日志、RBAC、通知、审计 | 仅 Server |
| Worker 进程 | 应用集群、私有 VPC、VM、sidecar 或外部网络 | 执行 SDK processor/脚本/插件并上报日志结果 | 只出站连接 Worker Tunnel |
| 通知提供方 | SaaS/webhook/email/PagerDuty/办公机器人 | 接收渲染后的通知 payload | 仅 Server 出站 |

最重要的边界是：**业务 Worker 不暴露入站任务端口**。它们主动拨出到 `server.worker_tunnel_addr`。不要为了让调度器调用 Worker 而创建业务 Worker Service。

## 选择安装路径

| 场景 | 推荐路径 | 原因 |
| --- | --- | --- |
| 本地评估 | `config/dev.toml` + Web dev server 或 Compose | 快速、SQLite 可丢弃、日志清晰。 |
| 小型内部 VM | 单二进制 + systemd + PostgreSQL/MySQL | 运维简单，适合一个 Server 节点。 |
| 团队共享环境 | Docker Compose + PostgreSQL/MySQL | 可复现，接近 release 镜像，容易 smoke。 |
| Kubernetes 生产 | Helm chart + 外部数据库 + ingress/gateway | Server/Web 分离，支持 TLS/mTLS 和平台 Secret。 |
| 离线或严格变更控制 | 固定 Docker image digest 和 release assets | 可重复 rollout/rollback。 |

SQLite 只适合明确接受单节点本地持久性的场景。生产优先 PostgreSQL 或 MySQL，并用你们现有数据库体系备份。

## 网络模型

请分开规划四条流量：

1. **人类 Web 流量**：浏览器 → Web nginx → Server API。代理要支持长 API 和 SSE。
2. **Management API 流量**：应用/服务 → Server HTTP API。应用用 `x-tikeo-api-key`，人类用会话 token。
3. **Worker Tunnel 流量**：Worker → Server Worker Tunnel endpoint（默认 `9998`）。TLS/gateway 场景必须支持 gRPC/HTTP2。
4. **通知提供方流量**：Server → Slack/DingTalk/Feishu/WeCom/PagerDuty/email/webhook。密钥在渠道配置或环境兼容引用中。

不要把 `0.0.0.0` 当客户端 URL。它只是监听地址。客户端应使用 `127.0.0.1`、Service DNS 或真实域名。

## 基线配置

从提交的配置文件开始：

```bash
cp config/postgres.toml /etc/tikeo/tikeo.toml
# 或
cp config/mysql.toml /etc/tikeo/tikeo.toml
```

用环境变量或平台 Secret 注入数据库凭据：

```bash
export TIKEO__STORAGE__DATABASE_URL='postgres://tikeo:${PASSWORD}@postgres:5432/tikeo'
export TIKEO__SERVER__LISTEN_ADDR='0.0.0.0:9090'
export TIKEO__SERVER__WORKER_TUNNEL_ADDR='0.0.0.0:9998'
export TIKEO__OBSERVABILITY__LOGGING__LEVEL='info'
```

环境变量约定是 `TIKEO__SECTION__KEY`，例如 `storage.database_url` 对应 `TIKEO__STORAGE__DATABASE_URL`。完整默认值见 [配置参考](../reference/configuration)，按场景复制见 [配置 Cookbook](../reference/configuration-cookbook)。

## Docker Compose 生产形态

共享非 Kubernetes 环境可以这样启动：

```bash
cp deploy/compose/tikeo.env.example .env
# 编辑 .env：镜像 tag、端口、数据库密码、时区。
docker compose --env-file .env up -d --build
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:8080/ >/dev/null
```

需要数据库在同一 stack 时使用 `docker-compose.postgres.yml` 或 `docker-compose.mysql.yml`。生产建议使用托管数据库。`.env` 中要明确 volume 名、镜像 tag 和端口映射，便于回滚。

## Kubernetes 生产形态

Kubernetes 使用 Helm 和外部数据库 Secret：

```bash
kubectl create namespace tikeo
kubectl -n tikeo create secret generic tikeo-database \
  --from-literal=database-url='postgres://tikeo:${PASSWORD}@postgres:5432/tikeo'

helm upgrade --install tikeo deploy/helm/tikeo \
  --namespace tikeo \
  --set server.envFromSecret=tikeo-database \
  --set server.service.type=ClusterIP \
  --set web.service.type=ClusterIP
```

Web 和 Server API 走常规 ingress。Worker Tunnel 要单独处理：使用支持 gRPC/HTTP2 的控制器路径，或为 Worker 暴露独立 LoadBalancer/Service。Nginx Ingress、Envoy Gateway、Traefik 和 Gateway API 的具体配置见 [Kubernetes 控制器 runbook](./kubernetes-controller-runbook)。

## TLS 与 mTLS 决策

| 流量 | 最低要求 | 生产建议 |
| --- | --- | --- |
| Web/API 浏览器流量 | ingress/proxy TLS | 边缘 TLS、安全 cookie、公网场景加 WAF/限流。 |
| SDK Management API | TLS | TLS + 应用级 API Key + 范围权限。 |
| Worker Tunnel | 本地可明文 | 跨网络使用 TLS 或 mTLS。 |
| 通知提供方出站 | provider HTTPS/SMTP TLS | 密钥轮换，启用策略前先 test-send。 |

相关配置命名空间是 `transport_security.http` 和 `transport_security.worker_tunnel`。Helm 中对应 `server.tls.workerTunnel.mtlsRequired` 等 values。

## 初始化与权限

Server 可达后，只创建一次首个 Owner：

```bash
curl -fsS http://127.0.0.1:9090/api/v1/auth/bootstrap | jq .data.registrationOpen
TOKEN="$(curl -fsS -X POST http://127.0.0.1:9090/api/v1/auth/bootstrap/register \
  -H 'content-type: application/json' \
  -d '{"username":"owner","email":"owner@example.com","password":"Tikeo@2026!","confirmPassword":"Tikeo@2026!"}' | jq -r .data.token)"
```

随后用 Web 或 Management API 创建 namespace/app、service account 和 SDK API Key。生产 Worker 不应使用人类 session token。

## Worker 发布模式

每个服务团队按这个顺序接入：

1. 选择语言 SDK 并阅读对应 SDK 文档。
2. 约定稳定的 `namespace`、`app`、`workerPool` 和 processor 名称。
3. 设置 `TIKEO_WORKER_ENDPOINT` 或语言 SDK 的 `WorkerConfig.endpoint`。
4. 先启动一个 Worker，在 **Workers** 页面确认在线。
5. 触发测试任务，确认 `executionMode=single` 或广播行为。
6. 只有日志和结果证据正确后再扩容 Worker。

## 观测与证据

生产流量前至少检查：

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:9090/api-docs/openapi.json >/tmp/tikeo-openapi.json
```

启用文件日志和 OpenTelemetry 示例：

```toml
[observability.logging]
level = "info"
log_dir = "/var/log/tikeo"

[observability.tracing]
enabled = true
otlp_endpoint = "http://otel-collector:4318/v1/traces"
headers = []
```

运维证据应包含 health/ready、bootstrap 结果、Worker 在线快照、触发实例、任务日志、审计事件，以及如果启用通知则包含投递 attempt。

## 备份与恢复

备份数据库，而不只是容器 volume。有效 runbook 包含：

- 数据库备份计划和恢复演练。
- 配置文件或 Helm values 版本。
- Docker image tag/digest 或 release asset checksum。
- Secret 名称和轮换流程，但不打印 secret 值。
- 恢复后要跑的 smoke 命令。

SQLite 只有在 Server 停止或数据库安全 checkpoint 后才适合文件复制。PostgreSQL/MySQL 用原生备份工具。

## 升级与回滚

1. 阅读 release notes 和镜像 tag。
2. 在 staging 更新 Server 镜像。
3. 运行 health/ready 和 management trigger smoke。
4. 验证 Web 静态包和 Worker Tunnel。
5. 推进生产。
6. 回滚到上一个镜像 tag 和配置/Helm values。数据库迁移不一定可逆，生产升级前必须演练。

验证 Server 容器版本：

```bash
docker run --rm yhyzgn/tikeo-server:v0.2.9 --version
```

## 前置条件

- 可达数据库 endpoint 或明确的本地 SQLite 评估路径。
- Docker 或 Kubernetes 权限。
- Web/API 与 Worker Tunnel 的 DNS/TLS 方案。
- Owner 初始化计划和至少一个应用级 SDK API Key。
- 数据库 URL 与通知 provider 凭据的 Secret 管理方案。

## 验收

生产就绪验收应通过：

```bash
curl -fsS https://tikeo.example.com/readyz
curl -fsS https://tikeo.example.com/api-docs/openapi.json >/tmp/openapi.json
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```

Kubernetes 还应检查：

```bash
kubectl -n tikeo get pods,svc,ingress
kubectl -n tikeo logs deploy/tikeo-server --tail=120
```

## 故障排查

| 现象 | 优先检查 |
| --- | --- |
| Web 能打开但 API 失败 | 反向代理 API 路径、CORS/origin、Server service DNS、auth token。 |
| Worker 不在线 | Worker Tunnel URL、gRPC/HTTP2 代理、TLS/mTLS CA/client cert、防火墙出站。 |
| Job 一直 pending | Worker capability 不匹配、Worker disabled、namespace/app 不匹配、queue/lease 日志。 |
| 通知测试失败 | Channel enabled、target configured、secret refs resolved、provider 出站网络。 |
| SSE 不更新 | 代理 buffering/timeout；见 [SSE 实时通道](./sse-realtime)。 |

## 生产检查清单

- [ ] 数据库使用 PostgreSQL/MySQL，或明确接受 SQLite 单节点路径。
- [ ] Server/Web/Docs 镜像固定 tag 或 digest。
- [ ] Worker Tunnel 可被 Worker 网络访问，且不要求 Worker 入站端口。
- [ ] API 和 Worker Tunnel 的 TLS/mTLS 决策已记录。
- [ ] Owner bootstrap 已完成并关闭。
- [ ] 自动化使用应用级 SDK API Key，而非人类 session token。
- [ ] 已采集 health/ready/OpenAPI/Worker/instance/log/audit smoke 证据。
- [ ] 生产流量前已验证备份与回滚。
