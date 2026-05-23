# 099 — Phase 3 API token scopes

## Context
Phase 098 added durable API token lifecycle endpoints, but tokens still inherited the full role permission set. The Phase 3 auth risk list still called out fine-grained token scopes as incomplete.

## Objectives
1. Let callers optionally create API tokens with `scopes` in `resource:action` form.
2. Validate requested token scopes against the current principal permissions before creation.
3. Persist scope metadata without storing plaintext token material, expose scopes in token metadata, and narrow effective permissions when authenticating with a scoped token.
4. Ensure an `admin` role bearer cannot bypass scoped-token restrictions.

## Constraints
- Preserve the HTTP envelope `{ code, message, data }`.
- Do not add new dependencies or store plaintext API tokens.
- Do not pull Phase 4 items back into Phase 3: Node.js SDK, K8s Helm, PowerJob migration, XXL-JOB migration.
- Do not claim multi-tenant namespace/app/worker-pool scope binding or token rotation/expiry policy complete.

## Expected verification
- `cargo test -p tikee-server api_token --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md`.
- Commit with Lore trailers and push.
