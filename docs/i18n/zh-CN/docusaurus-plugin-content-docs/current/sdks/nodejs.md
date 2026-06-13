---
title: Node.js Worker SDK
description: Node.js SDK 与 Bun 驱动 Worker demo 的 operator-grade 验收入口。
---

# Node.js Worker SDK

Node.js SDK 位于 `sdks/nodejs/tikeo`，可运行 Worker demo 位于 `examples/nodejs/worker-demo`。仓库内前端/Node 命令默认使用 Bun。本文以 `src/config.ts`、`src/client.ts`、`src/management.ts` 和 demo `src/main.ts` 为事实来源。Node.js Worker 是 **outbound-only**：进程主动连接 Worker Tunnel、注册能力、接收 `DispatchTask`、发送 task log/result；不要把业务 Worker 写成 inbound HTTP Service。

## 依赖坐标

npm 包坐标来自 `sdks/nodejs/tikeo/package.json`：`name = "@yhyzgn/tikeo"`、`version = "0.2.0"`、`type = "module"`，并声明 `engines.node >=24.0.0`。安装发布包时使用不带 `v` 的版本：

```bash
bun add @yhyzgn/tikeo@${TIKEO_VERSION}
npm install @yhyzgn/tikeo@${TIKEO_VERSION}
pnpm add @yhyzgn/tikeo@${TIKEO_VERSION}
```

仓库内 demo 使用 file dependency：`"@yhyzgn/tikeo": "file:../../../sdks/nodejs/tikeo"`。SDK 导出 `WorkerConfig`、`localConfig`、`Client`、`TaskContext`、`TaskOutcome`、`ManagementClient`、`apiJob`、`pluginApiJob`、`scriptApiJob`、`apiTrigger`、`broadcastApiTrigger`、`BroadcastSelectorRequest` 和 `API_KEY_HEADER`。

## WorkerConfig 默认值

`localConfig(endpoint, clientInstanceId)` 返回 `new WorkerConfig({ endpoint, clientInstanceId })`。`WorkerConfig` 默认值来自 `src/config.ts`：`namespace="default"`，`app="default"`，`name=input.name || input.clientInstanceId`，`region="local"`，`version="dev"`，`cluster="local"`，`capabilities=[]`，`labels={}`，`structured.tags=[]`，`structured.sdkProcessors=[]`，`structured.scriptRunners=[]`，`structured.pluginProcessors=[]`，`heartbeatEveryMs=10_000`。`validate()` 会拒绝空 endpoint、clientInstanceId、namespace、app、name、cluster，以及非正 heartbeat。

Node.js demo 覆盖 operator scope：`TIKEO_WORKER_ENDPOINT` 默认 `http://127.0.0.1:9998`，`TIKEO_WORKER_CLIENT_INSTANCE_ID` 默认 `nodejs-worker-demo-local`，namespace/app 默认 `dev-alpha`/`orders`，cluster/region 默认 `local`，tag 包含 `nodejs` 与 `manual-demo`，默认 SDK processors 是 `demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail,demo.exception`，`worker_pool` 默认 `nodejs-blue`。`TIKEO_ENABLE_PLUGIN_SQL` 默认启用时，会广告 plugin type `sql` 与 processor `billing.sql-sync`。

## 最小 Worker

最小 Node.js Worker 只做 outbound tunnel 连接与 processor 执行。下面片段保留源码 API，但省略 demo 中的 script runner registry；生产中只有当 SRT、Deno、container 或 local command runner 真正可用时才广告对应能力。

```ts
import { Client, TaskContext, localConfig, type TaskOutcome } from "@yhyzgn/tikeo";

const config = localConfig(
  process.env.TIKEO_WORKER_ENDPOINT ?? "http://127.0.0.1:9998",
  process.env.TIKEO_WORKER_CLIENT_INSTANCE_ID ?? "nodejs-worker-demo-local",
);
config.namespace = "dev-alpha";
config.app = "orders";
config.addTag("nodejs");
config.addSDKProcessor("demo.echo");

const client = new Client(config);
async function processTask(task: TaskContext): Promise<TaskOutcome> {
  task.logInfo("nodejs echo started");
  return { success: true, message: "nodejs demo echo processed" };
}

while (true) {
  try {
    const session = await client.connect();
    const stop = session.startHeartbeat();
    try { await session.processNext(processTask); }
    finally { stop(); session.close(); }
  } catch (error) {
    console.warn(`worker tunnel ended, reconnecting: ${(error as Error).message}`);
    await new Promise((resolve) => setTimeout(resolve, 2_000));
  }
}
```

能力广告纪律：`addSDKProcessor`、`addScriptRunner`、`addPluginProcessor` 会影响 Server 调度，不能用来“占位”。任务日志用 `TaskContext.logInfo/logError`，SDK 诊断用 `sdkLog`/console。TypeScript 业务代码要明确 payload decoding、输出大小、timeout 与 secret refs，避免把灵活的 JS runtime 变成不可审计的宿主机执行。

## Management API 与管理客户端凭证

Node.js management surface 位于 `sdks/nodejs/tikeo/src/management.ts`。`ManagementClient(endpoint, apiKey, namespace="default", app="default")` 会 trim endpoint 尾部斜杠，空 namespace/app 默认 `default`，请求头包含 `accept: application/json`、`x-tikeo-api-key`，有 body 时加 `content-type: application/json`。凭证应从 `TIKEO_MANAGEMENT_API_KEY` 注入；不要把浏览器 session、OIDC cookie 或人类 API token 复用给 Worker。

helper 行为：`apiJob` 创建 `scheduleType=api`、`processorName`、`enabled=true`、默认 retry policy；`pluginApiJob` 写入 `processorType`；`scriptApiJob` 写入 `scriptId`；`apiTrigger()` 写入 `triggerType=api` 与 `executionMode=single`；`broadcastApiTrigger(selector)` 写入 `triggerType=api`、`executionMode=broadcast` 与 `broadcastSelector`。`triggerJob(jobId)` 不传 request 时默认使用 `apiTrigger()`。

```ts
import {
  ManagementClient,
  apiJob,
  apiTrigger,
  broadcastApiTrigger,
  type BroadcastSelectorRequest,
} from "@yhyzgn/tikeo";

const management = new ManagementClient(
  process.env.TIKEO_MANAGEMENT_ENDPOINT ?? "http://127.0.0.1:9090",
  process.env.TIKEO_MANAGEMENT_API_KEY ?? "",
  "dev-alpha",
  "orders",
);
const created = await management.createJob(apiJob("nodejs-echo-api", "demo.echo"));
const instance = await management.triggerJob(created.id, apiTrigger());
if (instance.triggerType !== "api" || instance.executionMode !== "single") throw new Error("unexpected trigger response");

const selector: BroadcastSelectorRequest = {
  tags: ["manual-demo"],
  region: "us-east-1",
  labels: { worker_pool: "nodejs-blue" },
};
await management.triggerJob(created.id, broadcastApiTrigger(selector));
```

参考锚点：[`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)、[`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)、[`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)、[`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)、[`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)。

## Demo 行为与 Bun 验证

Node.js demo 脚本为 `bun src/main.ts`，测试为 `bun test`。`configureScripts(config)` 会按 `TIKEO_WORKER_SCRIPT_LANGUAGES` 默认尝试 `shell,python,javascript,typescript,powershell,php,groovy,rhai`。auto sandbox 下 JS/TS 使用 Deno，其它语言使用 SRT；也支持 Docker/Podman 与显式 local command。`TIKEO_SANDBOX_AUTO_INSTALL` 可关闭工具自动安装。验收时应比较启动日志、`client.registration()` JSON 和 Web 控制台，确保 structured capabilities 只包含实际注册成功的 runner。

## 失败与异常 demo

所有语言 demo 都区分业务失败和运行时异常。`demo.fail` 返回正常的 failed `TaskOutcome`，用于验证业务规则失败；`demo.exception` 会 throw、panic、raise 或返回 processor error，用于验证 SDK 能把真实异常栈作为任务日志透传，同时仍把实例结果标记为失败。验收时两个 processor 都要触发：前者证明业务失败语义，后者证明异常堆栈能穿过 Worker Tunnel 并出现在 Notification Center 的执行透传页面。

## 运维依据与排错边界

核对 Node.js 集成时，先读 `sdks/nodejs/tikeo/src/config.ts` 的 `WorkerConfig`、`validate()` 和 capability helper，再读 `client.ts` 中 `registerMessage`、`Session.startHeartbeat`、`processNext` 与 `taskResult` 写回。`task.ts` 规定 `TaskProcessor` 返回 `{ success, message }`，实例日志通过 `TaskContext.logInfo/logError` 进入 tunnel。demo 的 `configureScripts` 是脚本能力排错入口：auto 模式下 JS/TS 走 Deno，其它语言走 SRT，container 和 local command 都需要显式条件。现场出现“任务没有到 Node Worker”时，先比较 `client.registration()` JSON 与 job 绑定，而不是修改业务代码；重点核对 namespace/app、processor 名、plugin type、`worker_pool` label、tag `manual-demo` 和 `broadcastSelector`。Bun 是仓库默认命令入口，但发布包仍面向 Node.js ESM 消费者。

## 生产上线检查

上线前明确运行时：仓库命令默认 Bun，发布包是 ESM Node.js SDK，目标镜像需要包含实际选择的 Bun 或 Node.js 24+ runtime。`clientInstanceId` 只用于稳定观测和重连提示，不能替代 Server 分配的 `workerId`、generation、lease 和 fencing token。多副本部署时，用 namespace、app、cluster、region、tags 和 `worker_pool` label 隔离环境。每次新增 processor、plugin type 或 script runner，都应同步更新 job 绑定、告警和回滚手册。Management API key 必须通过 `TIKEO_MANAGEMENT_API_KEY` 或等价 Secret 注入，禁止放入 `package.json` scripts、lockfile、console 输出或前端 bundle。

生产观测还应覆盖 Bun/Node 版本、event loop 延迟、tunnel 重连次数、heartbeat 延迟、任务失败分类和 management 请求错误率。滚动发布时先验证一个实例的 `client.registration()`，再扩容 worker pool，避免不同镜像层或 PATH 导致脚本能力漂移。

如果服务同时包含 Web API 和 Worker 进程，建议拆分启动命令和部署单元，避免健康检查、端口暴露和权限模型互相污染。任务 payload 解码、Buffer 大小、TextDecoder 错误处理和第三方 npm 包调用都应纳入代码评审；不要让前端构建产物或浏览器环境变量接触 Worker 管理凭证。

对于 Node.js 服务，建议把 Worker 专用 tsconfig、启动脚本和环境变量前缀同 Web 前端隔离。

这能避免 Vite、SSR 或浏览器注入机制把 Worker 配置误打包到用户可见产物里。

灰度时先只启动一个 Worker 实例，确认能力快照稳定后再扩容。

观察一个完整任务周期后再放量。

上线复核必须记录版本、配置和命令。

包括运行时版本和镜像摘要。
灰度期间禁止同时变更业务逻辑。
所有结论写入发布记录。
再扩容。
## 现场验收 runbook

1. SDK 测试与构建：`cd sdks/nodejs/tikeo && bun install && bun test && bun run build`，确认 `dist/index.js`、declaration 和 proto 资源生成。
2. demo dry-run：`cd examples/nodejs/worker-demo && bun install && TIKEO_WORKER_DRY_RUN=1 bun start && bun test`，确认输出 registration 与 `dry_run_heartbeat_sequence`，并确认没有业务 inbound service。
3. live tunnel：启动 Server，设置 `TIKEO_WORKER_ENDPOINT=http://127.0.0.1:9998`、namespace/app、cluster/region、`TIKEO_WORKER_POOL=nodejs-blue`，运行 demo，Web 控制台应看到 outbound Worker session。
4. Management 验收：用 `ManagementClient` 创建 `apiJob("nodejs-echo-api", "demo.echo")` 并 `apiTrigger()`；确认请求携带 `x-tikeo-api-key`，密钥来自 `TIKEO_MANAGEMENT_API_KEY`，响应 `triggerType=api` 与 `executionMode=single`。
5. 广播验收：只在明确需要扇出时调用 `broadcastApiTrigger`，用 `broadcastSelector` 限定 tag `manual-demo` 和 label `worker_pool=nodejs-blue`。
6. 失败与边界：触发 `demo.fail`，确认 instance 日志和失败状态；移除 Deno/SRT 后重启，确认不可用脚本 runner 没有被错误广告。

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
