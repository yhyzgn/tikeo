# 079 — Script governance audit materialization

## Context
Phase 078 made `script_execution_governance` instance logs queryable and UI-visible. The instance logs API now parses governance JSON into `governance_event`, `governance_failure_class`, and `governance_message`, supports `page_token=script_execution_governance` as a compatibility filter, and the Web Instances log drawer highlights governance failures. `AlertCondition::ScriptGovernanceFailure` exists as the alert-rule shape, but governance logs are not yet materialized into durable `audit_logs` rows or an alert-history stream.

## Objectives
1. Materialize script governance events into audit rows or a clearly bounded audit-adjacent repository method without adding DB foreign keys.
2. Preserve instance log compatibility and avoid duplicating user-visible messages inconsistently.
3. Add tests that a governance failure can be queried from audit/governance surfaces by failure class.
4. Keep alerting event hooks deterministic; no external webhook smoke is required.

## Constraints
- Server must never execute user scripts or require Docker/K8s privileges.
- Relationships remain soft-only; do not add database foreign keys.
- HTTP business responses remain `{ code, message, data }`.
- Keep Rust/server/web code split by responsibility; do not grow large monolithic files.
- Do not add Swagger.

## Expected verification
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -- --help`
- `cd web && bun run typecheck && bun test && bun run build`
- `cargo test --manifest-path sdks/rust/tikee/Cargo.toml`
- `cargo test --manifest-path sdks/rust/tikee/Cargo.toml --features wasm`
- `cargo clippy --manifest-path sdks/rust/tikee/Cargo.toml --all-targets --all-features -- -D warnings`
- `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, `.memory/risks.md` if any risk changes.
- Create the next `.prompt/080-*.md` before commit.
- Mark completed roadmap items in `design/` using `[x]` only, no ✅.
- Commit with Lore trailers and push.

## Interruption note — 2026-05-23 workflow edge-condition fix
Before continuing this slice, a Web workflow bug was fixed: legacy dev seed data used edge `condition: "success"`, while the current UI selector already uses canonical `on_success`. The Web API boundary now normalizes stale aliases before create/update/dry-run, the editor normalizes loaded definitions, and `scripts/dev-seed.sql` seeds `on_success`.

## Interruption note — 2026-05-23 Web UX fixes
Before continuing this slice, two Web UX fixes were completed: script editing moved from modal to guarded secondary route `/scripts/:id/edit` with the existing diff-before-save confirmation preserved, and the workflow DAG editor canvas gained a fullscreen toggle for large workflow editing.
