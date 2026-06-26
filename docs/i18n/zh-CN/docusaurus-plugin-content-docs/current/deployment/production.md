---
title: 生产部署指南
description: Tikeo Server、Web、数据库、Worker Tunnel、挂载、TLS、观测、备份、回滚和 smoke 验证的生产 runbook。
keywords: [tikeo 生产部署, docker compose, helm, worker tunnel, postgres, mysql]
---

# 生产部署指南

生产 Tikeo 环境包含五个职责：

| 组件 | 职责 | 暴露给谁 |
| --- | --- | --- |
| Tikeo Server | HTTP API、调度、migration、Worker Tunnel、通知、审计。 | 运维、SDK Management clients、主动连接的 Workers。 |
| Tikeo Web | Jobs、Workers、Workflows、Scripts、Notifications、Audit、RBAC 控制台。 | 人类操作员。 |
| Database | 持久化任务、实例、日志、RBAC、通知、审计、集群 ownership、outbox。 | 仅 Server。 |
| Worker 进程 | 执行 normal processors/scripts/plugins 并回传日志和结果。 | 只主动连接 Worker Tunnel。 |
| 通知供应商 | 接收渲染后的消息。 | Server 出站。 |

Worker 不开放入站任务端口，只拨出连接 `server.worker_tunnel_addr`。

## 选择部署路径

| 场景 | 推荐路径 | 说明 |
| --- | --- | --- |
| 本机评估 | `config/dev.yml` 或 Compose SQLite | 快速、可丢弃。 |
| 小型 VM | 单二进制 + systemd + PostgreSQL/MySQL | 运维简单。 |
| 共享非 Kubernetes 环境 | Docker Compose + PostgreSQL/MySQL | 使用发布镜像和显式挂载。 |
| Kubernetes 生产 | Helm + 外部数据库 + ingress/gateway | 支持 HA、Secrets、TLS/mTLS、平台观测。 |
| 离线/强变更控制 | 固定镜像 digest 和 release assets | 具备可复现回滚证据。 |

SQLite 仅适合可接受单节点本地持久化的场景。生产优先 PostgreSQL/MySQL/CockroachDB-compatible PostgreSQL wire。

## Server 基线配置

从唯一正式生产模板开始：

```bash
cp config/tikeo.yml /etc/tikeo/tikeo.yml
```

在挂载 YAML 中设置生产值。优先使用结构化数据库字段；密码包含 `@`、`/`、`:`、`#` 时无需手动 URL encode。

```yaml
server:
  listen_addr: "0.0.0.0:9090"
  worker_tunnel_addr: "0.0.0.0:9998"

storage:
  database:
    type: postgres
    host: postgres.example
    port: 5432
    username: tikeo
    password: "p@ss/word:with#chars"
    database: tikeo
    params:
      sslmode: require

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
        path: /logs
```

完整 Server/Worker 表格见 [配置参考](../reference/configuration)。

## 挂载与持久化目录

| 部署面 | 配置路径 | Data/db 路径 | 日志路径 | 挂载建议 |
| --- | --- | --- | --- | --- |
| Docker 镜像默认 | `/config/tikeo.yml` | SQLite 用 `/data/tikeo.db` | 默认 stdout，除非启用文件日志 | 快速评估可用；SQLite 不可丢时挂 `/data`。 |
| Docker 外部配置 | `/config/tikeo.yml` | SQLite 用 `/data/tikeo.db` | `file.path=/logs` 时 `/logs/tikeo.log` | 只读挂载 config，并挂 `/config/tls`、`/data`、`/logs`。 |
| Docker Compose SQLite | `/config/tikeo.yml` | `tikeo-data:/data` | `tikeo-logs:/logs` | Compose 已显式挂载 config、TLS、data、logs。 |
| Docker Compose PostgreSQL | `/config/tikeo.yml` | DB 服务的 `tikeo-postgres-data:/var/lib/postgresql/data` | `tikeo-logs:/logs` | Server `/data` 只是统一运行时挂载；DB 状态在数据库服务。 |
| Docker Compose MySQL | `/config/tikeo.yml` | DB 服务的 `tikeo-mysql-data:/var/lib/mysql` | `tikeo-logs:/logs` | 备份 MySQL volume 或托管数据库。 |
| Kubernetes 原始 manifest | ConfigMap 中 `/config/tikeo.yml` | SQLite manifest 中 PVC 挂 `/data` | 默认 stdout | 只有启用文件日志时才加日志 PVC。 |
| Kubernetes Raft/HA | `/config/tikeo.yml` + 结构化 DB Secret | 外部 DB | 默认 stdout | 使用 StatefulSet/headless peers 与 Secret 注入 DB 字段。 |
| Helm SQLite | chart ConfigMap 中 `/config/tikeo.yml` | `/data` PVC | 默认 stdout | 仅开发/小型单节点。 |
| Helm 外部 DB | `/config/tikeo.yml` + 结构化 DB Secret keys | 托管/自建 DB | 默认 stdout | Secret keys：`type`、`host`、`port`、`username`、`password`、`database`。 |
| Binary/systemd | `/etc/tikeo/tikeo.yml` | 本地 SQLite 用 `/var/lib/tikeo` | 启用时 `/var/log/tikeo/tikeo.log` | 目录归属 `tikeo` 用户。 |
| Web/Docs 静态镜像 | 不需要 | 不需要 | nginx stdout | 无持久化数据。 |

## Docker run 形态

```bash
mkdir -p ./tikeo/config/tls ./tikeo/data ./tikeo/logs
cp config/tikeo.yml ./tikeo/config/tikeo.yml

docker run -d --name tikeo-server \
  -p 9090:9090 -p 9998:9998 \
  -v "$PWD/tikeo/config/tikeo.yml:/config/tikeo.yml:ro" \
  -v "$PWD/tikeo/config/tls:/config/tls:ro" \
  -v "$PWD/tikeo/data:/data" \
  -v "$PWD/tikeo/logs:/logs" \
  yhyzgn/tikeo-server:latest \
  serve --config /config/tikeo.yml
```

## Docker Compose 形态

```bash
cp deploy/compose/tikeo.env.example .env
# Docker 参数改 .env；Tikeo 服务配置改 config/tikeo.yml。
docker compose --env-file .env pull
docker compose --env-file .env up -d
curl -fsS http://127.0.0.1:9090/readyz
```

使用 `docker-compose.postgres.yml` 或 `docker-compose.mysql.yml` 前，先把 `config/tikeo.yml` 的 `storage.database.type` 和 host/user/password/database 改成对应数据库服务。

## Kubernetes 与 Helm 形态

Helm 外部数据库模式创建结构化 Secret keys：

```bash
kubectl -n tikeo create secret generic tikeo-database \
  --from-literal=type=postgres \
  --from-literal=host=postgres.example \
  --from-literal=port=5432 \
  --from-literal=username=tikeo \
  --from-literal=password='p@ss/word:with#chars' \
  --from-literal=database=tikeo

helm upgrade --install tikeo deploy/helm/tikeo \
  --namespace tikeo \
  --set server.storage.mode=external \
  --set server.storage.type=postgres \
  --set server.storage.existingSecret=tikeo-database
```

Web 和 HTTP API 走常规 ingress。Worker Tunnel 单独规划：使用支持 gRPC/HTTP2 的控制器路径，或给 Worker 暴露专用 LoadBalancer/service。

## TLS/mTLS 与 SSE

- 只有挂载 `/config/tls` 或 Kubernetes Secret 后再启用 `transport_security.http.*` / `transport_security.worker_tunnel.*`。
- SSE endpoint 需要关闭代理 buffering，并设置较长 read/idle timeout。见 [SSE 实时刷新部署](./sse-realtime)。
- `0.0.0.0` 只是绑定地址，不要作为客户端 URL。

## Smoke 验证

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:8080/ >/dev/null
```

然后至少连接一个 Worker，触发测试任务，查看实例日志；如果启用了通知策略，还要验证通知投递。
