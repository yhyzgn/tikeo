# tikee Go SDKs

Go SDK packages live under `sdks/go/<sdk-name>/` and must be independently buildable.

```bash
(cd sdks/go/tikee && go test ./...)
(cd examples/go/worker-demo && go test ./...)
```

The first Go slice is a Worker client boundary using official `google.golang.org/grpc` for `ClientConn` creation and official `google.golang.org/protobuf` generated Worker Tunnel bindings. It vendors the Worker Tunnel protobuf contract and keeps generated files split below 1500 lines.
