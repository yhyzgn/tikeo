# Next Work

## Immediate next slice
- Continue with `.prompt/072-phase3-script-policy-engine-and-sandbox-runners.md`.
- Focus areas:
  1. Add explicit script policy metadata for capabilities/resources/network/filesystem/secrets.
  2. Enforce default-deny policy validation and approval gates before release/execution.
  3. Start Worker-side non-WASM runner abstraction while preserving Server no-user-code execution.

## Current status
- Phase 071 completed and verified. Scripts now have released immutable version pointers; publish/rollback APIs update soft release pointers and audit actions; WASM dispatch binds to released `script_versions` bytes/SHA-256/version metadata and fails closed without a release pointer.
