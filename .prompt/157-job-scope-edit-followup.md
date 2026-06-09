# 157 — Job scope edit follow-up

## Context

2026-06-09 completed job namespace/app edit support:

- `PATCH /api/v1/jobs/{job}` accepts optional `namespace` and `app` fields.
- Backend checks both the current job scope and destination scope before moving.
- Storage updates `namespace_id` / `app_id` and treats scope moves as versioned job changes.
- Web Jobs edit drawer now uses tenant scope Select fields for namespace/app instead of disabled display-only selects.
- Canary target choices are constrained to the selected namespace/app.

## Verified commands

- `cargo test -p tikeo-server job_management_update_can_move_namespace_and_app -- --nocapture`
- `cd web && bun test src/api/client.test.ts src/pages/__tests__/JobsPage.test.tsx`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cd web && bun run typecheck && bun run test && bun run build`
- `python3 scripts/check-source-size.py`
- `git diff --check`

## Follow-up options

1. Add a dedicated Web interaction/e2e smoke for editing a job scope after a real Scopes-page namespace/app setup.
2. Expose scope-change details more prominently in job version history diff/audit views.
3. Consider whether moving a job should optionally clear or warn about schedule calendars/scripts that are scoped differently once those resources become hard-scoped.
4. Continue the previous docs/API reference track from `154-docs-ci-and-reference-depth-followup.md` / `.memory/next.md`.

## Guardrails

- Do not regress to disabled namespace/app edit fields.
- Do not allow target-scope writes without target-scope authorization.
- Do not allow canary routing across namespace/app unless a future explicit cross-scope canary design is written and approved.
