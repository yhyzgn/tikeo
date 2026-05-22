# 065 — Phase 3 frontend route meta, lazy loading, 401/403, URL query governance

## Context
Web dangerous/mutating actions now use permission-aware rendering and confirmation gates. The remaining frontend governance roadmap item is route metadata, lazy loading, unified auth-error handling, and URL query persistence.

## Required next work
1. Introduce a route metadata table/source of truth for path, label, permission resource/action, and layout/menu behavior.
2. Convert heavy route pages to lazy-loaded chunks where practical without breaking guards.
3. Unify 401/403 handling in the API client/UI: expired session should clear token and redirect/login prompt; forbidden should show consistent 403 feedback.
4. Persist list filters/page state in URL query params for key pages (audit/jobs/workflows/scripts where feasible).
5. Preserve existing route guards and API envelope `{ code, message, data }`; no database schema changes expected.
6. Update design/.memory/session logs and create `.prompt/066-*.md`.
7. Run full verification, commit with Lore-style trailers, and push.
