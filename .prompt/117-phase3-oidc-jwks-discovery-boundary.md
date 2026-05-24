# 117 — Phase 3 OIDC JWKS discovery boundary

## Goal
Advance the OIDC callback from token exchange to provider discovery/JWKS retrieval while preserving fail-closed session issuance.

## Scope
- Fetch the OpenID Provider Configuration document from the configured issuer.
- Require and validate a `jwks_uri` value.
- Fetch the JWKS document and reject empty key sets.
- Continue failing closed before accepting the `id_token` because signature/claims validation is not yet implemented.
- Extend the local mock IdP test to prove token, discovery, and JWKS endpoints are all hit exactly once.

## Out of scope
- JWT header parsing and `kid` key selection.
- Signature, issuer, audience, nonce, and expiry validation.
- OIDC user/role/tenant mapping.
- Session issuance from IdP identity.
