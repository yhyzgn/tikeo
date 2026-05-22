# 085 — OIDC/SSO foundation

## Context
Phase 084 added deterministic HTTP trace-id propagation and local tracing spans without requiring an external OTLP collector. The Phase 3 roadmap still has OIDC/SSO integration open; current auth remains development username/password session/token flow.

## Objectives
1. Add an OIDC/SSO configuration and API contract foundation without requiring a live identity provider in tests.
2. Expose operator-visible auth mode/status metadata so Web/clients can distinguish local auth from future OIDC-backed login.
3. Keep current development admin login working unchanged and preserve RBAC/session behavior.

## Constraints
- Do not require Keycloak/Auth0/Okta/network smoke in verification.
- Do not store provider secrets in Web or logs.
- Preserve the `{ code, message, data }` envelope.
- Keep modules split by responsibility; avoid putting OIDC parsing into route handlers directly.

## Expected verification
- Targeted Rust tests for OIDC config/status behavior and local auth compatibility.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/086-*.md` before commit.
- Commit with Lore trailers and push.
