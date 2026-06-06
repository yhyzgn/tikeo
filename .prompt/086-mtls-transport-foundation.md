# 086 — mTLS transport foundation

## Context
Phase 085 added OIDC/SSO configuration and status metadata without requiring a live identity provider. The Phase 3 roadmap still has mTLS transport encryption open; Worker Tunnel and HTTP listeners currently run without TLS configuration.

## Objectives
1. Add mTLS/TLS configuration data shapes and operator-visible diagnostics without requiring real certificates in tests.
2. Keep current local development listeners unchanged by default.
3. Prepare Worker Tunnel and HTTP gateway for future rustls/tonic TLS wiring with fail-safe validation rules.

## Constraints
- Do not require certificate generation, OpenSSL, or network TLS smoke in CI.
- Do not break local `config/dev.toml` or SDK demo defaults.
- Preserve Worker outbound-only architecture; do not add Worker inbound ports.
- Preserve the `{ code, message, data }` envelope.

## Expected verification
- Targeted Rust tests for TLS/mTLS config defaults, status diagnostics, and invalid partial config validation.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`

## Completion notes
- Update `design/tikeo-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/087-*.md` before commit.
- Commit with Lore trailers and push.
