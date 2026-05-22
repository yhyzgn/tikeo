# Next Work

## Immediate next slice
- Continue with `.prompt/078-script-governance-audit-alerting.md`.
- Focus areas:
  1. Promote `script_execution_governance` instance logs into audit/alert inputs without adding foreign keys.
  2. Add query/filter and Web highlighting for script governance failure classes.
  3. Keep Server as metadata dispatcher only; script execution remains Worker-side opt-in from released immutable snapshots.

## Current status
- Phase 077 added governance failure classification for dispatch-side script gating and Worker-side script result failures.
- Rust SDK result messages now include stable `failure_class` JSON for recognized script runner failures.
- Server stores recognized governance result classes as instance logs.

## SDK naming note
- Rust SDK is `sdks/rust/tikee` / crate `tikee`. Java core SDK module/artifact is `tikee`; Java package prefix remains `com.yhyzgn.tikee`.
