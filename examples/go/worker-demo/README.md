# Go Worker demo

Go Worker demo aligned with the Java manual acceptance scopes.

Direct live Worker Tunnel mode, same default behavior as the Java and Rust demos:

```bash
# Start tikee first, for example from the repository root:
# ./scripts/dev.sh

cd examples/go/worker-demo
go run .
```

By default this connects to `http://127.0.0.1:9998`, registers under `dev-alpha/orders`, advertises the structured SDK processors and the SQL plugin processor, and should appear in the Worker cluster page as `go-worker-demo-local`.

Build/test:

```bash
go test ./...
```

Dry-run configuration smoke test:

```bash
TIKEE_WORKER_DRY_RUN=1 go run .
```

Explicit live Worker Tunnel example with script runner advertisement:

```bash
TIKEE_WORKER_ENDPOINT=http://127.0.0.1:9998 \
TIKEE_WORKER_CLIENT_INSTANCE_ID=go-worker-demo-local \
TIKEE_WORKER_SCRIPT_LANGUAGES=shell,python \
go run .
```

Defaults:

- namespace/app: `dev-alpha/orders`
- client instance id: `go-worker-demo-local`
- worker pool label: `go-blue`
- SDK processors: `demo.echo`, `demo.context`, `demo.bytes`, `demo.heartbeat`, `demo.fail`
- plugin processor: `type=sql`, `processorName=billing.sql-sync`
- tags: `go`, `manual-demo`

Environment variables:

- `TIKEE_WORKER_DRY_RUN=1` switches to dry-run mode without opening the Worker Tunnel.
- `TIKEE_WORKER_CONNECT=0` is also accepted as a compatibility dry-run switch.
- `TIKEE_WORKER_ENDPOINT` defaults to `http://127.0.0.1:9998`.
- `TIKEE_WORKER_CLIENT_INSTANCE_ID` overrides the stable client instance id.
- `TIKEE_WORKER_NAMESPACE` / `TIKEE_WORKER_APP` override the default `dev-alpha/orders` scope.
- `TIKEE_WORKER_POOL` overrides the default `go-blue` worker pool label.
- `TIKEE_WORKER_SDK_PROCESSORS` overrides the comma-separated SDK processor list.
- `TIKEE_ENABLE_PLUGIN_SQL` defaults to enabled; set `TIKEE_ENABLE_PLUGIN_SQL=0` to stop advertising the SQL plugin processor.
- `TIKEE_PLUGIN_SQL_TYPE` and `TIKEE_PLUGIN_SQL_PROCESSOR` override the default `sql` / `billing.sql-sync` structured plugin fields.
- `TIKEE_WORKER_SCRIPT_LANGUAGES=shell,python` advertises structured script runners with `TIKEE_WORKER_SCRIPT_SANDBOX` as backend label.
- `TIKEE_MANAGEMENT_CREATE_EXAMPLES=1` uses `TIKEE_HTTP_URL` and `TIKEE_API_KEY` to create SDK/plugin job examples in the configured scope.
