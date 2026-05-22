# 087 — Script approval policy gates

## Context
Phase 086 added TLS/mTLS configuration and diagnostics without requiring certificates in tests. Phase 3 still has script approval/policy gates open: full production-grade multi-level approval and signing can wait, but the current script policy engine needs a deterministic approval gate foundation.

## Objectives
1. Add a minimal script approval/policy gate that records why a script release is blocked when dangerous capabilities are requested.
2. Keep existing create/update/publish/rollback flows working for safe policies.
3. Make policy gate outcomes queryable through existing script APIs or audit logs without adding foreign keys.

## Constraints
- No new external policy engine dependency yet.
- No real signing infrastructure or KMS in tests.
- Server must not execute user scripts.
- Preserve the `{ code, message, data }` envelope.

## Expected verification
- Targeted Rust tests for safe policy publish and blocked dangerous policy publish/update behavior.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/088-*.md` before commit.
- Commit with Lore trailers and push.
