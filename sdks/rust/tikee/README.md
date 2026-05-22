# tikee

Rust Worker SDK for active outbound tikee Worker Tunnel connections.

Standalone validation from repository root:

```bash
cargo test --manifest-path sdks/rust/tikee/Cargo.toml --all-features
```

This crate is self-contained for publishing: it vendors its Worker Tunnel protobuf definition under `proto/` and does not depend on server workspace crates.

Registration model: the client may provide `client_instance_id` only as a stable hint; authoritative `worker_id` is assigned by the tikee in `WorkerRegistered`.

## Dynamic script runners

Dynamic script bindings are executed only when a Worker explicitly registers a matching runner and advertises the matching capability to tikee. The Server never executes script content.

- Shell: advertise `script:shell`, register `ContainerScriptRunner::new(ScriptRunnerKind::Shell, "alpine:3.20")` or another approved runner.
- Python: advertise `script:python`, use a Python image/runner.
- Node.js: advertise `script:node`, use a Node image/runner.
- PowerShell: advertise `script:powershell`, use a PowerShell image/runner.
- Rhai: advertise `script:rhai`, use a Rhai-capable image/runner.
- Controlled shared pools may advertise `script:*`; avoid `*` except in isolated development pools.

`ContainerScriptRunner` invokes a Docker-compatible CLI from the Worker process with a default-deny boundary: `--network=none`, `--read-only`, no host mounts, stdin script content from the released immutable snapshot, SHA-256 validation before spawn, tikee metadata env, and only policy-whitelisted env vars. Deploy script-capable Workers as dedicated Docker/K8s pools with container-runtime access; do not give that access to the tikee Server.

Optional live smoke when Docker is available:

```rust,no_run
use tikee::{ContainerScriptRunner, ScriptRunnerKind, ScriptRunnerRegistry};

let mut runners = ScriptRunnerRegistry::new();
runners.register(ContainerScriptRunner::new(ScriptRunnerKind::Shell, "alpine:3.20"));
```

Then call `WorkerSession::process_next_with_script_runners(&processor, &runners)` from a Worker that registered the corresponding `script:shell` capability.
