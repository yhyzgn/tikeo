# 114 — Phase 3 tenant scope management UI

## Goal
Expose the tenant namespace/app/worker-pool management API in the Web console so operators can create and inspect scope metadata without direct API calls.

## Scope
- Add typed Web API client methods for namespaces, apps, and worker pools.
- Add a governed `/scopes` route and menu entry requiring `tenants:read`.
- Provide create forms for namespace, app, and worker pool guarded by `tenants:manage`.
- Show current namespace/app/worker-pool metadata in focused tables.
- Keep UI read/create-only; destructive lifecycle policy remains out of scope.

## Out of scope
- Delete/rename/cascade scope lifecycle.
- OIDC identity-to-tenant mapping UI.
- Advanced tenant isolation policy editor.
