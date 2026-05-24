# 116 — Phase 3 OIDC token exchange boundary

## Goal
Move OIDC callback from shape-only validation to a real authorization-code token exchange boundary while preserving fail-closed identity/session semantics.

## Scope
- Exchange callback `code` against the configured provider token endpoint using client credentials.
- Keep the standard `{ code, message, data }` envelope for failures.
- Require `id_token` in the token response before any future verification step can proceed.
- Fail closed after exchange until JWKS/signature/claims validation and user mapping are implemented.
- Cover the exchange path with a local mock IdP token endpoint test.

## Out of scope
- JWKS discovery/cache and signature validation.
- Nonce/state persistence.
- OIDC user/role/tenant mapping.
- Session issuance from IdP identity.
