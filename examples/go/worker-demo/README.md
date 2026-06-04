# Go Worker demo

Go Worker demo aligned with the Java manual acceptance scopes.

```bash
cd examples/go/worker-demo
go test ./...
go run .
```

Dry-run mode prints the registration snapshot and a local heartbeat. Live Worker Tunnel mode:

```bash
TIKEE_WORKER_CONNECT=1 \
TIKEE_WORKER_ENDPOINT=http://127.0.0.1:9998 \
TIKEE_WORKER_CLIENT_INSTANCE_ID=go-worker-demo-local \
go run .
```

Defaults:

- namespace/app: `dev-alpha/orders`
- client instance id: `go-worker-demo-local`
- SDK processors: `demo.echo`, `demo.context`, `demo.bytes`, `demo.heartbeat`, `demo.fail`
- tags: `go`, `manual-demo`

Optional structured capabilities:

- `TIKEE_ENABLE_PLUGIN_SQL=1` advertises plugin processor `type=sql`, `processorName=billing.sql-sync`.
- `TIKEE_WORKER_SCRIPT_LANGUAGES=shell,python` advertises structured script runners with `TIKEE_WORKER_SCRIPT_SANDBOX` as backend label.
- `TIKEE_MANAGEMENT_CREATE_EXAMPLES=1` uses `TIKEE_HTTP_URL` and `TIKEE_API_KEY` to create SDK/plugin job examples in the configured scope.
