---
title: Docker Compose 部署
description: Tikeo Docker Compose 与 docker run 完整部署指南，覆盖 SQLite、PostgreSQL、MySQL、Web、Prometheus、持久化挂载和 SSE 代理配置。
---

# Docker Compose 部署

这一页是用 Docker Hub 已发布镜像运行 Tikeo 的运维手册。它覆盖仓库内置的 Docker Compose stack，也补充了在没有 Compose 的主机上使用 `docker run` 手工部署的方式。

Compose 文件默认使用这些镜像：

- `yhyzgn/tikeo-server:latest`
- `yhyzgn/tikeo-web:latest`
- 启用 `observability` profile 时使用 `prom/prometheus:latest`

这些 Compose 文件不会从本地 Dockerfile 构建镜像。生产环境应在 `.env` 中把 `TIKEO_IMAGE` 和 `TIKEO_WEB_IMAGE` 固定到明确发布版本，不要直接依赖 `latest`。

## 部署方式一览

| 方式 | 文件或命令 | 存储 | 包含 Web | 包含 Prometheus | 适用场景 |
| --- | --- | --- | --- | --- | --- |
| SQLite Compose | `docker-compose.yml` | `tikeo-data` 中的 `/data/tikeo.db` | 是 | 可选 profile | 单节点 demo、小规模部署、首次验证 |
| PostgreSQL Compose | `docker-compose.postgres.yml` | `tikeo-postgres` 服务和 `tikeo-postgres-data` | 是 | 可选 profile | 同一主机上自建 PostgreSQL 的共享环境 |
| MySQL Compose | `docker-compose.mysql.yml` | `tikeo-mysql` 服务和 `tikeo-mysql-data` | 是 | 可选 profile | 标准化使用 MySQL 的共享环境 |
| Compose + Prometheus | 增加 `--profile observability` | 与所选 stack 相同 | 是 | `tikeo-prometheus` | 本地 SLO、指标和告警规则验证 |
| Docker run | 显式 `docker run` 命令 | 挂载路径或外部数据库 | 可选 | 可选 | 没有 Compose 的主机、调试容器连线 |

## 命名、网络和镜像策略

service key 和 `container_name` 都是显式稳定名称：

- `tikeo-server`
- `tikeo-web`
- `tikeo-prometheus`
- `tikeo-postgres`
- `tikeo-mysql`

这些名称同时也是 Compose 网络内的 Docker DNS 名称。Web nginx 会把 API 和 SSE 流量转发到 `http://tikeo-server:9090`，Prometheus 会抓取 `tikeo-server:9090`。不要只改其中一边；如果使用自定义 Compose override，必须同步 nginx 和 Prometheus 的 upstream。

镜像名与 service 名是两件事。除非你明确维护私有镜像仓库，否则保持 `yhyzgn/tikeo-server` 和 `yhyzgn/tikeo-web` 不变。

## 配置归属

| 配置面 | 应该放什么 | 不应该放什么 |
| --- | --- | --- |
| `config/tikeo.yml` | Tikeo 服务行为：HTTP 监听、`server.worker_tunnel_addr`、`storage.database.*`、认证、TLS、日志、集群、重试、通知公网 URL。 | Docker 镜像 tag、宿主机端口、Compose volume 名。 |
| 从 `deploy/compose/tikeo.env.example` 复制出的 `.env` | Docker 参数：镜像 tag、宿主机端口、named volume、数据库容器凭据、时区、mimalloc、本地 worker-demo 辅助值。 | 常规部署中的 `TIKEO__STORAGE__DATABASE__HOST` 等服务覆盖；正常改 YAML。 |
| Compose `environment` | 容器运行时值，例如 `TZ` 和 mimalloc 策略。 | Tikeo 业务配置。 |
| nginx `web/nginx/default.conf` | Web 静态资源、API 反向代理、SSE 安全代理行为。 | Server 存储、认证、TLS 或调度配置。 |

关键文件如下：

| 路径 | 用途 |
| --- | --- |
| `docker-compose.yml` | Server + Web + 可选 Prometheus，默认 SQLite。 |
| `docker-compose.postgres.yml` | Server + Web + PostgreSQL + 可选 Prometheus。 |
| `docker-compose.mysql.yml` | Server + Web + MySQL + 可选 Prometheus。 |
| `deploy/compose/tikeo.env.example` | 复制为 `.env`；控制镜像 tag、宿主机端口、volume、数据库容器凭据、本地 Worker 辅助变量。 |
| `config/tikeo.yml` | 挂载到 Server 容器的 `/config/tikeo.yml`；这是正式 Server 配置文件。 |
| `observability/prometheus/prometheus.yml` | Compose observability profile 使用的 Prometheus 抓取与规则文件配置。 |
| `observability/prometheus/tikeo-recording-rules.yml` | Dashboard 和告警使用的 recording rules。 |
| `observability/prometheus/tikeo-alert-rules.yml` | Prometheus 加载的 alert rules。 |

## 运行时挂载和持久化数据

| 运行时路径 | SQLite stack | PostgreSQL stack | MySQL stack | 含义 |
| --- | --- | --- | --- | --- |
| `/config/tikeo.yml` | `./config/tikeo.yml:/config/tikeo.yml:ro` | 同左 | 同左 | Server 配置文件。 |
| `/config/tls` | `./config/tls:/config/tls:ro` | 同左 | 同左 | 可选 HTTP 和 Worker Tunnel TLS/mTLS 文件。 |
| `/data` | `tikeo-data:/data` | `tikeo-data:/data` | `tikeo-data:/data` | SQLite 数据库路径和统一运行时 data 挂载。 |
| `/logs` | `tikeo-logs:/logs` | `tikeo-logs:/logs` | `tikeo-logs:/logs` | `observability.logging.channels.file.path` 为 `/logs` 时的文件日志。 |
| DB 服务数据 | 不使用 | `tikeo-postgres-data:/var/lib/postgresql/data` | `tikeo-mysql-data:/var/lib/mysql` | 自建数据库持久化存储。 |

备份时要备份真正拥有数据库的 volume。SQLite 模式是 `tikeo-data`；PostgreSQL 和 MySQL 模式分别是对应数据库服务 volume。

## 准备 `.env`

每个部署目录创建一次 `.env`：

```bash
cp deploy/compose/tikeo.env.example .env
```

首次启动前重点检查：

| 变量 | 默认值 | 含义 |
| --- | --- | --- |
| `TIKEO_IMAGE` | `yhyzgn/tikeo-server:latest` | Server 镜像；生产固定到 release tag。 |
| `TIKEO_WEB_IMAGE` | `yhyzgn/tikeo-web:latest` | Web 控制台镜像；应与 Server 使用同一发布线。 |
| `TIKEO_HTTP_PORT` | `9090` | 映射到 Server HTTP、OpenAPI、metrics、健康检查和 Web upstream 的宿主机端口。 |
| `TIKEO_WORKER_TUNNEL_PORT` | `9998` | Worker Tunnel 宿主机端口；Worker 主动连接这里。 |
| `TIKEO_WEB_PORT` | `8080` | Web 控制台宿主机端口。 |
| `TIKEO_PROMETHEUS_PORT` | `9091` | 启用 observability 时 Prometheus 的宿主机端口。 |
| `TIKEO_DATA_VOLUME` | `tikeo-data` | Server data volume；SQLite stack 下拥有 SQLite 数据。 |
| `TIKEO_LOGS_VOLUME` | `tikeo-logs` | Server 日志 volume。 |
| `TIKEO_POSTGRES_*` | 见示例文件 | PostgreSQL 容器数据库、用户、密码、宿主机端口和 volume。凭据要与 `config/tikeo.yml` 一致。 |
| `TIKEO_MYSQL_*` | 见示例文件 | MySQL 容器数据库、用户、密码、root 密码、宿主机端口和 volume。凭据要与 `config/tikeo.yml` 一致。 |
| `TIKEO_WORKER_TUNNEL_PUBLIC_ENDPOINT` | `http://127.0.0.1:9998` | 本地 Worker demo 辅助值；Server 容器不会读取它。 |

## 准备 `config/tikeo.yml`

Server 容器运行：

```bash
tikeo serve --config /config/tikeo.yml
```

启动前编辑 `config/tikeo.yml`：

- `server.listen_addr` 控制容器内 Server 监听地址。
- `server.worker_tunnel_addr` 控制容器内 Worker Tunnel 监听地址。
- `storage.database.*` 选择 SQLite、PostgreSQL、MySQL 或其他受支持数据库。
- `notification_delivery.public_console_base_url` 应填写浏览器可访问的 Web 控制台地址，例如本地 Compose 部署的 `http://127.0.0.1:8080`。
- 启用 HTTP 或 Worker Tunnel TLS/mTLS 时，`transport_security.*` 指向 `/config/tls` 下的文件。
- 如果希望文件日志写入 `tikeo-logs` volume，`observability.logging.channels.file.path` 保持 `/logs`。

数据库配置优先使用结构化字段而不是 URL 字符串。密码包含 `@`、`/`、`:` 或 `#` 时可以直接作为 YAML 值填写，Tikeo 会安全地编码内部数据库 URL。

## SQLite Compose 运行手册

当单个 Server 容器拥有数据库文件时使用这一模式。

1. 准备配置：

   ```bash
   cp deploy/compose/tikeo.env.example .env
   # 可选：编辑 .env，固定 TIKEO_IMAGE、TIKEO_WEB_IMAGE 和宿主机端口。
   # config/tikeo.yml 中 storage.database.type 保持 sqlite。
   ```

2. 校验渲染后的 Compose 文件：

   ```bash
   docker compose --env-file .env -f docker-compose.yml config --quiet
   ```

3. 拉取并启动。因为 `docker-compose.yml` 是默认 Compose 文件名，短命令和显式 `-f docker-compose.yml` 等价：

   ```bash
   docker compose --env-file .env pull
   docker compose --env-file .env up -d
   ```

   ```bash
   docker compose --env-file .env -f docker-compose.yml pull
   docker compose --env-file .env -f docker-compose.yml up -d
   ```

4. 验证业务端点和控制台：

   ```bash
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/healthz
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/api/v1/metrics/summary
   curl -fsS http://127.0.0.1:${TIKEO_WEB_PORT:-8080}/
   ```

5. 查看容器和日志：

   ```bash
   docker compose --env-file .env -f docker-compose.yml ps
   docker compose --env-file .env -f docker-compose.yml logs -f tikeo-server
   ```

6. 停止但不删除数据：

   ```bash
   docker compose --env-file .env -f docker-compose.yml stop
   ```

7. 删除容器但保留 named volume：

   ```bash
   docker compose --env-file .env -f docker-compose.yml down
   ```

## PostgreSQL Compose 运行手册

当数据库由同一个 Compose project 内的 PostgreSQL 容器提供时使用这一模式。

1. 准备 `.env` 并设置数据库凭据：

   ```bash
   cp deploy/compose/tikeo.env.example .env
   # 编辑 TIKEO_POSTGRES_DB、TIKEO_POSTGRES_USER、TIKEO_POSTGRES_PASSWORD、
   # TIKEO_POSTGRES_PORT 和 TIKEO_POSTGRES_DATA_VOLUME。
   ```

2. 在 `config/tikeo.yml` 中把 `storage.database` 替换为匹配值：

   ```yaml
   storage:
     database:
       type: postgres
       host: tikeo-postgres
       port: 5432
       username: tikeo
       password: "change-me"
       database: tikeo
       params:
         sslmode: disable
   ```

   对内主机名必须是 `tikeo-postgres`。

3. 校验、拉取并启动：

   ```bash
   docker compose --env-file .env -f docker-compose.postgres.yml config --quiet
   docker compose --env-file .env -f docker-compose.postgres.yml pull
   docker compose --env-file .env -f docker-compose.postgres.yml up -d
   ```

4. 验证 Server ready 和数据库健康：

   ```bash
   docker compose --env-file .env -f docker-compose.postgres.yml ps
   docker compose --env-file .env -f docker-compose.postgres.yml logs --tail=80 tikeo-postgres
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/api/v1/metrics/summary
   ```

5. 备份应面向 `tikeo-postgres-data` 或受控维护容器中的 `pg_dump`。PostgreSQL 模式不要把 Server `/data` volume 当成数据库备份。

## MySQL Compose 运行手册

当数据库由同一个 Compose project 内的 MySQL 容器提供时使用这一模式。

1. 准备 `.env` 并设置数据库凭据：

   ```bash
   cp deploy/compose/tikeo.env.example .env
   # 编辑 TIKEO_MYSQL_DATABASE、TIKEO_MYSQL_USER、TIKEO_MYSQL_PASSWORD、
   # TIKEO_MYSQL_ROOT_PASSWORD、TIKEO_MYSQL_PORT 和 TIKEO_MYSQL_DATA_VOLUME。
   ```

2. 在 `config/tikeo.yml` 中把 `storage.database` 替换为匹配值：

   ```yaml
   storage:
     database:
       type: mysql
       host: tikeo-mysql
       port: 3306
       username: tikeo
       password: "change-me"
       database: tikeo
   ```

   对内主机名必须是 `tikeo-mysql`。

3. 校验、拉取并启动：

   ```bash
   docker compose --env-file .env -f docker-compose.mysql.yml config --quiet
   docker compose --env-file .env -f docker-compose.mysql.yml pull
   docker compose --env-file .env -f docker-compose.mysql.yml up -d
   ```

4. 验证 Server ready 和数据库健康：

   ```bash
   docker compose --env-file .env -f docker-compose.mysql.yml ps
   docker compose --env-file .env -f docker-compose.mysql.yml logs --tail=80 tikeo-mysql
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/api/v1/metrics/summary
   ```

5. 备份应面向 `tikeo-mysql-data` 或受控维护容器中的 `mysqldump`。MySQL 模式不要把 Server `/data` volume 当成数据库备份。

## 为任意 Compose stack 添加 Prometheus

三个主 Compose 文件都通过 `observability` profile 提供 Prometheus。

SQLite stack：

```bash
docker compose --env-file .env -f docker-compose.yml --profile observability up -d tikeo-prometheus
curl -fsS http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/-/ready
```

PostgreSQL stack：

```bash
docker compose --env-file .env -f docker-compose.postgres.yml --profile observability up -d tikeo-prometheus
curl -fsS http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/-/ready
```

MySQL stack：

```bash
docker compose --env-file .env -f docker-compose.mysql.yml --profile observability up -d tikeo-prometheus
curl -fsS http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/-/ready
```

Prometheus 会加载 `prometheus.yml`、`tikeo-recording-rules.yml` 和 `tikeo-alert-rules.yml`。抓取目标是 Compose 网络内的 `tikeo-server:9090`。

## Web nginx、`/api/v1/`、`/api/` 与 SSE

Web 镜像通过 nginx 提供静态资源，并反向代理 API 请求。

- `/api/v1/` 是版本化 Management API 路由。它声明在 `/api/` 前面，并包含 SSE 安全代理配置：HTTP/1.1 upstream、关闭 buffering、关闭 cache、长读写超时、关闭 gzip、设置 `X-Accel-Buffering: no`。
- `/api/` 是非 v1 或未来 API 路径的通用 fallback。它故意不携带更重的长连接流式配置。
- `/api-docs/` 代理 OpenAPI UI 和 JSON。

如果 `tikeo-web` 前面还有外层反向代理，外层代理也必须对 SSE 关闭 buffering。流式验证命令见 [SSE 实时部署](./sse-realtime)。

## Docker run 一览

`docker run` 比 Compose 更手工：需要自己创建网络、选择稳定容器名、挂载同样的文件、保持端口一致。它适合单主机调试，或不能使用 Compose 的环境。

### Docker run + SQLite

```bash
mkdir -p config/tls

docker network create tikeo-net

docker volume create tikeo-data
docker volume create tikeo-logs

docker run -d --name tikeo-server \
  --network tikeo-net \
  --restart unless-stopped \
  -p 9090:9090 \
  -p 9998:9998 \
  -v "$PWD/config/tikeo.yml:/config/tikeo.yml:ro" \
  -v "$PWD/config/tls:/config/tls:ro" \
  -v tikeo-data:/data \
  -v tikeo-logs:/logs \
  -e TZ=Asia/Shanghai \
  -e MIMALLOC_PURGE_DELAY=0 \
  -e MIMALLOC_PURGE_DECOMMITS=1 \
  -e MIMALLOC_ABANDONED_PAGE_PURGE=1 \
  yhyzgn/tikeo-server:latest serve --config /config/tikeo.yml

docker run -d --name tikeo-web \
  --network tikeo-net \
  --restart unless-stopped \
  -p 8080:80 \
  yhyzgn/tikeo-web:latest

curl -fsS http://127.0.0.1:9090/readyz
curl -fsS http://127.0.0.1:8080/
```

因为两个容器都在 `tikeo-net` 网络中，且 Server 容器名为 `tikeo-server`，Web 容器可以按内置 nginx 配置访问 Server。

### Docker run + PostgreSQL

先启动 PostgreSQL，然后把 `config/tikeo.yml` 改成 `type: postgres` 和 `host: tikeo-postgres`，再使用上面的 Server 与 Web 命令。

```bash
docker network create tikeo-net

docker volume create tikeo-postgres-data

docker run -d --name tikeo-postgres \
  --network tikeo-net \
  --restart unless-stopped \
  -p 15432:5432 \
  -v tikeo-postgres-data:/var/lib/postgresql/data \
  -e POSTGRES_DB=tikeo \
  -e POSTGRES_USER=tikeo \
  -e POSTGRES_PASSWORD=change-me \
  postgres:16-alpine
```

随后启动 `tikeo-server`，并确保 `/config/tikeo.yml` 指向 `tikeo-postgres:5432`。

### Docker run + MySQL

先启动 MySQL，然后把 `config/tikeo.yml` 改成 `type: mysql` 和 `host: tikeo-mysql`，再使用上面的 Server 与 Web 命令。

```bash
docker network create tikeo-net

docker volume create tikeo-mysql-data

docker run -d --name tikeo-mysql \
  --network tikeo-net \
  --restart unless-stopped \
  -p 13306:3306 \
  -v tikeo-mysql-data:/var/lib/mysql \
  -e MYSQL_DATABASE=tikeo \
  -e MYSQL_USER=tikeo \
  -e MYSQL_PASSWORD=change-me \
  -e MYSQL_ROOT_PASSWORD=change-root \
  mysql:8.4 \
  --character-set-server=utf8mb4 \
  --collation-server=utf8mb4_0900_ai_ci
```

随后启动 `tikeo-server`，并确保 `/config/tikeo.yml` 指向 `tikeo-mysql:3306`。

### Docker run + Prometheus

```bash
docker run -d --name tikeo-prometheus \
  --network tikeo-net \
  --restart unless-stopped \
  -p 9091:9090 \
  -v "$PWD/observability/prometheus/prometheus.yml:/etc/prometheus/prometheus.yml:ro" \
  -v "$PWD/observability/prometheus/tikeo-recording-rules.yml:/etc/prometheus/tikeo-recording-rules.yml:ro" \
  -v "$PWD/observability/prometheus/tikeo-alert-rules.yml:/etc/prometheus/tikeo-alert-rules.yml:ro" \
  prom/prometheus:latest \
  --config.file=/etc/prometheus/prometheus.yml \
  --web.enable-lifecycle

curl -fsS http://127.0.0.1:9091/-/ready
```

## Worker 连接

Worker 主动连接 Server Worker Tunnel。默认本地部署使用：

```bash
TIKEO_WORKER_TUNNEL_PUBLIC_ENDPOINT=http://127.0.0.1:9998
```

远程 Worker 应把公网 endpoint 配成真正可访问的 Worker Tunnel URL，并且防火墙只开放实际需要的 Server HTTP/Web 端口和 Worker Tunnel 端口。不要暴露任意业务 Worker 端口。

如果 Worker 声明 script runner，需要在 Worker 镜像中预装 sandbox tools，或挂载已准备好的 `TIKEO_SANDBOX_TOOLS_DIR`；见 [Worker sandbox tools and Dockerfiles](./worker-sandbox-tools)。

## 升级与回滚

1. 在 `.env` 中固定镜像：

   ```bash
   TIKEO_IMAGE=yhyzgn/tikeo-server:v0.3.19
   TIKEO_WEB_IMAGE=yhyzgn/tikeo-web:v0.3.19
   ```

2. 拉取并重启选定 stack：

   ```bash
   docker compose --env-file .env -f docker-compose.yml pull
   docker compose --env-file .env -f docker-compose.yml up -d
   ```

3. 删除旧备份产物前先验证业务输出：

   ```bash
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
   curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/api/v1/metrics/summary
   curl -fsS http://127.0.0.1:${TIKEO_WEB_PORT:-8080}/
   ```

回滚时，把两个镜像变量改回上一个可用 tag，再执行同样的 `pull` 和 `up -d`。

## 前置条件

- Compose 示例需要 Docker Engine 和 Docker Compose v2。
- `config/tikeo.yml` 已按目标数据库、TLS、日志、认证和公网控制台 URL 检查。
- 需要的宿主机路径存在：`./config/tikeo.yml`、`./config/tls`，启用 Prometheus 时还包括 `./observability/prometheus/*`。
- 目标备份策略可以覆盖使用中的 named volume。
- `.env` 中宿主机端口没有与现有服务冲突。

## 验收

启动后不要只看容器是否 running，要验证真实服务行为：

```bash
docker compose --env-file .env -f docker-compose.yml ps
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/healthz
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/readyz
curl -fsS http://127.0.0.1:${TIKEO_HTTP_PORT:-9090}/api/v1/metrics/summary
curl -fsS http://127.0.0.1:${TIKEO_WEB_PORT:-8080}/
```

数据库模式还要确认数据库容器 healthy，并确认所选数据库 volume 已纳入备份。

Prometheus 验证 readiness 和 target 状态：

```bash
curl -fsS http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/-/ready
curl -fsS "http://127.0.0.1:${TIKEO_PROMETHEUS_PORT:-9091}/api/v1/targets?state=active"
```

## 故障排查

| 现象 | 检查项 | 处理方式 |
| --- | --- | --- |
| `tikeo-server` 立刻退出 | `docker compose logs tikeo-server` | 确认 `/config/tikeo.yml` 已挂载且 YAML 有效。 |
| PostgreSQL 连接失败 | `storage.database.host` 与 `.env` 凭据 | 使用 `host: tikeo-postgres`；保持 username、password、database 与 `TIKEO_POSTGRES_*` 一致。 |
| MySQL 连接失败 | `storage.database.host` 与 `.env` 凭据 | 使用 `host: tikeo-mysql`；保持 username、password、database 与 `TIKEO_MYSQL_*` 一致。 |
| Web API 返回 502 | nginx upstream 与容器名 | 保持 Server service/container 名为 `tikeo-server`，或同步修改 nginx。 |
| SSE 页面更新延迟或断开 | 外层代理 buffering | 所有代理层都要对 `/api/v1/` 关闭 buffering、cache 和 gzip。 |
| Prometheus 没有 active target | Prometheus target 和网络 | 同一 Docker 网络内 target 必须是 `tikeo-server:9090`。 |
| `container name is already in use` | 旧容器仍存在 | 确认不需要后再执行 `docker rm tikeo-server tikeo-web tikeo-prometheus tikeo-postgres tikeo-mysql`。 |
| 端口已被占用 | `.env` 宿主机端口 | 修改 `TIKEO_HTTP_PORT`、`TIKEO_WEB_PORT`、`TIKEO_WORKER_TUNNEL_PORT` 或数据库宿主机端口。 |

## 生产检查清单

- [ ] `TIKEO_IMAGE` 和 `TIKEO_WEB_IMAGE` 已固定到 release tag。
- [ ] `config/tikeo.yml` 与所选存储模式和公网控制台 URL 一致。
- [ ] 数据库密码已从示例值修改，且没有提交到源码。
- [ ] 实际数据库 volume 或托管数据库已备份，并做过恢复验证。
- [ ] 启用 TLS 或 mTLS 时，`/config/tls` 中证书文件正确。
- [ ] Web 代理和所有外层代理都保留 `/api/v1/` 的 SSE 行为。
- [ ] 启用 Prometheus 时，它抓取 `tikeo-server:9090`，并加载 recording rules 和 alert rules。
- [ ] Worker 可以访问公网 Worker Tunnel endpoint，且不需要入站业务端口。
- [ ] 当前环境的升级和回滚镜像 tag 已记录。
