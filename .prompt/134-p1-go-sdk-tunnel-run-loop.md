# 134 — P1 Go SDK tunnel run-loop next slice

## Context
Go SDK official gRPC/protobuf foundation is now in place: `sdks/go/tikee` validates config, exposes registration/heartbeat/task processor shapes, vendors the Worker Tunnel proto, creates official `grpc.ClientConn`, and includes generated Worker Tunnel client bindings under `internal/workerpb`. Generated protobuf output is split into sub-1500-line package files to respect the repository file-size rule.

Python and Node.js SDKs are explicitly deferred per user instruction; do not start them in the next slice.

## Preferred next slice
Implement the Go SDK ergonomic Worker Tunnel run loop:
- send `RegisterWorker` through `OpenTunnel`
- process `WorkerRegistered`, `Ping`, and `DispatchTask` messages
- route dispatches to `TaskProcessor` and send `TaskResult` / `TaskLog` with assignment token
- handle heartbeat interval and unregister/close behavior
- add in-process fake gRPC server tests for registration, heartbeat, dispatch result, and close path

## Validation target
- `(cd sdks/go/tikee && go test ./...)`
- `(cd examples/go/worker-demo && go test ./...)`
- source-size check: no `.go`, `.rs`, `.java`, `.proto`, `.ts`, or `.tsx` file over 1500 lines
- do not implement Python/Node.js SDKs in this slice
