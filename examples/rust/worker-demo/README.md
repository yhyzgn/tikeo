# Rust Worker Demo

Runnable demo for `sdks/rust/tikeo`, aligned with the Java manual acceptance scopes.

Direct live Worker Tunnel mode, same default behavior as the Java demos:

```bash
# Start tikeo first, for example from the repository root:
# ./scripts/dev.sh

cd examples/rust/worker-demo
cargo run
```

By default this connects to `http://127.0.0.1:9998`, registers under `dev-alpha/orders`, advertises the structured SDK processors, SQL plugin processor, and the same script language matrix as the Java demos.

Dry-run configuration smoke test:

```bash
TIKEO_WORKER_DRY_RUN=1 cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
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

- `TIKEO_WORKER_DRY_RUN=1` switches to dry-run mode without opening the Worker Tunnel.
- `TIKEO_WORKER_CONNECT=0` is also accepted as a compatibility dry-run switch.
- `TIKEO_WORKER_ENDPOINT` defaults to `http://127.0.0.1:9998`.
- `TIKEO_WORKER_CLIENT_INSTANCE_ID` / `TIKEO_WORKER_INSTANCE_ID` override the stable client instance id.
- `TIKEO_WORKER_NAMESPACE` / `TIKEO_WORKER_APP` override the default `dev-alpha/orders` scope.
- `TIKEO_WORKER_POOL` overrides the default `rust-blue` worker pool label.
- `TIKEO_WORKER_SDK_PROCESSORS` overrides the comma-separated SDK processor list.
- `TIKEO_ENABLE_PLUGIN_SQL` defaults to enabled; set `TIKEO_ENABLE_PLUGIN_SQL=0` to stop advertising the SQL plugin processor.
- `TIKEO_PLUGIN_SQL_TYPE` and `TIKEO_PLUGIN_SQL_PROCESSOR` override the default `sql` / `billing.sql-sync` structured plugin fields.
- `TIKEO_WORKER_SCRIPT_SANDBOX` supports `auto`, `srt`, `deno`, `v8`, `wasmtime`, `wasmedge`, `docker`, `podman`, and `custom`; `container` is accepted as `docker`.
- `TIKEO_SANDBOX_AUTO_INSTALL=0` disables automatic sandbox tool installation. With the default setting, the demo checks and installs SRT, ripgrep, Deno, and Rhai tools as needed.
- `TIKEO_WORKER_STATE_DIR` overrides the managed sandbox tool install root.
- `TIKEO_ENABLE_SCRIPT_<LANG>=0` disables a default language, for example `TIKEO_ENABLE_SCRIPT_RHAI=0`.
- `TIKEO_<LANG>_IMAGE` overrides the container image only when `TIKEO_WORKER_SCRIPT_SANDBOX=docker` or `podman` is explicitly selected.

Execution note: Rust demo now follows the Java lightweight auto path. `auto` uses SRT for shell/python/powershell/php/groovy/rhai and Deno for JavaScript/TypeScript, with automatic tool resolution/installation before structured capabilities are advertised. Docker/Podman are heavier explicit backends and are never selected by default.
