# Node.js Worker demo 🟢

[🇨🇳 中文示例文档](../../../README.zh-CN.md#能证明产品价值的快速开始)

This Bun-powered demo validates the Node.js SDK against the same manual acceptance matrix as the
Rust, Go, Python, and Java demos.

```bash
cd examples/nodejs/worker-demo
bun install
TIKEO_WORKER_DRY_RUN=1 bun start
bun test
```

Management API create + trigger example:

```bash
TIKEO_API_KEY=<app-scoped-sdk-key> \
TIKEO_HTTP_URL=http://127.0.0.1:8080 \
TIKEO_MANAGEMENT_CREATE_EXAMPLES=1 \
bun start
```

When enabled, the demo creates API-scheduled SDK/plugin jobs and immediately calls
`POST /api/v1/jobs/{job}:trigger`, printing the returned instance id, `triggerType=api`, and `executionMode=single`.

Live mode defaults to `http://127.0.0.1:9998`, `dev-alpha/orders`, stable worker id hints, SQL plugin
processor `billing.sql-sync`, and script runners using `auto` sandbox resolution.

Operational cautions: Bun is the default package runner in this repository; do not switch to npm/yarn
for project-local verification unless release tooling explicitly requires it.
