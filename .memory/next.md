# Next Work

## Immediate next slice
- Continue with `.prompt/074-script-runner-protocol-and-ui-binding.md`.
- Focus areas:
  1. Decide and implement Worker Tunnel protocol metadata for non-WASM script bindings, still binding only released immutable `script_versions` snapshots.
  2. Wire dispatcher/SDK adapter selection without letting scheduler Server execute user code.
  3. Update Web/docs to clearly show which script languages are executable by which worker capabilities.

## Current status
- Phase 073 slice completed. Rust SDK has an explicit opt-in local subprocess runner foundation with digest/release checks, default-deny policy enforcement, timeout/output caps, env clearing, and runtime-unavailable handling. It is not yet wired into Worker Tunnel dispatch protocol for non-WASM scripts.
