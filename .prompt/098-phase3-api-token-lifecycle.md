# 098 — Phase 3 API token lifecycle foundation

## Context
Phase 3 RBAC was previously limited to username/password sessions and `resource/action` permissions, while the closeout risks still called out API token lifecycle as a missing production auth foundation.

## Objectives
1. Add authenticated API token create/list/revoke endpoints under `/api/v1/auth/api-tokens`.
2. Reuse the durable DB-backed session store so token hashes, expiry, cache invalidation, and bearer authentication remain centralized.
3. Return raw token material only at creation time; list responses must expose metadata only and never `token_hash`.
4. Audit API token create/revoke operations.

## Constraints
- Preserve the HTTP envelope `{ code, message, data }`.
- Do not add new dependencies or introduce plaintext token persistence.
- Do not pull Phase 4 items back into Phase 3: Node.js SDK, K8s Helm, PowerJob migration, XXL-JOB migration.
- Do not claim fine-grained token scopes, rotation policy, OIDC federation, or multi-tenant scope binding complete.

## Expected verification
- `cargo test -p tikee-server api_token_lifecycle_creates_lists_authenticates_and_revokes --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md`.
- Commit with Lore trailers and push.
