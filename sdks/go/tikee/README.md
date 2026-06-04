# tikee Go Worker SDK

Go SDK for active outbound Worker Tunnel clients and app-scoped management API calls.

This package provides:

- worker configuration validation
- structured Worker capabilities: tags, SDK processors, script runners, and plugin processors
- real Worker Tunnel registration, heartbeat, task result, and graceful unregister helpers
- task processor/outcome interfaces
- `grpc.ClientConn` creation with endpoint normalization
- generated Worker Tunnel client bindings in `internal/workerpb`
- app-scoped management client helpers for SDK, plugin, and script jobs

Dispatch routing must use structured capability fields. Legacy free-form `Capabilities` remain operator metadata only.

The vendored proto is generated with official `protoc-gen-go` / `protoc-gen-go-grpc`; `scripts/generate-workerpb.sh` regenerates bindings and splits the generated protobuf file into sub-1500-line package files to preserve the repository source-size rule.

```bash
cd sdks/go/tikee
go test ./...
```
