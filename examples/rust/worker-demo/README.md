# Rust Worker Demo

Runnable demo for `sdks/rust/tikee`, aligned with the Java manual acceptance scopes.

Dry-run configuration smoke test:

```bash
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Live Worker Tunnel mode:

```bash
TIKEE_WORKER_CONNECT=1 \
TIKEE_WORKER_ENDPOINT=http://127.0.0.1:9998 \
TIKEE_WORKER_CLIENT_INSTANCE_ID=rust-worker-demo-local \
TIKEE_ENABLE_SCRIPT_SHELL=1 \
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Defaults:

- namespace/app: `dev-alpha/orders`
- client instance id: `rust-worker-demo-local`
- SDK processors: `demo.echo`, `demo.context`, `demo.bytes`, `demo.heartbeat`, `demo.fail`
- tags: `rust`, `manual-demo`

Environment variables:

- `TIKEE_WORKER_CONNECT` defaults to dry-run; set `1` to connect to the Worker Tunnel.
- `TIKEE_WORKER_ENDPOINT` defaults to `http://127.0.0.1:9998`.
- `TIKEE_WORKER_CLIENT_INSTANCE_ID` / `TIKEE_WORKER_INSTANCE_ID` override the stable client instance id.
- `TIKEE_WORKER_NAMESPACE` / `TIKEE_WORKER_APP` override the default `dev-alpha/orders` scope.
- `TIKEE_WORKER_SDK_PROCESSORS` overrides the comma-separated SDK processor list.
- `TIKEE_ENABLE_PLUGIN_SQL=1` advertises structured plugin processor `type=sql`, `processorName=billing.sql-sync`.
- `TIKEE_ENABLE_SCRIPT_SHELL=1`, `TIKEE_ENABLE_SCRIPT_PYTHON=1`, `TIKEE_ENABLE_SCRIPT_NODE=1`, `TIKEE_ENABLE_SCRIPT_RHAI=1`, or `TIKEE_ENABLE_SCRIPT_POWERSHELL=1` register structured script runners.
- `TIKEE_<LANG>_IMAGE` overrides the container image, for example `TIKEE_SHELL_IMAGE=alpine:3.20`.

Script tasks are executed only through `ContainerScriptRunner`, which starts an isolated container with `--network=none`, read-only root filesystem, bounded memory, and script content from the released immutable snapshot.
