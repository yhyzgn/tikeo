# 131 - P1 OIDC tenant scope mapping

## Goal
Make OIDC usable for enterprise tenant/app scoped access without granting unmapped external identities broad sessions.

## Scope
- Add tenant-governed OIDC identity mapping APIs: list, upsert, delete.
- Map `issuer + subject` to a local username plus optional namespace/app/worker-pool scope bindings.
- Keep OIDC callback fail-closed for unmapped subjects.
- Return scope metadata in `AuthSession`, matching `/auth/me`, so UI can immediately represent tenant-limited sessions.
- Extend Web Scopes page to manage OIDC mappings next to namespace/app/Worker Pool metadata.

## Validation target
- Server targeted OIDC tests cover mapping API and existing callback fail-closed/session behavior.
- Web typecheck and ScopesPage test cover OIDC mapping UI surface.
