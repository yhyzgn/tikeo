# 100 — Worker cluster page UX refresh

## Context
The user reported the Worker cluster page interaction and layout were hard to use. The existing page showed a hero, four queue stats, and two plain lists, which made it hard to filter Worker capacity or focus on queue status.

## Objectives
1. Rework the Worker cluster page into a data-dense operations dashboard.
2. Keep API contracts unchanged: continue using `GET /api/v1/workers` and `GET /api/v1/dispatch-queue`.
3. Add practical interactions: worker search, namespace filter, capability filter, queue status drill-down, and clear refresh affordance.
4. Split the page into focused React modules instead of growing a single large page file.
5. Add regression coverage for the new interaction/layout contracts.

## Constraints
- Preserve existing auth and API envelope behavior.
- No new frontend dependencies.
- Keep responsive behavior for mobile/tablet layouts.

## Expected verification
- `bun test web/src/pages/__tests__/WorkersPage.test.tsx`
- `cd web && bun run typecheck`
- `cd web && bun run lint`
- `cd web && bun run build`

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, and `.memory/next.md`.
- Commit with Lore trailers and push.
