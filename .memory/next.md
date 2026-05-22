# Next Work

## Immediate next slice
- Continue with `.prompt/073-phase3-script-sandbox-runner-implementations.md` unless user reports more UI/runtime issues.
- If more Web pages show endless loading, first inspect unstable dependency objects passed into hooks/effects.

## Current status
- Phase 072 completed and pushed. Script policy metadata/snapshots, Web chunk splitting, and Rust SDK non-WASM runner abstraction are done.
- Audit log page loading loop was fixed by stabilizing URL query defaults; Web typecheck/test/build passed.
