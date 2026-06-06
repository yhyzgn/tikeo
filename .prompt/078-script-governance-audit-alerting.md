# 078 — Script governance audit and alerting follow-up

## Context
Phase 077 added script execution governance visibility after the tikeo rename. The dispatcher and Worker result path now classify script execution governance failures into instance logs with `event=script_execution_governance` and `failure_class` values for no eligible worker capability, missing worker runner, policy rejection, digest mismatch, timeout, output limit, and runtime unavailable. Rust SDK `TaskOutcome::failure_class()` wraps recognized script failure results as JSON before reporting them to Server.

The Server still never executes user scripts. Dynamic scripts execute only on Worker-side opt-in runners and only from released immutable `script_versions` snapshots.

## Objectives
1. Promote script governance logs into first-class audit/alert inputs without adding DB foreign keys.
2. Add server-side query/filter affordances for `script_execution_governance` events where the current job instance log/audit schemas support it.
3. Wire alert-rule/event hooks for repeated script governance failures, especially `script_no_eligible_worker_capability`, `script_runtime_unavailable`, and `script_digest_mismatch`.
4. Extend Web visibility so instance/log views can highlight governance failure classes instead of showing only raw JSON.

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
- `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml`
- `cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm`
- `cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings`
- `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`

## Completion notes
- Update `design/tikeo-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, `.memory/risks.md` if any risk changes.
- Create the next `.prompt/079-*.md` before commit.
- Mark completed roadmap items in `design/` using `[x]` only, no ✅.
- Commit with Lore trailers and push.
