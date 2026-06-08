# Source size cleanup plan — 2026-06-08

## Goal
Bring normal project source files back under the <=1500-line rule without changing runtime behavior.

## Current audit target
Normal source directories exclude generated dependencies and build output (`target`, `node_modules`, `dist`). Initial known over-limit files:

- `crates/tikeo-server/src/tunnel/dispatcher.rs`
- `crates/tikeo-storage/src/repository.rs`
- `crates/tikeo-storage/src/repository/workflow.rs`
- `crates/tikeo-server/src/http/tests/part_03.rs` (test source)
- possible Web aggregate/generated-like files after focused audit

## Cleanup strategy
1. Add a source-size audit script and test first so future regressions fail clearly.
2. Split implementation files by responsibility, preserving public APIs and test behavior.
3. Treat test files separately: split large test modules only when straightforward; otherwise document a narrow test-only boundary in the audit script.
4. Run full Rust/Web validation after every source split.

## Non-goals
- No schema changes.
- No behavioral rewrites.
- No new dependencies.
- No worker networking model changes.
