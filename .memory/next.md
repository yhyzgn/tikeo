# Next Work

## Immediate next slice
- Continue with `.prompt/071-phase3-script-release-pointer-and-worker-version-binding.md`.
- Focus areas:
  1. Add explicit script release/publish pointer instead of dispatching from mutable current script rows.
  2. Add rollback API and approval state transitions around immutable versions.
  3. Make Worker dispatch always bind to a released immutable version snapshot.

## Current status
- Phase 070 completed and verified. WASM binding now carries SHA-256 integrity metadata and optional version snapshot hooks; Rust SDK verifies digest before execution; Web displays sandbox policy/integrity metadata; Java Gradle 10 deprecation warnings are cleared.
