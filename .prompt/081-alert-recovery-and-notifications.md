# 081 — Alert recovery and notification history

## Context
Phase 080 added persistent alert rules and alert event history, and script governance failures now materialize into both audit rows and alert history entries. The alert subsystem still lacks recovery transitions, notification history shaping, and cleaner operator-facing management surfaces.

## Objectives
1. Add deterministic recovery/resolve transitions on the existing alert event stream.
2. Add notification history summaries without external webhook smoke.
3. Keep the API envelope `{ code, message, data }` and soft-only relationships.

## Constraints
- No external Slack/DingTalk/Feishu/PagerDuty verification.
- No database foreign keys.
- Keep modules split by responsibility; avoid large monolithic files.

## Expected verification
- Targeted Rust tests for recovery transitions and history querying.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`

## Completion notes
- Update `design/tikeo-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/082-*.md` before commit.
- Commit with Lore trailers and push.
