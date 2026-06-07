# Tikeo Go Worker SDK 🐹

[🇨🇳 中文 SDK 文档](../../../docs/zh-CN/sdk.md)

Go SDK for active outbound Worker Tunnel clients and app-scoped management APIs.

## Features

- Real gRPC Worker Tunnel client using official `google.golang.org/grpc`.
- Structured worker capabilities for SDK processors, plugin processors, script runners, and tags.
- Task-scoped logging through `TaskContext.LogInfo` / `LogError`.
- Bridge-friendly SDK diagnostics through the `Logger` interface, default `INFO`, console output, and optional `tikeo-sdk.log`.
- SRT/Deno sandbox auto tooling aligned with Java/Rust behavior.
- Management helpers for SDK, plugin, and script jobs.

## Usage

```go
config := tikeo.LocalConfig("http://127.0.0.1:9998", "orders-go-1")
config.Namespace = "dev-alpha"
config.App = "orders"
config.AddSDKProcessor("demo.echo")
tikeo.ConfigureLogging(tikeo.LogConfigFromEnv())
client, err := tikeo.NewClient(config)
_ = client
_ = err
```

## Operational cautions

- Task instance logs and SDK diagnostics are separate by design.
- Keep diagnostics at INFO unless investigating connectivity, sandbox, or registration issues.
- Do not advertise script capabilities until the sandbox runner is registered and executable.
- Use `SetLogger` to bridge into slog/zap/logrus without adding SDK framework coupling.

## Verification

```bash
go test ./...
```
