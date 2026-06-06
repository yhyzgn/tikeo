# 082 — Alert notification summary

## Context
Phase 081 added deterministic alert recovery transitions on top of alert rule and event history storage. The alert subsystem still needs operator-friendly notification history summaries and rollups, but verification must remain local and deterministic.

## Objectives
1. Add notification history summary queries over the existing alert event stream.
2. Keep recovery/firing/suppressed/silenced event history queryable by rule, resource, and failure class.
3. Preserve the standard API envelope and soft-link-only storage model.

## Constraints
- No external webhook smoke.
- No database foreign keys.
- Keep modules split by responsibility.

## Expected verification
- Targeted Rust tests for notification summary and history filters.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`

## Completion notes
- Update `design/tikeo-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/083-*.md` before commit.
- Commit with Lore trailers and push.
