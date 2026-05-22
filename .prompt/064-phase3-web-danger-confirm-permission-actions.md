# 064 — Phase 3 Web dangerous-action confirmation and permission-aware actions

## Context
Audit governance is now strengthened with before/after/trace/result fields and a governed JSON export endpoint. The next Phase 3 roadmap item is Web UI dangerous-operation confirmation and permission-aware action rendering.

## Required next work
1. Audit Web UI destructive/mutating actions (delete script, delete user, workflow recover/skip/fail/succeed, trigger/run/materialize, worker/queue actions) and add confirmation dialogs for dangerous operations.
2. Use existing auth/RBAC permission context to hide or disable actions the current principal cannot perform; preserve route guards.
3. Ensure all backend responses still use `{ code, message, data }`; no database foreign keys.
4. Add or update tests where practical (frontend typecheck/build at minimum; backend tests if permission metadata changes).
5. Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, and create `.prompt/065-*.md`.
6. Run full verification, commit with Lore-style trailers, and push.
