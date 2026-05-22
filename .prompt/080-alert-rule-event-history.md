# 080 — Alert rule API and deterministic event history

## Context
Phase 078 introduced the `AlertCondition::ScriptGovernanceFailure` condition shape, and Phase 079 materialized `script_execution_governance` failures into durable audit rows with `failure_reason` filtering. The Phase 3 roadmap still has the alert system incomplete: rule APIs, event ingestion, dedupe/silence, notification history, and recovery notifications are not first-class yet.

## Objectives
1. Add a bounded alert rule management/query API for the existing alert model without adding Swagger.
2. Materialize deterministic alert events/history for script governance failures from the same Server-side governance hook used by audit materialization.
3. Add basic dedupe/silence semantics suitable for tests and Web/API visibility; avoid external webhook smoke.
4. Preserve HTTP response envelopes as `{ code, message, data }` and keep relationships soft-only.

## Constraints
- Server must not execute user scripts.
- No database foreign keys.
- No external Slack/DingTalk/Feishu/PagerDuty calls in verification.
- Keep modules split by responsibility; do not grow large monolithic files.
- Do not add Swagger.

## Expected verification
- Targeted Rust tests for alert rule CRUD/history and governance event ingestion.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`
- `cd web && bun run typecheck && bun test && bun run build` if Web changes are made.

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/081-*.md` before commit.
- Commit with Lore trailers and push.
