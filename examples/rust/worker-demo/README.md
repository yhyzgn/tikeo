# Rust Worker Demo

Runnable demo for `sdks/rust/tikee`, aligned with the Java manual acceptance scopes.

Direct live Worker Tunnel mode, same default behavior as the Java demos:

```bash
# Start tikee first, for example from the repository root:
# ./scripts/dev.sh

cd examples/rust/worker-demo
cargo run
```

By default this connects to `http://127.0.0.1:9998`, registers under `dev-alpha/orders`, advertises the structured SDK processors, SQL plugin processor, and the same script language matrix as the Java demos.

Dry-run configuration smoke test:

```bash
TIKEE_WORKER_DRY_RUN=1 cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Defaults:

- namespace/app: `dev-alpha/orders`
- client instance id: `rust-worker-demo-local`
- worker pool label: `rust-blue`
- SDK processors: `demo.echo`, `demo.context`, `demo.bytes`, `demo.heartbeat`, `demo.fail`
- plugin processor: `type=sql`, `processorName=billing.sql-sync`
- script runners: `shell`, `python`, `javascript`, `typescript`, `powershell`, `php`, `groovy`, `rhai`
- default script backend resolution: Java-parity `auto` -> `srt` for shell/python/powershell/php/groovy/rhai, `deno` for JavaScript/TypeScript
- tags: `rust`, `manual-demo`

Environment variables:

- `TIKEE_WORKER_DRY_RUN=1` switches to dry-run mode without opening the Worker Tunnel.
- `TIKEE_WORKER_CONNECT=0` is also accepted as a compatibility dry-run switch.
- `TIKEE_WORKER_ENDPOINT` defaults to `http://127.0.0.1:9998`.
- `TIKEE_WORKER_CLIENT_INSTANCE_ID` / `TIKEE_WORKER_INSTANCE_ID` override the stable client instance id.
- `TIKEE_WORKER_NAMESPACE` / `TIKEE_WORKER_APP` override the default `dev-alpha/orders` scope.
- `TIKEE_WORKER_POOL` overrides the default `rust-blue` worker pool label.
- `TIKEE_WORKER_SDK_PROCESSORS` overrides the comma-separated SDK processor list.
- `TIKEE_ENABLE_PLUGIN_SQL` defaults to enabled; set `TIKEE_ENABLE_PLUGIN_SQL=0` to stop advertising the SQL plugin processor.
- `TIKEE_PLUGIN_SQL_TYPE` and `TIKEE_PLUGIN_SQL_PROCESSOR` override the default `sql` / `billing.sql-sync` structured plugin fields.
- `TIKEE_WORKER_SCRIPT_SANDBOX` supports `auto`, `srt`, `deno`, `v8`, `wasmtime`, `wasmedge`, `docker`, `podman`, and `custom`; `container` is accepted as `docker`.
- `TIKEE_ENABLE_SCRIPT_<LANG>=0` disables a default language, for example `TIKEE_ENABLE_SCRIPT_RHAI=0`.
- `TIKEE_<LANG>_IMAGE` overrides the container image when `TIKEE_WORKER_SCRIPT_SANDBOX=docker` or `podman`.

Execution note: Rust demo can execute script tasks through Docker/Podman container runners. For Java-parity `srt`, `deno`, `v8`, `wasmtime`, `wasmedge`, or `custom` backends, the demo advertises the structured capability and fails closed with a clear unavailable-backend error until a matching Rust runner is configured.
