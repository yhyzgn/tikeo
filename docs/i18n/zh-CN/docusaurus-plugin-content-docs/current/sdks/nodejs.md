---
title: Node.js Worker SDK
description: Node.js SDK 与 Bun 驱动 Worker demo 的验证入口。
---

# Node.js Worker SDK

Node.js SDK 位于 `sdks/nodejs/tikeo`，可运行 Worker demo 位于 `examples/nodejs/worker-demo`。仓库脚本默认使用 Bun；发布包为 Node.js 消费者声明 runtime baseline。

## 运行时要求

SDK package 声明 `engines.node >=24.0.0`。仓库内安装、测试、构建和 demo 执行默认使用 Bun。调整 baseline 时必须同步 `package.json`、README 徽章、文档和 CI runtime policy。

## 从 npm 安装

将 `${TIKEO_VERSION}` 替换为 README 顶部 `Node.js SDK` 徽标显示的版本号。npm 使用不带 `v` 的版本字符串。

```bash
bun add @yhyzgn/tikeo@${TIKEO_VERSION}
npm install @yhyzgn/tikeo@${TIKEO_VERSION}
pnpm add @yhyzgn/tikeo@${TIKEO_VERSION}
```

```ts
import { Client, WorkerConfig } from "@yhyzgn/tikeo";
```

## 验证 SDK

```bash
cd sdks/nodejs/tikeo
bun install
bun test
bun run build
```

build 会输出 `dist/index.js`、TypeScript declaration，以及复制到 `dist/proto` 的 Worker Tunnel protobuf 资源。

## 验证 demo

```bash
cd examples/nodejs/worker-demo
bun install
TIKEO_WORKER_DRY_RUN=1 bun start
bun test
```

dry-run mode 可在没有 live Server 的情况下验证本地包链接和 capability metadata。

## Live mode 预期

live mode 默认连接 `http://127.0.0.1:9998`，使用 demo 的开发 scope。demo 包含 JS/TS 友好的 processor 行为，以及已配置 script path 的 sandbox runner auto-resolution。运行前先启动 Server，并确认 Web 控制台能看到 Worker。


## Management API 创建并触发任务

Node.js management surface 实现在 `sdks/nodejs/tikeo/src/management.ts`。它发送 app 级 `x-tikeo-api-key` header，通常通过 `TIKEO_MANAGEMENT_API_KEY` 注入；不要把它与浏览器 session 或人类 API token 混用。`apiJob` 创建 API 调度的 processor job，`apiTrigger` 发送 `triggerType=api` 和默认 `executionMode=single`。

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

if (instance.triggerType !== "api" || instance.executionMode !== "single") {
  throw new Error("unexpected trigger response");
}
```

广播是 opt-in。`broadcastApiTrigger` 会序列化 `executionMode=broadcast` 与 `broadcastSelector`，代码评审时能清楚看出 fan-out，而不会把它误当成单 Worker 默认触发。

```ts
const selector: BroadcastSelectorRequest = {
  tags: ["manual-demo"],
  region: "us-east-1",
  labels: { worker_pool: "nodejs-blue" },
};
await management.triggerJob(created.id, broadcastApiTrigger(selector));
```


## Source-backed 参考链接

SDK helper 文档必须锚定到从源码整理出的 API 与协议参考：

- 创建 helper 端点：[`POST /api/v1/jobs`](../reference/management-openapi#post-api-v1-jobs)
- 触发 helper 端点：[`POST /api/v1/jobs/{job}:trigger`](../reference/management-openapi#post-api-v1-jobs-job-trigger)
- 实例轮询端点：[`GET /api/v1/instances/{instance}`](../reference/management-openapi#get-api-v1-instances-instance)
- 实例日志端点：[`GET /api/v1/instances/{instance}/logs`](../reference/management-openapi#get-api-v1-instances-instance-logs)
- Worker 派发消息：[`DispatchTask`](../reference/worker-tunnel-protobuf#dispatchtask)

## 能力广告纪律

Node.js Worker 集成 Web 生态很快，但仍必须遵守 Tikeo 调度契约。SQL、script、plugin 或 processor capability 必须真实可执行；缺失工具应 fail closed，并产生可见任务或诊断错误。

## 生产建议

优先使用包含 Bun 或明确 Node.js 24+ runtime 的最小镜像。除非发布工具明确引入，不要在该仓库路径混入 npm/yarn lockfile。

## 适合场景

Node.js Worker 适合前端平台、Webhook 编排、TypeScript 业务工具和 JavaScript 生态集成。评估时应同时验证 TypeScript 类型输出、Bun/Node runtime 选择、protobuf 资源打包，以及 live mode 下的 tunnel 注册与任务日志。

如果业务依赖原生扩展，请在目标镜像中单独验证构建与运行环境，不要只依赖开发机结果。
