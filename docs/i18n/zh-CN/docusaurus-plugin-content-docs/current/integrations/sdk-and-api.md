---
title: SDK 与 API 集成指南
description: 连接应用 Worker、创建 API 任务、触发任务、接入通知并验证执行证据的分步指南。
---

# SDK 与 API 集成指南

这页面向要把 Tikeo 接入业务服务的应用团队。它解释要写什么代码、要申请哪些凭证、Worker 如何连接、任务如何创建和触发，以及什么证据能证明集成成功。

## 集成模型

典型集成包含两个独立客户端：

| 客户端 | 凭证 | 方向 | 目的 |
| --- | --- | --- | --- |
| Worker SDK | Worker 身份/配置 | Worker → Worker Tunnel | 注册 processor，接收 `DispatchTask`，上报 `TaskLog`，返回 `TaskResult`。 |
| Management SDK/API client | `x-tikeo-api-key` | 应用 → Server HTTP API | 创建 API 触发任务、触发任务、读取实例/日志。 |

不要用 Worker Tunnel 做管理调用，也不要在应用服务里使用人类 Web session token。

## 写代码之前

先向平台运维确认：

- Server HTTP base URL，例如 `https://tikeo.example.com`。
- Worker Tunnel endpoint，例如 `https://tikeo-worker.example.com` 或私网 `http://tikeo-server:9998`。
- `namespace`、`app`、`workerPool` 命名规范。
- 一个应用级 SDK API Key，用于 `x-tikeo-api-key`。
- processor 名称和 payload schema。
- 如果任务状态需要触达人员，确认通知 channel/template/policy 预期。

## 步骤 1：选择语言 SDK

| 语言 | 页面 | 适用场景 |
| --- | --- | --- |
| Rust | [Rust SDK](../sdks/rust) | Rust 服务和高吞吐 Worker。 |
| Go | [Go SDK](../sdks/go) | 小型静态 Worker 服务和平台 agent。 |
| Java/Spring Boot | [Java SDK 与 Spring Boot](../sdks/java-spring-boot) | Spring 服务和注解式 processor。 |
| Python | [Python SDK](../sdks/python) | 数据/自动化任务和 Python 服务团队。 |
| Node.js | [Node.js SDK](../sdks/nodejs) | TypeScript/JavaScript 服务和快速 demo。 |

每个 SDK 页面都包含依赖坐标、WorkerConfig 默认值、最小 Worker、管理客户端凭证和现场验收 runbook。

## 步骤 2：实现 Worker processor

Worker 应声明自己真正能执行的 processor。示例命名：

```text
namespace: billing
app: invoices
workerPool: default
processorName: invoice.send-reminder
```

Worker 主动出站连接并上报 capability。能力声明要真实：不要在 Worker 能安全执行之前声明某个 processor、脚本后端或插件类型。

## 步骤 3：创建 API 触发任务

使用 Management SDK 或原始 HTTP API。关键字段是：

- `triggerType=api`
- `executionMode=single` 表示一个 Worker 结果；广播场景使用 broadcast helpers。
- processor 名称必须匹配 Worker 注册能力。
- namespace/app 必须匹配应用级 API Key 的范围。

HTTP 示例：

```bash
curl -fsS -X POST "$TIKEO_URL/api/v1/jobs" \
  -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY" \
  -H 'content-type: application/json' \
  -d '{
    "name":"send invoice reminder",
    "namespace":"billing",
    "app":"invoices",
    "processorType":"sdk",
    "processorName":"invoice.send-reminder",
    "triggerType":"api",
    "executionMode":"single",
    "enabled":true
  }' | jq .
```

类型化 helper 名称见各 SDK 页面：`ManagementClient`、`NewManagementClient`、`HttpTikeoJobClient`、`apiJob`、`apiTrigger`、`broadcastApiTrigger` 和 `BroadcastSelectorRequest`。

## 步骤 4：触发并查看实例

```bash
INSTANCE_ID="$(curl -fsS -X POST "$TIKEO_URL/api/v1/jobs/$JOB_ID:trigger" \
  -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY" \
  -H 'content-type: application/json' \
  -d '{"payload":{"invoiceId":"inv_123"}}' | jq -r .data.instanceId)"

curl -fsS "$TIKEO_URL/api/v1/instances/$INSTANCE_ID" \
  -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY" | jq .

curl -fsS "$TIKEO_URL/api/v1/instances/$INSTANCE_ID/logs" \
  -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY" | jq .
```

预期证据：实例状态成功或有清晰失败日志；日志包含 Worker 上报内容；Workers 页面显示 Worker 在线；必要时审计日志可追踪操作。

## 广播集成

广播适用于“每个匹配 Worker 都执行”的场景。它要求认真设计 selector：

- 按 namespace/app/workerPool 匹配。
- 可按 labels/tags/capabilities 匹配。
- 预期会有多个 child attempts 和按 Worker 的结果行。
- 使用 `broadcastSelector` 和 SDK helper `BroadcastSelectorRequest`，不要自造 JSON 约定。

业务操作必须执行一次时不要使用广播。

## 通知集成

任务成功/失败/always 通知不要写死在 Worker 里。使用通知中心：

1. 运维创建带 provider 凭据的 channel。
2. 创建或选择 template。
3. Job owner 创建 success/failure/always 绑定。
4. Runtime 用 `jobId`、`instanceId`、`status`、`operatorName`、`executionMode`、`logsUrl` 等字段物化消息。
5. 运维查看 delivery attempts 和 message trace。

参考 [通知](../user-guide/notifications) 与 [通知中心参考](../reference/notification-center)。

## 错误处理契约

Worker processor 应该：

- 在边界验证 payload，并返回可行动错误。
- 在外部调用前后写 task logs。
- 不记录 secret、token、provider URL、password 或 authorization header。
- 如果 Tikeo retry 可能重复调用下游系统，要使用幂等 key。
- 把取消和超时当作正常运维状态处理。

Management client 应该：

- 将 `TIKEO_MANAGEMENT_API_KEY` 放入 Secret 管理。
- 只重试幂等读取或明确幂等的触发。
- 在业务日志中记录 `instanceId`，便于关联证据。

## 本地集成 smoke

最快的完整检查：

```bash
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh
```

它会启动隔离环境、创建应用级凭证、用 `TIKEO_WORKER_CONNECT=1` 启动 Node.js Worker demo、创建并触发 API job，并把证据写到 `.dev/reports/management-trigger-e2e-*`。

## 前置条件

- 可访问 Server HTTP API 和 Worker Tunnel endpoint。
- 平台批准的 namespace/app/workerPool。
- `TIKEO_MANAGEMENT_API_KEY` 中的应用级 SDK API Key。
- 至少一个 Worker SDK 依赖已安装。
- 已定义 payload schema 和 processor 命名。

## 验收

成功集成应至少能执行：

```bash
curl -fsS "$TIKEO_URL/api/v1/jobs/$JOB_ID" -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY"
curl -fsS "$TIKEO_URL/api/v1/instances/$INSTANCE_ID" -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY"
curl -fsS "$TIKEO_URL/api/v1/instances/$INSTANCE_ID/logs" -H "x-tikeo-api-key: $TIKEO_MANAGEMENT_API_KEY"
```

Web 控制台应显示 Worker 在线、Job 启用、Instance 完成并带日志。

## 故障排查

| 现象 | 可能原因 | 修复 |
| --- | --- | --- |
| API Key 返回 401/403 | key 错误、app scope 错误、service account 禁用 | 重新签发应用级 key 并确认 namespace/app。 |
| Job 触发后 pending | 没有 Worker 上报匹配 processor/scope | 检查 Worker capability 和 namespace/app/workerPool。 |
| Worker 在线但不收任务 | processor 名称不匹配或 job disabled | 对比 job processor 字段与 Worker 注册。 |
| 缺日志 | Worker 没有上报 task logs 或 handler 前失败 | 在 handler start/end 增加日志。 |
| 下游重复执行 | retry 缺少幂等 | 用 `instanceId` 或业务 key 做幂等。 |

## 生产检查清单

- [ ] Worker 与 Management client 使用独立凭证/配置。
- [ ] Processor 名称和 payload schema 已与服务团队记录。
- [ ] API job 使用 `triggerType=api` 和预期 `executionMode`。
- [ ] 业务日志记录 instance ID。
- [ ] Worker 日志不包含 secret。
- [ ] 通知通过通知中心绑定，而不是临时 provider 代码。
