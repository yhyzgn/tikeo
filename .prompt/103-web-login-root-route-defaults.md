# 103 — Web login bypass and root dashboard route

## Context
The user reported two Web UX issues: when a valid session token exists, directly visiting the login page should skip it; directly visiting the domain root should have a default route, using the dashboard/overview page.

## Objectives
1. Add an explicit root `/` route that redirects to the dashboard route metadata path.
2. Make `LoginPage` detect an existing auth token and replace-navigate to the dashboard route.
3. Preserve normal post-login navigation, including returning to the originally requested protected route when available.
4. Keep the route target tied to `ROUTE_META.dashboard.path` instead of duplicating string literals.

## Verification
- RED observed first: `rtk bash -lc 'cd web && bun test src/pages/__tests__/RouteAuth.test.tsx'` failed for missing explicit `/` route and missing login token bypass.
- Green/full Web gate: `rtk bash -lc 'cd web && bun run lint && bun run typecheck && bun test && bun run build'` passed.
