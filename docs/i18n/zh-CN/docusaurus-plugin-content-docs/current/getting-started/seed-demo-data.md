---
title: 准备演示数据
description: 用可复现的本地命令准备 Tikeo 演示数据。
---

# 准备演示数据

演示数据用于验证调度链路，不用于把控制台“填满”。一套合格的本地演示至少要能说明：namespace/app 隔离、Worker capability 匹配、API 触发、实例日志、审计记录，以及失败时的可见原因。

## 前置条件

从仓库根目录执行命令。先确认基础工具可用：

```bash
cargo --version
python3 --version
curl --version
```

如果要运行 Web 或 Node.js demo，再确认 Bun：

```bash
bun --version
```

本页默认使用本地 HTTP API：

```bash
export TIKEO_HTTP_URL=http://127.0.0.1:9090
```

启动 Server：

```bash
cargo run --bin tikeo -- serve --config config/dev.yml
```

另开一个终端确认健康状态：

```bash
curl -fsS http://127.0.0.1:9090/healthz
curl -fsS http://127.0.0.1:9090/readyz
```

`config/dev.yml` 默认使用 SQLite：`sqlite://.dev/tikeo-dev.db?mode=rwc`。首次启动后需要在 Web 控制台或 bootstrap API 创建 Owner；后续 API 示例需要带本地登录得到的 Bearer token，或使用已有管理 token。

```bash
export TIKEO_TOKEN='<local-admin-token>'
```

不要把真实 token 写进文档、截图、issue 或脚本提交。

## 推荐方式一：通过管理 API 准备最小演示集

这条路径最适合手工演示和培训。它使用 Tikeo 的正式 API，不直接改数据库。

### 1. 创建 namespace、app 和 worker pool

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/namespaces \
  -H "Authorization: Bearer ${TIKEO_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{"name":"demo"}'

curl -fsS -X POST http://127.0.0.1:9090/api/v1/apps \
  -H "Authorization: Bearer ${TIKEO_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{"namespace":"demo","name":"orders"}'

curl -fsS -X POST http://127.0.0.1:9090/api/v1/worker-pools \
  -H "Authorization: Bearer ${TIKEO_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{"namespace":"demo","app":"orders","name":"blue"}'
```

如果 worker pool 已存在，创建请求可能返回重复记录错误；演示时可以换一个名称，或在控制台复用已有 pool。

### 2. 创建 API 触发 Job

`POST /api/v1/jobs` 的常用字段包括 `namespace`、`app`、`name`、`scheduleType`、`processorName`、`processorType`、`scriptId` 和 `enabled`。`processorName` 与 `scriptId` 不能同时用于同一个 Job。

```bash
curl -fsS -X POST http://127.0.0.1:9090/api/v1/jobs \
  -H "Authorization: Bearer ${TIKEO_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{
    "namespace": "demo",
    "app": "orders",
    "name": "demo-echo",
    "scheduleType": "api",
    "processorName": "demo.echo",
    "enabled": true
  }'
```

从响应的 `data.id` 记录 Job ID：

```bash
export TIKEO_DEMO_JOB_ID='<job-id-from-response>'
```

### 3. 触发 Job

```bash
curl -fsS -X POST "http://127.0.0.1:9090/api/v1/jobs/${TIKEO_DEMO_JOB_ID}:trigger" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{"triggerType":"api","executionMode":"single"}'
```

响应会返回实例 ID。保存它用于验收：

```bash
export TIKEO_DEMO_INSTANCE_ID='<instance-id-from-response>'
```

### 4. 查看实例、日志和 attempts

```bash
curl -fsS "http://127.0.0.1:9090/api/v1/instances/${TIKEO_DEMO_INSTANCE_ID}" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"

curl -fsS "http://127.0.0.1:9090/api/v1/instances/${TIKEO_DEMO_INSTANCE_ID}/logs" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"

curl -fsS "http://127.0.0.1:9090/api/v1/instances/${TIKEO_DEMO_INSTANCE_ID}/attempts" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"
```

## 推荐方式二：使用联调脚本批量准备数据

仓库内已有脚本 `scripts/dev-integration-seed.sh`，会通过 HTTP API 创建一组联调用 namespace、app、worker pool、plugin processor 和 Job。它适合需要运行 Java demo workers 或多语言 smoke 的场景。

```bash
export TIKEO_HTTP_URL=http://127.0.0.1:9090
export TIKEO_ADMIN_USERNAME='<local-admin-username>'
export TIKEO_ADMIN_PASSWORD='<local-admin-password>'

scripts/dev-integration-seed.sh
```

脚本会等待 `${TIKEO_HTTP_URL}/healthz`，然后创建这些演示对象：

| 类型 | 示例值 |
|---|---|
| namespace | `dev-alpha`、`dev-beta`、`dev-ops` |
| app | `orders`、`billing`、`analytics`、`automation` |
| worker pool | `dev-alpha/orders/{boot2-blue,boot3-blue,go-blue,rust-blue,python-blue,nodejs-blue}`、`dev-alpha/billing/boot4-green`、`dev-beta/analytics/boot3-batch`、`dev-ops/automation/boot4-ops` |
| processor | `demo.echo`、`demo.context`、`demo.bytes`、`demo.report`、`billing.sql-sync`、`demo.workflow.step`、`demo.heartbeat`、`demo.fail` |
| Job | `echo-api`、`context-api`、`bytes-api`、`report-api`、`sql-sync-api`、`workflow-step-api`、`heartbeat-api`、`fail-api` |

脚本完成后可以继续运行：

```bash
scripts/start-java-demo-workers.sh
```

## 可选方式：SQLite 本地快照脚本

`scripts/dev-seed.sh` 会把 `scripts/dev-seed.sql` 写入本地 SQLite 数据库，并输出 namespace、app、worker pool、job、script、workflow、queue 的行数。直接 SQL seed 现在与 API seed 和各语言 demo 默认值使用同一套拓扑；按默认配置启动的 Worker 可以匹配 `dev-alpha/orders` 下的演示 Job。它只适合一次性本地展示或开发排查，不适合生产、共享环境或对审计链路有要求的验收。脚本默认不会覆盖已有 `ns-dev-*` seed 数据；只有显式执行 `scripts/dev-seed.sh --refresh .dev/tikeo-dev.db` 或设置 `TIKEO_DEV_SEED_REFRESH=1` 时才会刷新这些演示行。

```bash
cargo run --bin tikeo -- serve --config config/dev.yml
# 等 migrations 完成后停止 Server，再执行：
scripts/dev-seed.sh .dev/tikeo-dev.db
```

如果数据库不存在，脚本会提示先启动 Tikeo 让 migrations 创建 schema。

## 入站 Webhook 触发与出站通知渠道不要混用

Tikeo 有两类名字相近但方向相反的能力：

| 能力 | 方向 | 典型路径/表 | 用途 |
|---|---|---|---|
| 入站 Webhook 触发 | 外部系统 → Tikeo | `POST /api/v1/events/webhooks/{job}:trigger` | 把外部事件转成 Job Instance，`triggerType=webhook`。 |
| 出站通知渠道 | Tikeo → 外部系统 | `notification_channels`、`/api/v1/notification-channels` | 把作业事件投递到 webhook、Slack、钉钉、飞书、企微、PagerDuty 或 email。 |

入站 Webhook 签名字段包括 `secretRef`、`signature`、`timestamp`、`nonce`；`secretRef` 只解析环境变量引用，例如 `env:TIKEO_WEBHOOK_SECRET`。不要在 payload 示例里写真实密钥。

无签名的本地触发示例：

```bash
curl -fsS -X POST "http://127.0.0.1:9090/api/v1/events/webhooks/${TIKEO_DEMO_JOB_ID}:trigger" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d '{
    "source": "local-demo",
    "eventType": "demo.created",
    "payload": {"orderId":"demo-001"}
  }'
```

## 验收

完成演示数据准备后，应能拿到这些证据：

- `GET /api/v1/namespaces` 能看到演示 namespace。
- `GET /api/v1/apps?namespace=demo` 或脚本对应 namespace 能看到 app。
- `GET /api/v1/worker-pools?namespace=demo&app=orders` 能看到 worker pool。
- `GET /api/v1/jobs` 能看到 `scheduleType=api` 的 Job。
- `POST /api/v1/jobs/{job}:trigger` 返回实例 ID。
- 有在线 Worker 广告对应 `processorName` 时，实例最终进入成功或可解释的失败状态。
- `GET /api/v1/instances/{instance}/logs` 能看到执行日志或治理失败原因。
- 控制台中的 Workers、Jobs、Instances、Audit 页面与 API 结果一致。

## 排障

| 现象 | 检查项 | 处理 |
|---|---|---|
| `/healthz` 不通 | Server 是否运行、端口是否被占用 | 用 `cargo run --bin tikeo -- serve --config config/dev.yml` 重新启动，确认监听端口。 |
| `/readyz` 失败 | SQLite 文件权限、migration 日志、配置覆盖 | 查看 Server 日志和 `config/dev.yml` 中 `storage.database.*`。 |
| API 返回 401/403 | token、角色权限、scope 绑定 | 重新登录或使用有 `jobs:write`、`jobs:read`、`instances:execute`、`tenants:manage` 权限的账号。 |
| Job 一直 pending | 没有在线 Worker 或 capability 不匹配 | 启动对应 demo Worker，确认它广告 `demo.echo` 等 processor。不要用宽泛 wildcard capability 掩盖问题。 |
| 入站 Webhook 返回签名错误 | `secretRef`、`timestamp`、`nonce`、`signature` | 确认 `secretRef` 指向进程环境变量；timestamp 与当前时间相差不要超过 300 秒；nonce 不要复用。 |
| `scripts/dev-integration-seed.sh` 登录失败 | bootstrap 状态、用户名密码、token 环境变量 | 设置 `TIKEO_SMOKE_AUTH_TOKEN` 或 `TIKEO_ADMIN_TOKEN`，或设置正确的 `TIKEO_ADMIN_USERNAME` / `TIKEO_ADMIN_PASSWORD`。 |

## 清理

本地 SQLite 环境可以停止 Server 后删除数据库文件：

```bash
rm -f .dev/tikeo-dev.db .dev/tikeo-dev.db-shm .dev/tikeo-dev.db-wal
```

如果只想清理 API 创建的对象，请先删除 Job，再删除 worker pool、app、namespace；避免留下引用关系导致删除失败。

```bash
curl -fsS -X DELETE "http://127.0.0.1:9090/api/v1/jobs/${TIKEO_DEMO_JOB_ID}" \
  -H "Authorization: Bearer ${TIKEO_TOKEN}"
```

## 生产检查清单

- 不在生产数据库执行 `scripts/dev-seed.sh`。
- 不手工插入演示行绕过 RBAC、审计、migration 和领域校验。
- 不把真实 token、Webhook URL、SMTP 密码、PagerDuty routing key 或 Authorization header 写入示例。
- 演示 Job 使用明确 `namespace`、`app`、`processorName` 或 `scriptId`。
- 演示完成后清理临时 Job、worker pool、API key 和测试账号。
- 对外展示前确认实例日志、通知渠道和审计日志中没有敏感业务数据。

## 故障排查

步骤失败时，先保留完整命令、响应状态和 Server 日志时间窗口，再检查认证、namespace/app scope、Worker 匹配、存储 readiness 和代理行为，不要直接修改生产配置。
