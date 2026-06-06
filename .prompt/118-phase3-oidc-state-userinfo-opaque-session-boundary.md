# 118 — Phase 3 OIDC state and UserInfo opaque-session boundary

## Goal
Correct OIDC integration to match tikeo's auth model: external provider data may identify a user, but tikeo login state remains an opaque token persisted in `auth_sessions` and cached through moka.

## Scope
- Persist generated OIDC `state` values as hashed one-time records.
- Include only server-generated state in authorization URLs.
- Consume callback state exactly once and reject replay/expired/unknown state.
- Exchange authorization code for a provider access token, discover the UserInfo endpoint, and fetch external subject metadata.
- Fail closed before session issuance until external subject -> local user/role/tenant mapping is implemented.
- Remove the previously introduced provider-token-as-session route from current code and roadmap language.

## Out of scope
- Using provider tokens as tikeo login state.
- Mapping external subjects to local users/roles/tenants.
- Issuing local opaque sessions from OIDC identity.
- Live external IdP smoke tests.
