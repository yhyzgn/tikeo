# Go Worker demo

Go Worker demo aligned with the Java manual acceptance scopes.

Direct live Worker Tunnel mode, same default behavior as the Java and Rust demos:

```bash
# Start tikeo first, for example from the repository root:
# ./scripts/dev.sh

cd examples/go/worker-demo
go run .
```

By default this connects to `http://127.0.0.1:9998`, registers under `dev-alpha/orders`, advertises the structured SDK processors, SQL plugin processor, and the same script language matrix as the Java demos.

Build/test:

```bash
go test ./...
```

Dry-run configuration smoke test:

```bash
TIKEO_WORKER_DRY_RUN=1 go run .
```

Defaults:

- namespace/app: `dev-alpha/orders`
- client instance id: `go-worker-demo-local`
- worker pool label: `go-blue`
- SDK processors: `demo.echo`, `demo.context`, `demo.bytes`, `demo.heartbeat`, `demo.fail`
- plugin processor: `type=sql`, `processorName=billing.sql-sync`
- script runners: `shell`, `python`, `javascript`, `typescript`, `powershell`, `php`, `groovy`, `rhai`
- default script backend resolution: Java-parity `srt` for shell/python/powershell/php/groovy/rhai, `deno` for JavaScript/TypeScript
- tags: `go`, `manual-demo`

Environment variables:

- `TIKEO_WORKER_DRY_RUN=1` switches to dry-run mode without opening the Worker Tunnel.
- `TIKEO_WORKER_CONNECT=0` is also accepted as a compatibility dry-run switch.
- `TIKEO_WORKER_ENDPOINT` defaults to `http://127.0.0.1:9998`.
- `TIKEO_WORKER_CLIENT_INSTANCE_ID` overrides the stable client instance id.
- `TIKEO_WORKER_NAMESPACE` / `TIKEO_WORKER_APP` override the default `dev-alpha/orders` scope.
- `TIKEO_WORKER_POOL` overrides the default `go-blue` worker pool label.
- `TIKEO_WORKER_SDK_PROCESSORS` overrides the comma-separated SDK processor list.
- `TIKEO_ENABLE_PLUGIN_SQL` defaults to enabled; set `TIKEO_ENABLE_PLUGIN_SQL=0` to stop advertising the SQL plugin processor.
- `TIKEO_PLUGIN_SQL_TYPE` and `TIKEO_PLUGIN_SQL_PROCESSOR` override the default `sql` / `billing.sql-sync` structured plugin fields.
- `TIKEO_WORKER_SCRIPT_LANGUAGES` defaults to all Java-parity demo languages; set it to a comma-separated list to override advertised/executable script runners.
- `TIKEO_ENABLE_SCRIPT_<LANG>=0` disables a default language, for example `TIKEO_ENABLE_SCRIPT_PHP=0`.
- `TIKEO_WORKER_SCRIPT_SANDBOX` overrides the advertised sandbox backend label for all languages.
- `TIKEO_MANAGEMENT_CREATE_EXAMPLES=1` uses `TIKEO_HTTP_URL` and `TIKEO_API_KEY` to create SDK/plugin job examples in the configured scope.

Execution note: Go demo advertises Java-parity `srt`/`deno` script backends and fails closed for script execution until a real Go sandbox runner is configured. It does not expose any non-Java sandbox label.
