# tikee

Rust Worker SDK for active outbound tikee Worker Tunnel connections.

Standalone validation from repository root:

```bash
cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features
```

This crate is self-contained for publishing: it vendors its Worker Tunnel protobuf definition under `proto/` and does not depend on server workspace crates.

Registration model: the client may provide `client_instance_id` only as a stable hint; authoritative `worker_id` is assigned by the tikee in `WorkerRegistered`. Dispatch routing uses structured capabilities only: `sdk_processors`, `script_runners`, and `plugin_processors`.

## Dynamic script runners

Dynamic script bindings are executed only when a Worker explicitly registers a matching structured script runner. The Server never executes script content, and production Workers must not execute scripts as bare host subprocesses.

- Shell: call `config.add_script_runner("shell", "container")` and register `ContainerScriptRunner::new(ScriptRunnerKind::Shell, "alpine:3.20")`.
- Python: call `config.add_script_runner("python", "container")` and use a Python image/runner.
- JavaScript/TypeScript: call `config.add_script_runner("javascript", "deno")` / `config.add_script_runner("typescript", "deno")` and use a Deno-capable runner or container image.
- PowerShell: call `config.add_script_runner("powershell", "container")` and use a PowerShell image/runner.
- Rhai: call `config.add_script_runner("rhai", "container")` and use a Rhai-capable image/runner.

`ContainerScriptRunner` invokes a Docker-compatible CLI from the Worker process with a default-deny boundary: `--network=none`, `--read-only`, no host mounts, stdin script content from the released immutable snapshot, SHA-256 validation before spawn, tikee metadata env, and only policy-whitelisted env vars. Deploy script-capable Workers as dedicated Docker/K8s pools with container-runtime access; do not give that access to the tikee Server.

Optional live smoke when Docker is available:

```rust,no_run
use tikee::{ContainerScriptRunner, ScriptRunnerKind, ScriptRunnerRegistry, WorkerConfig};

let mut config = WorkerConfig::local("http://127.0.0.1:9998", "rust-worker-demo");
let mut runners = ScriptRunnerRegistry::new();
runners.register(ContainerScriptRunner::new(ScriptRunnerKind::Shell, "alpine:3.20"));
config.add_script_runner("shell", "container");
```

Then call `WorkerSession::process_next_with_script_runners(&processor, &runners)` from a dedicated sandbox-capable Worker.

Sandbox rule: non-WASM scripts must run in a sandbox boundary. Prefer `ContainerScriptRunner` or a stronger runtime such as Firecracker/gVisor/Kata. `LocalSubprocessScriptRunner` exists only for SDK tests and isolated development diagnostics; do not register it in production workers.
