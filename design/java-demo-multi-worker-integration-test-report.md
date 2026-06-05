# Java Demo 单 Server 多 Worker 联调数据与启动矩阵测试报告

- 报告时间：2026-06-03 13:00 CST
- 范围：本轮新增的开发联调数据脚本、Java demo 多 Worker 启动脚本、Java demo scope 注入能力
- 目标：验证一个 tikee server 下，多个 Java demo worker 可以按不同 namespace/app/worker pool 注册，为后续联合触发任务与观察 Worker 列表/实例日志提供可复用测试资产。

## 1. 测试对象

| 对象 | 路径 | 本轮验证目的 | 状态 |
| --- | --- | --- | --- |
| 联调数据 seed 脚本 | `scripts/dev-integration-seed.sh` | 通过真实 HTTP 管理 API 创建 namespace/app/worker pool/plugin/job | ✅ 通过 |
| 多 demo worker 启动脚本 | `scripts/start-java-demo-workers.sh` | 一键启动/查看/停止 5 个 Java demo worker 实例 | ✅ 通过静态与 status 验证 |
| Spring Boot 2 demo scope 注入 | `examples/java/spring-boot2-worker-demo` | namespace/app/worker_pool 可通过环境变量注入并进入注册信息 | ✅ 通过 |
| Spring Boot 3 demo scope 注入 | `examples/java/spring-boot3-worker-demo` | namespace/app/worker_pool 可通过环境变量注入并进入注册信息 | ✅ 通过 |
| Spring Boot 4 demo scope 注入 | `examples/java/spring-boot4-worker-demo` | namespace/app/worker_pool 可通过环境变量注入并进入注册信息 | ✅ 通过 |
| Java 示例文档 | `examples/java/README.md` | 说明单 server 多 worker 联调命令与矩阵 | ✅ 已补充 |

## 2. 测试边界

### 2.1 本轮覆盖边界

本轮测试覆盖以下内容：

1. **脚本语法与可执行入口**
   - `dev-integration-seed.sh` shell 语法正确。
   - `start-java-demo-workers.sh` shell 语法正确。
   - 3 个 Java demo 的 `scripts/run-demo-worker.sh` shell 语法正确。

2. **真实 API seed 创建链路**
   - 使用临时 SQLite DB 与临时 server 端口验证。
   - 通过 `/healthz` 等待 server 可用。
   - 通过 bootstrap/login 获得 admin token。
   - 通过管理 API 创建：
     - namespace
     - app
     - worker pool
     - worker pool quota
     - API schedule job
   - 验证脚本可在全新空库上从 0 生成联调数据。

3. **Java demo 配置绑定**
   - `tikee.worker.namespace` 支持 `TIKEE_WORKER_NAMESPACE`。
   - `tikee.worker.app` 支持 `TIKEE_WORKER_APP`。
   - `tikee.worker.cluster` 支持 `TIKEE_WORKER_CLUSTER`。
   - `tikee.worker.region` 支持 `TIKEE_WORKER_REGION`。
   - worker labels 增加 `worker_pool: ${TIKEE_WORKER_POOL:default}`。
   - demo run script 会把这些环境变量传入 Gradle run/bootRun 进程。

4. **Java demo 单元/上下文测试**
   - Spring Boot 2/3/4 demo 均执行 `./gradlew test --no-daemon`。
   - 测试包含 dry-run worker 注册信息断言。
   - 新增断言覆盖 `worker_pool=demo-pool` 标签。

5. **启动矩阵管理能力**
   - `scripts/start-java-demo-workers.sh --status` 可以列出 5 个目标 worker 的 scope、端口、日志路径和运行状态。
   - 脚本提供 `--detach`、`--status`、`--stop` 操作入口。

### 2.2 联合冒烟边界复跑结果

以下内容为本轮复跑后的完整端到端验证结果、仍保留的专项边界和后续建议：

| 边界项 | 本轮复跑结果 | 证据 | 状态 |
| --- | --- | --- | --- |
| 同时真实启动 5 个 Java demo worker 并完成 server worker list 注册断言 | 已启动 5 个 worker；`/api/v1/workers` 返回 `online=5`，5 个 logical instance 分属 `dev-alpha/orders`、`dev-alpha/billing`、`dev-beta/analytics`、`dev-ops/automation`；worker pool 通过 master domain 体现为 `boot2-blue`、`boot3-blue`、`boot4-green`、`boot3-batch`、`boot4-ops` | `/home/neo/Projects/neo/pub/tikee/.dev/reports/java-multi-worker-20260603T044644Z-205593/workers-final.json`、`/home/neo/Projects/neo/pub/tikee/.dev/reports/java-multi-worker-20260603T044644Z-205593/worker-process-status.txt`、`/home/neo/Projects/neo/pub/tikee/.dev/reports/java-multi-worker-20260603T044644Z-205593/health-18182.json` ~ `health-18186.json` | ✅ 通过 |
| 手动触发 seed 生成的 8 个 API jobs 并等待实例完成 | 已触发 8 个 seed jobs；7 个业务 processor succeeded，`fail-api` 走预期失败路径；实例日志均已采集 | `/home/neo/Projects/neo/pub/tikee/.dev/reports/java-multi-worker-20260603T044644Z-205593/final-e2e-summary.json`、`/home/neo/Projects/neo/pub/tikee/.dev/reports/java-multi-worker-20260603T044644Z-205593/trigger-results.json`、`/home/neo/Projects/neo/pub/tikee/.dev/reports/java-multi-worker-20260603T044644Z-205593/retrigger-sql-result.json` | ✅ 通过 |
| SQL plugin job 调度绑定 | 首次触发暴露 seed 缺口：`sql-sync-api` 未带 `processorType=sql` 时被当作 SDK processor 匹配并 fail-closed；已修 `scripts/dev-integration-seed.sh` 注册 SQL plugin 并创建 `processorType=sql` job，PATCH 当前临时 DB 后重触发成功 | `/home/neo/Projects/neo/pub/tikee/.dev/reports/java-multi-worker-20260603T044644Z-205593/sql-job-patch.json`、`/home/neo/Projects/neo/pub/tikee/.dev/reports/java-multi-worker-20260603T044644Z-205593/retrigger-sql-result.json` | ✅ 已修复并通过 |
| worker pool quota 对调度拥塞/并发限制的压力测试 | 已补存储层专项压力回归：`max_concurrency=1` 会阻止同池第二个 running claim；`max_queue_depth=1` 会在 active depth 超限时背压并在深度下降后恢复；拥塞池不会饿死后续开放池 | `cargo test -p tikee-storage worker_pool_ --all-features` | ✅ 通过 |
| 非 SQLite 数据库上的 seed API 验证 | 已使用 PostgreSQL 16 与 MySQL 8.4 compose 服务分别启动 tikee server，并通过真实 HTTP API 跑 `scripts/dev-integration-seed.sh`；两端均创建 3 namespaces、4 apps、5 worker pools、SQL plugin、8 jobs | `/home/neo/Projects/neo/pub/tikee/.dev/reports/db-seed-compat-20260603T060134Z/postgres-seed.log`、`/home/neo/Projects/neo/pub/tikee/.dev/reports/db-seed-compat-20260603T060134Z/postgres-jobs.json`、`/home/neo/Projects/neo/pub/tikee/.dev/reports/db-seed-compat-20260603T060134Z/postgres-worker-pools.json`、`/home/neo/Projects/neo/pub/tikee/.dev/reports/db-seed-compat-20260603T060134Z/postgres-plugins.json`、`/home/neo/Projects/neo/pub/tikee/.dev/reports/db-seed-compat-20260603T060134Z/mysql-seed.log`、`/home/neo/Projects/neo/pub/tikee/.dev/reports/db-seed-compat-20260603T060134Z/mysql-jobs.json`、`/home/neo/Projects/neo/pub/tikee/.dev/reports/db-seed-compat-20260603T060134Z/mysql-worker-pools.json`、`/home/neo/Projects/neo/pub/tikee/.dev/reports/db-seed-compat-20260603T060134Z/mysql-plugins.json` | ✅ 通过 |

## 3. 联调数据设计

### 3.1 Namespace/App/Worker Pool 矩阵

| Namespace | App | Worker Pool | 计划 Worker | 端口 | 说明 |
| --- | --- | --- | --- | --- | --- |
| `dev-alpha` | `orders` | `boot2-blue` | `java-boot2-orders-blue` | `18182` | Spring Boot 2 demo，订单域 blue pool |
| `dev-alpha` | `orders` | `boot3-blue` | `java-boot3-orders-blue` | `18183` | Spring Boot 3 demo，订单域 blue pool |
| `dev-alpha` | `billing` | `boot4-green` | `java-boot4-billing-green` | `18184` | Spring Boot 4 demo，账单域 green pool |
| `dev-beta` | `analytics` | `boot3-batch` | `java-boot3-analytics-batch` | `18185` | Spring Boot 3 demo，分析批处理 pool |
| `dev-ops` | `automation` | `boot4-ops` | `java-boot4-ops` | `18186` | Spring Boot 4 demo，运维自动化 pool |

### 3.2 Seed Job 矩阵

| Namespace | App | Job | Processor | Schedule Type | 状态 |
| --- | --- | --- | --- | --- | --- |
| `dev-alpha` | `orders` | `echo-api` | `demo.echo` | `api` | ✅ 创建验证通过 |
| `dev-alpha` | `orders` | `context-api` | `demo.context` | `api` | ✅ 创建验证通过 |
| `dev-alpha` | `orders` | `bytes-api` | `demo.bytes` | `api` | ✅ 创建验证通过 |
| `dev-alpha` | `billing` | `report-api` | `demo.report` | `api` | ✅ 创建验证通过 |
| `dev-alpha` | `billing` | `sql-sync-api` | `billing.sql-sync` | `api` | ✅ 创建验证通过 |
| `dev-beta` | `analytics` | `workflow-step-api` | `demo.workflow.step` | `api` | ✅ 创建验证通过 |
| `dev-beta` | `analytics` | `heartbeat-api` | `demo.heartbeat` | `api` | ✅ 创建验证通过 |
| `dev-ops` | `automation` | `fail-api` | `demo.fail` | `api` | ✅ 创建验证通过 |


### 3.3 完整联合冒烟结果（2026-06-03 13:00 CST）

本轮复跑使用临时 SQLite 配置启动 server，避免污染已初始化 `tikee-dev.db`：

- 运行目录：`/home/neo/Projects/neo/pub/tikee/.dev/reports/java-multi-worker-20260603T044644Z-205593`
- Server：`http://127.0.0.1:9090`，`/healthz` 通过
- Worker tunnel：`127.0.0.1:9998`
- Worker 矩阵：5/5 本地健康端点通过，server worker list `online=5`
- Job 触发：8 个 seed API jobs 均进入终态；7 个 succeeded，`fail-api` 为 demo 预期失败

| Job | Processor | 终态 | 说明 |
| --- | --- | --- | --- |
| `echo-api` | `demo.echo` | ✅ succeeded | Boot2 orders worker 执行 |
| `context-api` | `demo.context` | ✅ succeeded | Boot2 orders worker 执行 |
| `bytes-api` | `demo.bytes` | ✅ succeeded | Boot2 orders worker 执行 |
| `report-api` | `demo.report` | ✅ succeeded | Boot4 billing worker 执行 |
| `sql-sync-api` | `billing.sql-sync` / `processorType=sql` | ✅ succeeded | 首次触发暴露 seed 未写 `processorType=sql`；修复后重触发成功 |
| `workflow-step-api` | `demo.workflow.step` | ✅ succeeded | Boot3 analytics worker 执行 |
| `heartbeat-api` | `demo.heartbeat` | ✅ succeeded | Boot3 analytics worker 执行 |
| `fail-api` | `demo.fail` | ✅ expected failed | Boot4 ops worker 执行，验证失败路径可观测 |

## 4. 测试环境

| 项 | 值 |
| --- | --- |
| 仓库路径 | `/home/neo/Projects/neo/pub/tikee` |
| 操作系统上下文 | Fedora/Linux 本地开发机 |
| Java demo 构建工具 | Gradle wrapper |
| 完整联合冒烟 server | `target/debug/tikee serve --config /home/neo/Projects/neo/pub/tikee/.dev/reports/java-multi-worker-20260603T044644Z-205593/dev-e2e.toml` |
| 完整联合冒烟 HTTP | `http://127.0.0.1:9090` |
| 完整联合冒烟 worker tunnel | `127.0.0.1:9998` |
| 完整联合冒烟数据库 | `sqlite:///home/neo/Projects/neo/pub/tikee/.dev/reports/java-multi-worker-20260603T044644Z-205593/tikee-e2e.db?mode=rwc` |
| 常规 dev HTTP 默认值 | `http://127.0.0.1:9090` |
| 常规 worker tunnel 默认值 | `http://127.0.0.1:9998` |

## 5. 执行记录与结果

### 5.1 Shell 语法检查

命令：

```bash
bash -n \
  scripts/dev-integration-seed.sh \
  scripts/start-java-demo-workers.sh \
  examples/java/spring-boot2-worker-demo/scripts/run-demo-worker.sh \
  examples/java/spring-boot3-worker-demo/scripts/run-demo-worker.sh \
  examples/java/spring-boot4-worker-demo/scripts/run-demo-worker.sh
```

结果：

| 检查项 | 结果 |
| --- | --- |
| `scripts/dev-integration-seed.sh` | ✅ 通过 |
| `scripts/start-java-demo-workers.sh` | ✅ 通过 |
| Spring Boot 2 `run-demo-worker.sh` | ✅ 通过 |
| Spring Boot 3 `run-demo-worker.sh` | ✅ 通过 |
| Spring Boot 4 `run-demo-worker.sh` | ✅ 通过 |

### 5.2 启动矩阵 status 验证

命令：

```bash
scripts/start-java-demo-workers.sh --status
```

实际输出摘要：

```text
java-boot2-orders-blue       stopped  port=18182  dev-alpha/orders/boot2-blue
java-boot3-orders-blue       stopped  port=18183  dev-alpha/orders/boot3-blue
java-boot4-billing-green     stopped  port=18184  dev-alpha/billing/boot4-green
java-boot3-analytics-batch   stopped  port=18185  dev-beta/analytics/boot3-batch
java-boot4-ops               stopped  port=18186  dev-ops/automation/boot4-ops
```

结果：✅ 通过。

说明：本检查验证了脚本内置矩阵、PID/log 路径渲染和 status 控制流。未实际启动 5 个 demo worker。

### 5.3 Spring Boot 2 Demo 测试

命令：

```bash
cd examples/java/spring-boot2-worker-demo
./gradlew test --no-daemon
```

结果：

```text
BUILD SUCCESSFUL in 2m 2s
16 actionable tasks: 3 executed, 13 up-to-date
```

状态：✅ 通过。

覆盖点：

- Spring Boot 2 demo context 可启动。
- dry-run worker client 正常启动。
- namespace/app/cluster/region 注册信息仍正确。
- capabilities 包含 `java`, `spring-boot`。
- labels 包含：
  - `worker_pool=demo-pool`
  - `runtime=java`
  - `demo=spring-boot2-worker-demo`
- processor registry 包含 Java demo 处理器和 `billing.sql-sync` plugin processor。

### 5.4 Spring Boot 3 Demo 测试

命令：

```bash
cd examples/java/spring-boot3-worker-demo
./gradlew test --no-daemon
```

结果：

```text
BUILD SUCCESSFUL in 2m 15s
16 actionable tasks: 3 executed, 13 up-to-date
```

状态：✅ 通过。

覆盖点：

- Spring Boot 3 demo context 可启动。
- dry-run worker client 正常启动。
- labels 包含 `worker_pool=demo-pool`。
- demo endpoints 和 processor registry 测试通过。

### 5.5 Spring Boot 4 Demo 测试

命令：

```bash
cd examples/java/spring-boot4-worker-demo
./gradlew test --no-daemon
```

结果：

```text
BUILD SUCCESSFUL in 2m 14s
16 actionable tasks: 3 executed, 13 up-to-date
```

状态：✅ 通过。

覆盖点：

- Spring Boot 4 demo context 可启动。
- dry-run worker client 正常启动。
- labels 包含 `worker_pool=demo-pool`。
- demo endpoints 和 processor registry 测试通过。

### 5.6 真实 API Seed 验证

#### 5.6.1 临时 server 启动

临时配置文件：`.dev/verify-integration-seed.toml`

关键配置：

```toml
[server]
listen_addr = "127.0.0.1:19090"
worker_tunnel_addr = "127.0.0.1:19998"

[storage]
database_url = "sqlite://.dev/verify-integration-seed.db?mode=rwc"
timestamp_offset = "+08:00"
```

健康检查结果：

```text
temp-server-ready
```

状态：✅ 通过。

#### 5.6.2 Seed 脚本执行

命令：

```bash
TIKEE_HTTP_URL=http://127.0.0.1:19090 scripts/dev-integration-seed.sh
```

关键输出：

```text
✅ authenticated as smoke_admin
✅ namespace created: dev-alpha
✅ namespace created: dev-beta
✅ namespace created: dev-ops
✅ app created: dev-alpha/orders
✅ app created: dev-alpha/billing
✅ app created: dev-beta/analytics
✅ app created: dev-ops/automation
✅ worker pool created: dev-alpha/orders/boot2-blue queue=200 concurrency=8
✅ worker pool created: dev-alpha/orders/boot3-blue queue=200 concurrency=8
✅ worker pool created: dev-alpha/billing/boot4-green queue=100 concurrency=4
✅ worker pool created: dev-beta/analytics/boot3-batch queue=150 concurrency=6
✅ worker pool created: dev-ops/automation/boot4-ops queue=80 concurrency=3
✅ job created: dev-alpha/orders/echo-api -> demo.echo
✅ job created: dev-alpha/orders/context-api -> demo.context
✅ job created: dev-alpha/orders/bytes-api -> demo.bytes
✅ job created: dev-alpha/billing/report-api -> demo.report
✅ job created: dev-alpha/billing/sql-sync-api -> billing.sql-sync
✅ job created: dev-beta/analytics/workflow-step-api -> demo.workflow.step
✅ job created: dev-beta/analytics/heartbeat-api -> demo.heartbeat
✅ job created: dev-ops/automation/fail-api -> demo.fail
```

状态：✅ 通过。

说明：该验证证明 seed 脚本不是直接写库，而是通过真实 HTTP 管理 API 完成创建。

### 5.7 已初始化 dev DB 登录边界验证

尝试在本机已有 dev DB 上使用默认 smoke admin：

```bash
scripts/dev-integration-seed.sh
```

观察结果：

```text
curl: (22) The requested URL returned error: 401
```

结论：⚠️ 环境相关，不是 seed 数据创建逻辑失败。

原因：本机 dev DB 已初始化，默认 `smoke_admin/Tikee@2026!` 不一定存在或密码已变更。脚本已补充以下替代认证方式：

```bash
TIKEE_SMOKE_AUTH_TOKEN=<admin bearer token> scripts/dev-integration-seed.sh
# 或
TIKEE_ADMIN_TOKEN=<admin bearer token> scripts/dev-integration-seed.sh
# 或
TIKEE_ADMIN_USERNAME=<admin> TIKEE_ADMIN_PASSWORD=<password> scripts/dev-integration-seed.sh
```

## 6. 验证结论

| 结论项 | 结果 |
| --- | --- |
| 是否具备创建开发联调 namespace/app/worker pool 数据的脚本资产 | ✅ 是 |
| seed 是否通过真实管理 API 验证 | ✅ 是 |
| 是否具备一键启动多 Java demo worker 的脚本资产 | ✅ 是 |
| 3 个 Java demo 是否支持注入不同 namespace/app/worker_pool | ✅ 是 |
| 3 个 Java demo 单元/上下文测试是否通过 | ✅ 是 |
| 是否已完成完整 server + 5 worker + trigger job 的端到端联合验证 | ✅ 是，5 worker 在线，8 jobs 触发闭环完成（7 succeeded + 1 expected failed） |

## 7. 推荐后续联合冒烟流程

在开发机上做完整联调时，建议按以下顺序执行：

```bash
# 1. 启动 server + web
./scripts/dev.sh

# 2. 如果当前 dev DB 默认账号不可用，先从 Web 登录后复制 admin token，或使用已有 admin 账号
TIKEE_SMOKE_AUTH_TOKEN=<admin bearer token> scripts/dev-integration-seed.sh

# 3. 启动多 Java demo worker
scripts/start-java-demo-workers.sh --detach

# 4. 查看 worker 矩阵状态
scripts/start-java-demo-workers.sh --status

# 5. 在 Web 的 worker 列表确认 5 个 worker 在线，并检查 worker_pool 绑定

# 6. 手动触发 seed 创建的 API jobs，观察 demo 控制台日志与实例日志

# 7. 结束后停止 demo worker
scripts/start-java-demo-workers.sh --stop
```

## 8. 风险与建议

1. **已初始化 dev DB 默认账号不作为验收项**
   - 本轮完整验证使用临时库 bootstrap 管理员完成；已初始化 dev DB 中默认账号可能不存在或密码不同，这是环境状态而非功能缺口。
   - `scripts/dev-integration-seed.sh` 支持 `TIKEE_SMOKE_AUTH_TOKEN` / `TIKEE_ADMIN_TOKEN` / 显式账号密码，实际 dev DB 使用时应传入当前有效管理员凭据。

2. **完整联合测试已在临时 SQLite 环境跑通，但建议沉淀为自动化脚本**
   - 本轮已启动 5 个 demo worker 并完成 `seed -> start workers -> trigger -> wait -> collect logs` 闭环。
   - 后续建议把本次手工编排固化为一键 smoke 脚本，并保留足够长的 Spring 首启等待窗口。

3. **worker_pool 当前通过 label 暴露**
   - server worker 列表当前接受 `worker_pool`/`worker-pool` label 识别 worker pool。
   - 本轮 Java demo 已统一写入 `worker_pool` label。

4. **quota 拥塞/并发限制已补专项回归**
   - worker pool quota API 写入通过。
   - 存储层 dispatch queue claim 已覆盖 `max_concurrency`、`max_queue_depth` 与拥塞池跳过，避免同池超并发和跨池饥饿。

## 9. 最终状态

本轮开发联调数据和多 Java demo worker 启动矩阵达到“可落地执行的测试资产”状态：

- ✅ 脚本可执行
- ✅ Java demo 测试通过
- ✅ seed 真实 API 创建通过（含 SQL plugin processor 注册）
- ✅ 文档已补充
- ✅ 完整多 worker 在线触发闭环已执行并采集证据
- ✅ worker pool quota 拥塞/并发限制专项回归通过
- ✅ PostgreSQL/MySQL seed API 兼容专项通过

## 10. Worker Pool quota 拥塞/并发限制专项结果

本专项补充了直接作用于 `DispatchQueueRepository::claim_next_job_queue_item*` 的回归测试，验证调度队列在 worker pool quota 下的背压语义：

| 场景 | 断言 | 结果 |
| --- | --- | --- |
| `max_concurrency=1` | 同一 namespace/app/worker_pool 已有 running queue item 时，第二个 pending item 不会被 claim | ✅ 通过 |
| `max_queue_depth=1` | 同池 active depth 超限时 pending item 被背压；关闭一个 active item 后剩余 item 可继续 claim | ✅ 通过 |
| 拥塞池跳过 | 前面 20 个 queue items 属于超限池时，后续开放池 item 仍可被 claim，避免跨池饥饿 | ✅ 通过 |

修复点：

1. worker pool quota 查询从“仅按 pool name 查找”改为按 `namespace + app + worker_pool` 解析，避免不同 app/namespace 同名 pool 互相影响。
2. claim 扫描不再只看第一个候选，允许跳过被 quota 背压的拥塞池，继续寻找后续可运行池。
3. 保留 `max_queue_depth` 的可消化语义：active depth **超过**上限时背压；降回上限后允许继续 claim，避免队列无法自行恢复。

验证命令：

```bash
cargo test -p tikee-storage worker_pool_ --all-features
```

## 11. 非 SQLite seed API 兼容专项结果

本专项使用 `deploy/compose/database-compat-compose.yml` 启动 PostgreSQL 16 与 MySQL 8.4，分别以对应 DSN 启动 tikee server，并通过真实 HTTP 管理 API 执行 `scripts/dev-integration-seed.sh`。

- 运行目录：`/home/neo/Projects/neo/pub/tikee/.dev/reports/db-seed-compat-20260603T060134Z`
- PostgreSQL API：`http://127.0.0.1:19090`
- MySQL API：`http://127.0.0.1:19091`
- 验证资产：`scripts/db-seed-api-compat-smoke.sh`

| 数据库 | 验证内容 | 结果 |
| --- | --- | --- |
| PostgreSQL 16 | bootstrap/login、namespace/app/worker pool/quota、SQL plugin、8 个 seed jobs | ✅ 通过 |
| MySQL 8.4 | bootstrap/login、namespace/app/worker pool/quota、SQL plugin、8 个 seed jobs | ✅ 通过 |

验证命令：

```bash
scripts/db-seed-api-compat-smoke.sh
```

本轮手动执行证据包含：`postgres-seed.log`、`postgres-jobs.json`、`postgres-worker-pools.json`、`postgres-plugins.json`、`mysql-seed.log`、`mysql-jobs.json`、`mysql-worker-pools.json`、`mysql-plugins.json`。

## 12. 2026-06-04 跨语言 Worker parity 与持久化可见性补充验证

本节记录 2026-06-04 在 Java 多 Worker 联调基线之后追加完成的 Go/Rust SDK 与 demo 对齐、Worker 可见性持久化、Web Worker 集群视图重组和 CI 验证结果。该状态是当前手动抽查验收的最新基线。

### 12.1 代码与数据状态

| 项 | 当前状态 | 说明 |
| --- | --- | --- |
| Worker 可见性持久化 | ✅ 已完成 | `worker_sessions` 保存 `capabilities_json`、`structured_capabilities_json`、`labels_json`、`master_json`；`/api/v1/workers` 合并 live registry 与 DB online sessions，避免 server 重启后 Worker 列表完全依赖内存。 |
| Worker master/follower 观测 | ✅ 已完成 | master state 以 camelCase 序列化，Web Worker 页面按 namespace/app 与 cluster/region 分组展示 node。 |
| 调度队列 UI | ✅ 已拆分 | 调度队列从 Worker 集群主页面移到 `/workers/dispatch-queue` 二级页。 |
| i18n label | ✅ 已修正 | 中文显示“处理器”，英文显示 `Processor`，不再混合显示中英混排标签。 |
| Go SDK/demo | ✅ 已对齐 | 默认 live 连接；官方 gRPC/protobuf；结构化 scope/processor/capability；assignment token 任务日志；重连循环；README 说明 protoc 与 Dockerfile 安装。 |
| Rust SDK/demo | ✅ 已对齐 | 默认 live 连接；结构化 scope/processor/capability；assignment token 任务日志；重连循环；script runners 对齐 Java sandbox 名称。 |
| dev DB | ✅ 已同步 | `tikee-dev.db` 已包含 Java/Go/Rust 手动验收数据和 script/job 用例。 |

### 12.2 Go/Rust live 验收证据

| 语言 | 触发 Job | 选择器 | 实例日志断言 | 结果 |
| --- | --- | --- | --- | --- |
| Go | `job_go_demo_orders_echo` | structured tag `go` | `received task ... processor=demo.echo`；`completed task ... success=true message=go demo echo processed` | ✅ 通过 |
| Rust | `job_rust_demo_orders_echo` | structured tag `rust` | `received task ... processor=demo.echo`；`completed task ... success=true message=` | ✅ 通过 |

关键修复：Go/Rust SDK 日志上报必须带 `assignment_token`，否则服务端按当前安全规则会丢弃日志；现在 Go/Rust 与 Java 一样会在任务开始/完成时自动上报实例日志。

### 12.3 本地验证命令

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features -- --test-threads=1
cargo build --workspace --all-features
cd web && bun install --frozen-lockfile && bun run lint && bun run typecheck && bun test && bun run build
cd sdks/java && ./gradlew test jar sourcesJar
cd sdks/go/tikee && go test ./...
cd examples/go/worker-demo && go test ./...
cd sdks/rust/tikee && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features && cargo package --allow-dirty
cd examples/rust/worker-demo && cargo test
```

### 12.4 GitHub Actions CI

- CI run：`26947829951`
- 结果：✅ success
- 覆盖：Server / Rust workspace、Web / React、Java SDK、Rust SDK、Docker build validation。

### 12.5 后续必须自动化的剩余项

1. 将当前 Go/Rust/Java 手动联调流程收敛成 executable cross-language worker integration harness。
2. 增加 server restart persistence smoke：Worker 注册后重启 server，验证 `/api/v1/workers` 能先从 DB snapshot 展示，再由 live registry 覆盖。
3. 增加 Go/Rust 任务实例日志 API smoke，防止 assignment token 日志链路回退。
4. 增加 persisted worker 的 worker_pool scope filtering 回归，禁止任何约定命名式匹配回流。

## 13. 2026-06-05 反伪实现审计与跨语言自动化闭环

本轮按生产级实现约束复查 `server / web / sdks / examples demo`，重点排查伪代码、假能力广告、硬编码假数据、仅内存关键状态和 mock-only 路径被当作完成能力的问题。除明确标注为未来 SDK 的 Python/Node demo placeholder 外，发现项已直接改成可验证实现。

### 13.1 已修复的生产级问题

| 范围 | 问题 | 修复结果 | 验证 |
| --- | --- | --- | --- |
| Server scheduling | fixed-rate/daily interval 的自动调度 cursor 只在内存中，重启后有重复触发风险 | 新增 `schedule_cursors` 持久化表、entity、repository 与 SQLite 兼容迁移；tick loop 按 `(job_id, trigger_type, fire_at)` 幂等创建 pending instance | `cargo test -p tikee-storage`、`cargo test -p tikee-server` |
| Raft apply | 未知 `command_type` 以 deferred/unsupported 记录，容易误解为后续会自动补业务 apply | 未知或非法 payload 统一记录为 `rejected`，错误信息明确 `unsupported raft command_type` | `raft_apply_committed_entries_rejects_unknown_and_invalid_payloads` |
| Java/Go/Rust script capability | Go/Rust/Java starter 可能把不可执行沙箱当成 worker capability 广告，导致 UI/调度误以为可运行 | `Unavailable/UnsupportedScriptRunner` 保留 fail-closed handler，但不进入 structured `scriptRunners`；容器 runner 只广告真实可执行 backend canonical name | Java/Go/Rust SDK tests + cross-language smoke |
| Rust SDK outcome | Rust demo 返回成功消息前 SDK outcome 不支持 operator-facing success message | `TaskOutcome::Success(String)` 与 `is_success()` 已实现，session result/log 使用真实消息 | Rust SDK tests、cross-language smoke |
| Web i18n | 英文字典存在机械拼接坏翻译，中文 locale 对若干英文 label 未完全本地化 | 基础字典与 override 同步清理；新增质量门，禁止空英文、中文混入英文和典型机械拼接模式 | `cd web && bun run typecheck && bun test --run src/i18n/i18n.test.ts` |

### 13.2 跨语言自动化 harness 已落地

新增 `deploy/smoke/cross-language-worker-parity-smoke.sh`，将原先手动 Java/Go/Rust worker parity 验收升级为一键 smoke：

1. 启动隔离 tikee server、Web、Java Boot2/Boot3/Boot4、Go、Rust demo worker。
2. 通过真实 HTTP API 创建 namespace/app/worker pool/quota、SQL plugin processor 和 jobs。
3. 断言五类 worker 均在线，scope、worker_pool、processor、tags、master state 均来自结构化字段。
4. 触发 Java/Go/Rust echo jobs，断言实例终态 succeeded 且实例日志包含真实处理消息。
5. 重启 server 并停止 worker，断言 `/api/v1/workers` 仍能从持久化 `worker_sessions` snapshot 展示 worker、structuredCapabilities、labels、master。
6. 重连 worker 后断言 live registry 覆盖 snapshot。
7. 使用 worker_pool scoped API-Key 同时验证 live 与 persisted snapshot 的过滤结果一致，禁止回退到命名约定匹配。
8. Web route smoke 覆盖 `/workers` 和 `/workers/dispatch-queue` SPA shell。

本轮证据：

- 报告：`.dev/reports/cross-language-workers-20260605T032108Z-202626/cross-language-workers-20260605T032108Z-202626.json`
- 关键产物：`workers-initial.json`、`workers-before-restart.json`、`workers-after-restart-snapshot.json`、`workers-after-reconnect.json`、`worker-pool-filter-live.json`、`worker-pool-filter-persisted.json`、`go-logs.json`、`rust-logs.json`、`web-workers.html`、`web-dispatch-queue.html`

### 13.3 本轮验证命令

```bash
cargo test -p tikee-storage -- --nocapture
cargo test -p tikee-server -- --nocapture
cd sdks/go/tikee && go test ./... -count=1
cd examples/go/worker-demo && go test ./... -count=1
cd sdks/rust/tikee && cargo test --all-features -- --nocapture
cd examples/rust/worker-demo && cargo test -- --nocapture
cd sdks/java && ./gradlew test --no-daemon
cd web && bun run typecheck && bun test --run src/i18n/i18n.test.ts
bash -n deploy/smoke/cross-language-worker-parity-smoke.sh
deploy/smoke/cross-language-worker-parity-smoke.sh
```

### 13.4 审计结论

- 当前 server/web/sdks/demo 范围内，未发现仍被当作已完成功能的伪代码、硬编码假实现或 mock-only 路径。
- Python/Node demo README 中的 placeholder 属于用户已明确允许的未来 SDK 范围，不计入当前完成能力。
- 测试中的 fake/mock/noop 名称均限定在测试桩或协议 noop 场景，不作为生产能力暴露。
- Go/Rust 默认不广告脚本 runner；只有配置了真实可执行 backend 或显式开发 local runner 时才会进入 structured capabilities。
