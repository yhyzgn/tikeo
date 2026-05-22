# 092 — Phase 3 OIDC callback skeleton

## Context
Phase 091 added alert channel delivery readiness/redaction without sending real notifications. Phase 3 OIDC/SSO still has only config/status metadata; real IdP login callback remains open.

## Objectives
1. Add the next smallest locally verifiable OIDC/SSO hardening slice, preferably login URL/callback request shape and readiness validation without requiring a live IdP.
2. Preserve local username/password login and existing session behavior.
3. Keep secrets redacted and responses in `{ code, message, data }`.

## Constraints
- Do not pull Phase 4 items back into Phase 3: Node.js SDK, K8s Helm, PowerJob migration, XXL-JOB migration.
- Do not require external IdP/network calls in tests.
- Avoid accepting unsigned/unverified identity tokens as real login.

## Expected verification
- Targeted tests for default/configured OIDC readiness or callback shape.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`
- Web validation if Web files change.

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/093-*.md` before commit.
- Commit with Lore trailers and push.
