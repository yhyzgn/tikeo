---
title: Docker Compose 部署
description: 使用 Docker Hub 镜像、挂载 config/tikeo.yml、持久化 data/log/tls，并运行 SQLite/PostgreSQL/MySQL Compose stack。
---

# Docker Compose 部署

仓库内置 Compose 文件默认使用 Docker Hub 已发布镜像：

- `yhyzgn/tikeo-server:latest`
- `yhyzgn/tikeo-web:latest`

它们不会从本地 Dockerfile 构建。生产或需要回滚时，在 `.env` 中固定 `TIKEO_IMAGE` 和 `TIKEO_WEB_IMAGE` tag。

Compose 的 service key 与 container_name 都是显式稳定名称：`tikeo-server`、`tikeo-web`、`tikeo-prometheus`、`tikeo-postgres`、`tikeo-mysql`。仓库内置 Web nginx 会把 API/SSE 流量反代到 `http://tikeo-server:9090`，自定义 override 时要保持一致。

## 配置归属

| 配置面 | 应该放什么 | 不应该放什么 |
| --- | --- | --- |
| `config/tikeo.yml` | Tikeo 服务行为：监听、结构化数据库、认证、TLS、日志、集群、重试、通知投递。 | Docker 镜像 tag、宿主机端口、Docker volume 名。 |
| `.env` | Docker/Compose 参数：镜像 tag、宿主机端口、named volume、数据库容器凭据、时区、mimalloc、本地 worker-demo 辅助值。 | 常规部署中的 `TIKEO__...` 服务覆盖。 |
| Compose `environment` | 容器运行时值，例如 `TZ` 和 mimalloc。 | Tikeo 服务配置；请改 `config/tikeo.yml`。 |

## 挂载路径

| 运行时路径 | SQLite stack | PostgreSQL stack | MySQL stack | 含义 |
| --- | --- | --- | --- | --- |
| `/config/tikeo.yml` | `./config/tikeo.yml:/config/tikeo.yml:ro` | 同左 | 同左 | 唯一正式 Server 配置文件。 |
| `/config/tls` | `./config/tls:/config/tls:ro` | 同左 | 同左 | 可选 HTTP/Worker Tunnel TLS/mTLS 证书。 |
| `/data` | `tikeo-data:/data` | `tikeo-data:/data` | `tikeo-data:/data` | SQLite DB 路径和统一 data 挂载。 |
| `/logs` | `tikeo-logs:/logs` | `tikeo-logs:/logs` | `tikeo-logs:/logs` | `log_dir=/logs` 时的可选文件日志。 |
| DB 服务数据 | 不适用 | `tikeo-postgres-data:/var/lib/postgresql/data` | `tikeo-mysql-data:/var/lib/mysql` | 自建数据库持久化。 |

## 首次运行

```bash
cp deploy/compose/tikeo.env.example .env
# Docker 参数改 .env；Tikeo 服务配置改 config/tikeo.yml。

docker compose --env-file .env pull
docker compose --env-file .env up -d
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
open http://127.0.0.1:${TIKEO_WEB_PORT:-8080}
```

## PostgreSQL stack

启动前编辑 `config/tikeo.yml`：

```yaml
storage:
  database:
    type: postgres
    host: tikeo-postgres
    port: 5432
    username: tikeo
    password: "p@ss/word:with#chars"
    database: tikeo
    params:
      sslmode: disable
```

然后运行：

```bash
cp deploy/compose/tikeo.env.example .env
# 保持 .env 数据库容器凭据与 config/tikeo.yml 一致。
docker compose --env-file .env -f docker-compose.postgres.yml pull
docker compose --env-file .env -f docker-compose.postgres.yml up -d
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
```

## MySQL stack

启动前编辑 `config/tikeo.yml`：

```yaml
storage:
  database:
    type: mysql
    host: tikeo-mysql
    port: 3306
    username: tikeo
    password: "p@ss/word:with#chars"
    database: tikeo
```

然后运行：

```bash
cp deploy/compose/tikeo.env.example .env
# 保持 .env 数据库容器凭据与 config/tikeo.yml 一致。
docker compose --env-file .env -f docker-compose.mysql.yml pull
docker compose --env-file .env -f docker-compose.mysql.yml up -d
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
```

## 为什么使用结构化数据库字段

请使用 `storage.database.*` 字段配置数据库。密码中包含 `@`、`/`、`:`、`#` 时可以直接写普通值；Tikeo 会自动对内部 URL 做 percent-encode。

## 可选 Prometheus

```bash
docker compose --env-file .env --profile observability up -d tikeo-prometheus
curl -fsS http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/-/ready
```

## Worker 连接

Worker 主动连接 Server Worker Tunnel。本地 demo 使用 `http://127.0.0.1:9998` 或 `.env` 中的 `TIKEO_WORKER_TUNNEL_PUBLIC_ENDPOINT`。不要暴露任意业务 Worker 端口。
## 前置条件

- 已安装 Docker 和 Docker Compose v2。
- `config/tikeo.yml` 已按目标数据库、TLS、日志和公网 URL 检查。
- `/config/tikeo.yml`、`/config/tls`、`/data`、`/logs` 对应的 host path 或 named volume 已准备好。

## 验收

执行页面中的启动命令后，检查 `/healthz`、`/readyz` 和 Web 控制台。数据库模式还要确认 DB volume 或托管数据库已有备份策略。

## 故障排查

启动失败时先看 `docker compose logs tikeo-server`，确认 `config/tikeo.yml` 已挂载到 `/config/tikeo.yml`，再检查结构化数据库 host、port、username、password 和 database。

## 生产检查清单

- [ ] 对需要回滚的环境固定镜像版本。
- [ ] `/config/tikeo.yml`、`/config/tls`、`/data`、`/logs` 挂载与部署模式一致。
- [ ] 数据库凭据没有提交到源码。
- [ ] 生产流量前已验证 Worker Tunnel 和 SSE 代理行为。
