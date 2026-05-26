# Rust Worker Demo

Runnable demo for `sdks/rust/tikee`.

Dry-run configuration smoke test:

```bash
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Live Worker Tunnel mode:

```bash
TIKEE_WORKER_CONNECT=1 \
TIKEE_WORKER_ENDPOINT=http://127.0.0.1:9998 \
TIKEE_WORKER_INSTANCE_ID=rust-script-shell-local \
TIKEE_ENABLE_SCRIPT_SHELL=1 \
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
```

Environment variables:

- `TIKEE_WORKER_CONNECT` defaults to dry-run; set `1` to connect to the Worker Tunnel.
- `TIKEE_WORKER_ENDPOINT` defaults to `http://127.0.0.1:9998`.
- `TIKEE_WORKER_INSTANCE_ID` defaults to `rust-demo-instance`.
- `TIKEE_WORKER_NAMESPACE` / `TIKEE_WORKER_APP` default to `default` / `default`.
- `TIKEE_WORKER_CAPABILITIES` adds comma-separated Worker capabilities.
- `TIKEE_ENABLE_SCRIPT_SHELL=1` registers a container sandbox runner and advertises `script:shell`.
- `TIKEE_ENABLE_SCRIPT_PYTHON=1` advertises `script:python` with `python:3.13-alpine`.
- `TIKEE_ENABLE_SCRIPT_NODE=1` advertises `script:node` with `node:24-alpine`.
- `TIKEE_ENABLE_SCRIPT_POWERSHELL=1` advertises `script:powershell`.
- `TIKEE_<LANG>_IMAGE` overrides the container image, for example `TIKEE_SHELL_IMAGE=alpine:3.20`.

Script tasks are executed only through `ContainerScriptRunner`, which starts an isolated container with `--network=none`, read-only root filesystem, bounded memory, and script content from the released immutable snapshot. The Java demo intentionally stays SDK-processor-only and does not execute script bindings.

For deployment bootstrap, source `deploy/worker/identity.env.example` before running this demo to verify stable logical-worker identity metadata.
