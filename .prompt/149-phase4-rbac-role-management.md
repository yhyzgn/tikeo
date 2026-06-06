# Phase 4 / RBAC role management module

## Goal
Implement the production-grade RBAC role management module described in `design/rbac-role-management-plan.md`.

## Non-negotiable rules
- Do not use JWT or claim-encoded local sessions.
- Do not rely on string conventions or scattered hardcoded role/menu/action-element names. Use structured role, backend permission, menu permission, and UI action-element catalogs.
- The one-time bootstrap account must be represented by `users.bootstrap_admin` / `principal.bootstrapAdmin` and must bypass ordinary role constraints structurally. Never infer it from username.
- `admin` is the built-in default role and must not be deleted, disabled, or accidentally stripped of platform recovery capability.
- Users may be assigned roles only through managed role records. Do not allow free-form role input in Web or API payloads.
- Role/user assignment must require both user-management permission and role-assignment/manage permission.
- Role changes must revoke affected human sessions so permissions refresh immediately.
- Keep SDK Management API-Key / Service Account authorization separate from human RBAC roles.
- Keep source files modular; do not add `#[allow(clippy::too_many_lines)]` or similar bypasses.
- Web work must use `bun`/`bunx` by default and must update locale files for all user-facing strings.

## Required implementation scope
1. Storage migration and repository
   - Add role metadata fields and `user_roles` soft relation.
   - Add menu permission catalog/role binding storage or an equivalent structured storage shape.
   - Add UI action-element permission catalog/role binding storage for button/table-action/block-level controls where needed.
   - Backfill existing `users.role` into `user_roles` for admin/operator/viewer.
   - Split RBAC repository logic into a focused module if current files grow materially.

2. Backend API
   - Add role CRUD and backend permission/menu/UI action-element catalog endpoints.
   - Add role permission matrix update endpoint with full replacement semantics.
   - Update user create/update payloads to use role ids/names from managed roles.
   - Add `bootstrapAdmin` and menu permission data to `/auth/me`.
   - Remove admin hardcoded bypass from service/frontend logic; admin should win because its role has all permissions.
   - Add audit logs for role create/update/delete, permission matrix updates, and user role assignment.

3. Web UI
   - Add “角色管理” under governance menu.
   - Implement role list, create/edit drawer or page, backend permission matrix, menu permission matrix, UI action-element matrix, built-in role protection states, affected-user warning.
   - Update Users page to load roles from API and assign roles via structured selector.
   - Update `AuthGuard`/menu filtering to use `bootstrapAdmin`, permission catalog, server-provided menu keys, and server-provided UI action keys for operation elements such as view/edit/delete/trigger/approve/rollback buttons.
   - Ensure complete i18n in `web/src/i18n/locales/zh-CN.ts` and `en-US.ts`.

4. Verification
   - Rust storage tests for migrations/backfill/role CRUD/permission updates.
   - HTTP tests for role API, bootstrap bypass, admin locked behavior, user role assignment permission gates, session invalidation.
   - Bun tests for API client and permission/menu rendering helpers.
   - Playwright smoke for role page, user role assignment, and menu visibility.
   - Run `cargo fmt --all`, targeted/full Rust tests, `cargo clippy -p tikee-server -p tikee-storage --all-targets --all-features -- -D warnings`, `bun run typecheck`, and relevant `bun test` suites.

## Acceptance criteria
- A bootstrap admin can still access all protected backend APIs and all Web menus even if ordinary role bindings are removed.
- A non-bootstrap account receives only backend permissions, menu entries, and UI operation elements granted by its assigned roles.
- A role manager can edit role backend permission/menu/UI action-element matrices without editing code.
- A user manager without role assignment permission cannot change user roles.
- Deleting/disabling protected or in-use roles fails with clear impact details.
- No new user-facing Web text is hardcoded outside locale files.
- Existing admin/operator/viewer users remain compatible after migration.
