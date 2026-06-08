# @yhyzgn/tikeo 🟢

[🇨🇳 中文 SDK 文档](../../../README.zh-CN.md#行为一致的-sdk)

Node.js SDK aligned with the Rust, Go, Java, and Python Worker SDKs.

## Runtime requirements

- Node.js 24+ is the supported package runtime baseline.
- Bun is the repository package runner for test/build scripts.

## Features

- Worker Tunnel client with structured capabilities.
- Task processors with precise task-scoped logs.
- SDK diagnostics through `configureSdkLogging`, default `INFO`, console output, and optional `tikeo-sdk.log`.
- Management API client using `x-tikeo-api-key`.
- SRT/Deno/container/local script runners and fail-closed unavailable handlers.
- Default `auto`: SRT for native scripts, Deno for JavaScript/TypeScript.

## Usage

```ts
import { Client, WorkerConfig, configureSdkLogging } from "@yhyzgn/tikeo";

configureSdkLogging({ level: "info", logDir: "./logs" });
const config = new WorkerConfig({ endpoint: "http://127.0.0.1:9998", clientInstanceId: "orders-node-1" });
config.namespace = "dev-alpha";
config.app = "orders";
config.addSDKProcessor("demo.echo");
const client = new Client(config);
```

## Operational cautions

- Bun is the default repository package runner for this project.
- Do not capture global stdout/stderr for task logs; use the task-scoped log callback.
- Keep SDK diagnostics at INFO unless debugging Worker Tunnel or sandbox issues.

## Verification

```bash
bun test
bun run build
```
