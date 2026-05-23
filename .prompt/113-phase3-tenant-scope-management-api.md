# 113 — Phase 3 tenant scope management API foundation

## Goal
Add a locally verifiable management API foundation for tenant namespace, app, and worker-pool metadata without introducing database foreign keys.

## Scope
- Persist worker-pool metadata as soft links to namespace/app identifiers.
- Add repository operations to create/list namespaces, apps, and worker pools idempotently.
- Add authenticated management routes for `/api/v1/namespaces`, `/api/v1/apps`, and `/api/v1/worker-pools`.
- Seed tenant read/manage permissions for RBAC.
- Represent the new management routes in OpenAPI.

## Out of scope
- Full web UI for tenant/app/worker-pool management.
- OIDC identity-to-tenant mapping.
- Destructive delete flows and cascade cleanup policy.
