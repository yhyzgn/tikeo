# tikee Go Worker SDK

First Go SDK slice for active outbound Worker Tunnel clients.

This package currently provides a protocol-neutral worker boundary plus official Go gRPC/protobuf bindings:

- worker configuration validation
- registration message shape
- heartbeat message shape
- task processor/outcome interfaces
- `grpc.ClientConn` creation with endpoint normalization
- generated Worker Tunnel client bindings in `internal/workerpb`

It uses the official `google.golang.org/grpc` and `google.golang.org/protobuf` modules. The vendored proto is generated with official `protoc-gen-go` / `protoc-gen-go-grpc`; `scripts/generate-workerpb.sh` regenerates bindings and splits the generated protobuf file into sub-1500-line package files to preserve the repository source-size rule.

```bash
cd sdks/go/tikee
go test ./...
```
