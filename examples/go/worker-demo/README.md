# Go Worker demo

Dry-run demo for the first Go SDK slice.

```bash
cd examples/go/worker-demo
go test ./...
go run .
```

This demo exercises config, registration shape, heartbeat shape, and the SDK's official Go gRPC/protobuf dependency boundary. The SDK can create an official `grpc.ClientConn` and generated Worker Tunnel client; the full ergonomic tunnel run loop is a later slice.
