# Next Work

## Immediate next slice
- Continue with `.prompt/079-script-governance-audit-materialization.md`.
- Focus areas:
  1. Materialize `script_execution_governance` events into audit/governance query surfaces without foreign keys.
  2. Keep instance log compatibility and Web highlighting behavior stable.
  3. Preserve Server as metadata dispatcher only; script execution remains Worker-side opt-in from released immutable snapshots.

## Current status
- Phase 078 parsed governance log JSON into explicit API fields, added a governance-only log filter, added Web instance-log highlighting, and introduced `script_governance_failure` alert condition shape.
- Interruption fix completed: the dev workflow editor/API boundary now normalizes stale edge condition aliases (`success`/`failed`) to canonical `on_success`/`on_failure`, and dev seed data uses `on_success`.

## SDK naming note
- Rust SDK is `sdks/rust/tikee` / crate `tikee`. Java core SDK module/artifact is `tikee`; Java package prefix remains `com.yhyzgn.tikee`.
