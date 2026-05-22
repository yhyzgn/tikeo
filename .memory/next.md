# Next Work

## Immediate next slice
- Continue with `.prompt/073-phase3-script-sandbox-runner-implementations.md`.
- Focus areas:
  1. Implement first concrete non-WASM sandbox runner behind explicit opt-in.
  2. Preserve default-deny network/filesystem/secrets policy and released-version-only execution.
  3. Add timeout/output/runtime-unavailable tests and update docs.

## Current status
- Phase 072 completed. Script policy metadata is persisted on script rows and immutable version snapshots; HTTP rejects dangerous policy grants in this phase; Web displays/edits safe policy fields; Rust SDK has non-WASM runner abstractions that validate policy and refuse execution until a concrete sandbox runner is implemented. Web Vite/Rolldown chunk splitting removed the >500KB build warning.
