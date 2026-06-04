# Rust Worker Demo

Runnable demo for `sdks/rust/tikee`, aligned with the Java manual acceptance scopes.

Direct live Worker Tunnel mode, same default behavior as the Java demos:

```bash
# Start tikee first, for example from the repository root:
# ./scripts/dev.sh

cd examples/rust/worker-demo
cargo run
```

By default this connects to `http://127.0.0.1:9998`, registers under `dev-alpha/orders`, advertises the structured SDK processors and the SQL plugin processor, and should appear in the Worker cluster page as `rust-worker-demo-local`.

Dry-run configuration smoke test:

```bash
TIKEE_WORKER_DRY_RUN=1 cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Explicit live Worker Tunnel example with script runner advertisement:

```bash
TIKEE_WORKER_ENDPOINT=http://127.0.0.1:9998 \
TIKEE_WORKER_CLIENT_INSTANCE_ID=rust-worker-demo-local \
TIKEE_ENABLE_SCRIPT_SHELL=1 \
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Defaults:

- namespace/app: `dev-alpha/orders`
- client instance id: `rust-worker-demo-local`
- worker pool label: `rust-blue`
- SDK processors: `demo.echo`, `demo.context`, `demo.bytes`, `demo.heartbeat`, `demo.fail`
- plugin processor: `type=sql`, `processorName=billing.sql-sync`
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
- `TIKEE_ENABLE_SCRIPT_SHELL=1`, `TIKEE_ENABLE_SCRIPT_PYTHON=1`, `TIKEE_ENABLE_SCRIPT_NODE=1`, `TIKEE_ENABLE_SCRIPT_RHAI=1`, or `TIKEE_ENABLE_SCRIPT_POWERSHELL=1` register structured script runners.
- `TIKEE_<LANG>_IMAGE` overrides the container image, for example `TIKEE_SHELL_IMAGE=alpine:3.20`.

Script tasks are executed only through `ContainerScriptRunner`, which starts an isolated container with `--network=none`, read-only root filesystem, bounded memory, and script content from the released immutable snapshot.
