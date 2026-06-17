---
title: Node.js Worker SDK
description: Node.js 依赖、最小 Worker、异常处理和 Management client 写法。
---

# Node.js Worker SDK

先读 [SDK 与 API 集成指南](../integrations/sdk-and-api)。本文只说明 Node.js 特有的依赖安装、最小 Worker、异常捕获和 Management client 写法。Node.js SDK 位于 `sdks/nodejs/tikeo`；demo 位于 `examples/nodejs/worker-demo`。仓库 Node/frontend 命令默认使用 Bun。

## 前置条件

| Requirement | Value |
| --- | --- |
| Package | `@yhyzgn/tikeo` |
| 版本占位符 | `${TIKEO_VERSION}`，来自 README 顶部包徽标或 release tag。 |
| Module format | ESM (`type: module`) |
| Consumer runtime | Node.js `>=24.0.0` |
| Runtime deps | `@grpc/grpc-js`, `@grpc/proto-loader` |
| Repo command runner | `bun` / `bunx` |

```bash
bun add @yhyzgn/tikeo@${TIKEO_VERSION}
cd sdks/nodejs/tikeo
bun install --frozen-lockfile
bun test
bun run build
```

## 最小 Worker

`localConfig(endpoint, clientInstanceId)` 将 namespace/app 默认成 `default`，cluster/region 默认成 `local`，version 默认 `dev`，heartbeat 默认 10 秒。只添加 Worker 真正能运行的 processor。

```typescript
import { Client, TaskContext, TaskOutcome, installConsoleTaskLogBridge, localConfig } from "@yhyzgn/tikeo";

function process(task: TaskContext): TaskOutcome {
  console.info(`nodejs echo processor=${task.processorName}`);
  return { success: true, message: "nodejs echo processed" };
}

const config = localConfig("http://127.0.0.1:9998", "nodejs-worker-1");
config.namespace = "sdk-smoke";
config.app = "management";
config.addSDKProcessor("demo.echo");
config.labels.worker_pool = "nodejs-blue";

installConsoleTaskLogBridge(); // 只在当前 Tikeo 任务作用域内镜像 console.*
const client = new Client(config);
const session = await client.connect();
const stop = session.startHeartbeat();
try {
  while (true) await session.processNext(process);
} finally {
  stop();
  session.close();
}
```

## 异常捕获

| Case | Node.js 行为 |
| --- | --- |
| 预期业务失败 | 返回 `{ success: false, message }` 或 SDK `failed(...)` helper。 |
| Processor exception | 抛 `Error`；SDK 上报 task failure 并记录错误路径。 |
| 不支持的 processor | 返回 failure，不要广告未实现 processor。 |
| Task logs | 优先使用 `console.info/error/warn/log` + `installConsoleTaskLogBridge()`；`TaskContext.logInfo/logError` 仅作为底层 fallback。日志可通过 Management API logs endpoint 查看。 |

## Management client 写法

```typescript
import { BroadcastSelectorRequest, ManagementClient, apiJob, apiTrigger, broadcastApiTrigger } from "@yhyzgn/tikeo";

const client = new ManagementClient(
  "http://127.0.0.1:9090",
  process.env.TIKEO_MANAGEMENT_API_KEY ?? "",
  "sdk-smoke",
  "management",
);
const job = await client.createJob(apiJob("nodejs-echo-api", "demo.echo"));
const instance = await client.triggerJob(job.id, apiTrigger());
const selector: BroadcastSelectorRequest = { labels: { worker_pool: "nodejs-blue" } };
const broadcast = broadcastApiTrigger(selector);
console.log(`instance=${instance.id} triggerType=api executionMode=single`);
console.log(`broadcastSelector=${JSON.stringify(broadcast.broadcastSelector)}`);
```

`ManagementClient` 发送 `x-tikeo-api-key`，trim endpoint，空 namespace/app 默认 `default`，并提供 `apiJob`、`apiTrigger`、`broadcastApiTrigger`、`BroadcastSelectorRequest`：

- Create helper → [`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- Trigger helper → [`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- Instance polling → [`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- Log inspection → [`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker dispatch → [`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## 验收

| Check | Command or evidence |
| --- | --- |
| SDK tests/build | `cd sdks/nodejs/tikeo && bun test && bun run build` |
| Worker registration | Worker 带 `demo.echo` 和 `worker_pool=nodejs-blue`。 |
| API trigger | Instance 显示 `triggerType=api` 和 `executionMode=single`。 |
| Worker logs | Instance logs 包含 `nodejs echo processed` 或 demo 证据 `nodejs demo echo processed`。 |

## 故障排查

| 现象 | 修复 |
| --- | --- |
| 旧 Node 无法运行 package | 使用 Node.js 24+，仓库 demo 使用 Bun。 |
| Unauthorized Management request | 检查 `TIKEO_MANAGEMENT_API_KEY` 和 `x-tikeo-api-key`。 |
| Worker 处于 dry-run | Live tunnel demo 设置 `TIKEO_WORKER_CONNECT=1`。 |
| Broadcast 范围太宽 | 使用带 `broadcastSelector` 的 `broadcastApiTrigger`。 |

## 生产检查清单

- [ ] 仓库命令使用 Bun，package consumer 使用 Node.js 24+。
- [ ] Worker Tunnel URL 与 Management HTTP URL 分开。
- [ ] API key 从 `TIKEO_MANAGEMENT_API_KEY` 或 secret manager 加载。
- [ ] Exception 会转换成可观测 failure。
- [ ] Broadcast helper 只用于明确 fan-out。


## 统一配置参数与默认值

不同语言 SDK 的代码写法不同，但接入 Tikeo 时面对的是同一组语义。不要把这些参数理解成各语言私有字段；它们最终都会进入 Worker Tunnel 注册、任务派发、Management API 创建任务和实例触发链路。

| 参数 | 默认值 | 生产建议 |
| --- | --- | --- |
| `endpoint` | 本地 Worker Tunnel 通常是 `http://127.0.0.1:9998` | 生产必须指向 Server 暴露的 Worker Tunnel 地址，并与 TLS/mTLS 配置一致。 |
| `namespace` | `default` 或示例中的 `sdk-smoke` | 每个团队、租户或环境应使用清晰命名，不要把生产任务混进 default。 |
| `app` | `default` 或示例中的 `management` | 与 Management API Key 的 app scope 保持一致。 |
| `clientInstanceId` | 示例手工指定 | 生产中应唯一且稳定，便于 Worker 页面和审计定位。 |
| `cluster` / `region` | `local` | 多机房部署必须真实填写，广播和选择器会使用这些信息。 |
| `labels` | 空 map | 用 `worker_pool`、`region`、`cluster` 等标签表达调度边界。 |
| `sdkProcessors` | 空列表 | 只声明当前进程真实实现的 processor，避免实例被派发后失败。 |
| `heartbeat` | 约 10 秒 | 保持默认即可；高延迟网络再根据运维策略调整。 |

## 管理客户端凭证

Management client 使用应用级 API Key，不使用浏览器里的人工登录 token。创建 key 时要绑定 namespace/app，运行时通过 `TIKEO_MANAGEMENT_API_KEY` 注入。所有语言的请求都会发送 `x-tikeo-api-key`，创建任务时应明确 `triggerType=api`、`executionMode=single`，需要广播时再设置 `broadcastSelector`。

| 决策 | 推荐做法 | 风险 |
| --- | --- | --- |
| API Key 保存位置 | Secret Manager、Kubernetes Secret 或 CI secret | 不要写进代码、README、截图或 shell 历史。 |
| 权限范围 | app-scoped service account | 不要用 Owner 或全局管理账号跑 SDK。 |
| 轮换 | 发布窗口内双写新旧 key | 直接删除旧 key 会让 Worker 或自动化立即失败。 |
| 验证 | 先创建 API 手动触发任务，再触发一次 | 只构建通过不能证明 Management API 可用。 |

## 现场验收 runbook

1. 确认 Server `/readyz` 通过，Web 控制台能看到目标 namespace/app。
2. 使用当前语言启动一个只声明 `demo.echo` 的 Worker。
3. 在 Worker 页面确认 `clientInstanceId`、region、cluster、labels 和 processor 列表正确。
4. 使用 Management client 创建 API 触发任务，确认返回 job id。
5. 触发一次 single instance，进入 Instances 页面查看状态、Worker、日志和 result。
6. 如果要验证广播，设置 `broadcastSelector`，确认多个符合标签的 Worker 都生成 attempt 或广播实例证据。
7. 制造一次业务失败和一次运行时异常，确认日志中能看到 message、stack 或错误路径。
8. 给失败事件绑定通知渠道，确认消息中的实例 ID、时间、状态、操作人、执行类型可以追溯。

## 故障排查表

| 现象 | 可能原因 | 处理方式 |
| --- | --- | --- |
| Worker 页面看不到进程 | endpoint/TLS/mTLS 或 token 不匹配 | 先看 Worker 启动日志，再看 Server Worker Tunnel 日志。 |
| 实例一直等待 | processorName、标签或 region/cluster 不匹配 | 对照 Jobs 页和 Workers 页的 capability。 |
| 触发 API 返回 401/403 | `TIKEO_MANAGEMENT_API_KEY` 无效或 scope 不对 | 重新创建 app-scoped key，确认 header 是 `x-tikeo-api-key`。 |
| 执行失败但没有日志 | processor 异常未被 SDK 捕获或进程崩溃 | 升级 SDK，确保 task log API 被调用，并查看 Worker 本地日志。 |
| 广播没有命中目标 | `broadcastSelector` 标签与 Worker labels 不一致 | 先用单实例验证，再逐步加 selector。 |

## 生产检查清单

- [ ] 依赖坐标固定到发布版本，而不是随意使用本地源码路径。
- [ ] WorkerConfig 默认值已经被生产环境显式覆盖。
- [ ] 最小 Worker 在目标环境成功注册并展示能力。
- [ ] 管理客户端凭证来自 Secret，不来自人工账号。
- [ ] 现场验收 runbook 的创建、触发、日志、失败、通知链路均通过。
