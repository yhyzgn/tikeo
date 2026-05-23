# 115 — Phase 3 tenant scope lifecycle policy

## Goal
Add a safe destructive lifecycle policy for tenant namespace/app/worker-pool metadata now that create/list APIs and UI exist.

## Scope
- Add guarded DELETE routes for namespaces, apps, and worker pools.
- Reject namespace deletion while apps, worker pools, or jobs still reference it.
- Reject app deletion while worker pools or jobs still reference it.
- Allow worker-pool metadata deletion without touching online workers or job records.
- Add Web console delete actions with confirmation text explaining the non-empty rejection policy.

## Out of scope
- Implicit cascading deletion of jobs/scripts/workflows.
- OIDC identity-to-tenant mapping.
- Advanced tenant isolation policy editor.
