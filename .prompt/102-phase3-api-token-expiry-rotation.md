# 102 — Phase 3 API token expiry and rotation policy

## Context
Phase 098 added durable API token create/list/revoke and Phase 099 added `resource:action` scopes, but the Phase 3 risk list still called out API token rotation/expiry policy as incomplete.

## Objectives
1. Add configurable API token TTL policy under `auth.api_tokens` with default/min/max TTL seconds.
2. Let token creation request a bounded `expires_in_seconds`; reject values outside policy.
3. Add `POST /api/v1/auth/api-tokens/{id}/rotate` so operators can replace a token without exposing old plaintext.
4. Preserve existing token scopes during rotation and revoke the old token immediately.
5. Keep token storage hash-only and maintain standard `{code,message,data}` HTTP envelopes.

## Verification
- RED observed first: `rtk cargo test -p tikee-server api_token_policy --all-features` failed because token TTL ignored the request and too-long TTL returned 200.
- Targeted green: `rtk cargo test -p tikee-server api_token --all-features`.
- Config default coverage: `rtk cargo test -p tikee-config default_auth_config --all-features`.

## Remaining boundaries
- This does not implement multi-tenant namespace/app/worker-pool scope binding.
- This does not introduce plaintext token persistence or external secret storage.
