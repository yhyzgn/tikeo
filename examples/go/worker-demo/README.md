# Go Worker demo 🐹

[🇨🇳 中文示例文档](../../../README.zh-CN.md#能证明产品价值的快速开始)

This demo validates Go SDK parity with the Java, Rust, Python, and Node.js workers.

```bash
# Start Tikeo first from the repository root:
# ./scripts/dev.sh

cd examples/go/worker-demo
go run .
```

Dry-run smoke:

```bash
TIKEO_WORKER_DRY_RUN=1 go run .
go test ./...
```

Management API create + trigger example:

```bash
TIKEO_API_KEY=<app-scoped-sdk-key> \
TIKEO_HTTP_URL=http://127.0.0.1:8080 \
TIKEO_MANAGEMENT_CREATE_EXAMPLES=1 \
go run .
```

When enabled, the demo creates API-scheduled SDK/plugin jobs and immediately calls
`POST /api/v1/jobs/{job}:trigger`, printing the returned instance id, `triggerType=api`, and `executionMode=single`.

Defaults:

- scope: `dev-alpha/orders`
- worker pool: `go-blue`
- SDK processors: `demo.echo`, `demo.context`, `demo.bytes`, `demo.heartbeat`, `demo.fail`
- plugin processor: `type=sql`, `processorName=billing.sql-sync`
- script runners: shell, Python, JavaScript, TypeScript, PowerShell, PHP, Groovy, Rhai
- sandbox auto path: SRT for native scripts and Deno for JavaScript/TypeScript

Operational cautions: keep task logs task-scoped, keep SDK diagnostics at INFO unless debugging, and
do not advertise script runners unless the sandbox backend is available.
