# Phase 122 / P0 OIDC mapped opaque session issuance

## Goal
Complete the P0 OIDC path so an external `(issuer, subject)` identity can be mapped to a local tikee user/role/scope and receive a local opaque tikee session.

## Decisions
- Local login state remains 48-character opaque base62 bearer tokens persisted in `auth_sessions` and cached by moka; provider/JWT tokens never become local session state.
- JWT/id_token/provider access token must not become tikee local session state.
- OIDC mappings are soft links: no database foreign keys.
- Optional OIDC scope bindings are encoded as session metadata and returned by `/api/v1/auth/me`.

## Implementation
- Added `oidc_identities` entity, migration, SQLite compatibility, and repository.
- Added OIDC callback completion module for mapping lookup, local user lookup, session issuance, and audit logging.
- Split session metadata encoding into its own module so API-token and OIDC scoped sessions share binding decode without classifying OIDC sessions as API tokens.

## Verification
- `rtk cargo test -p tikee-server oidc_callback_issues_opaque_session_for_mapped_external_subject --all-features`
- `rtk cargo test -p tikee-server oidc --all-features`
- `rtk cargo clippy -p tikee-storage --all-targets --all-features -- -D warnings`
- `rtk cargo clippy -p tikee-server --all-targets --all-features -- -D warnings`
- `rtk cargo fmt --all -- --check`
