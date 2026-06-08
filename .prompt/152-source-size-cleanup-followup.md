# 152 — Source-size cleanup follow-up

## Current context
The historical source-size debt has been cleaned up. Normal Rust/TypeScript/TSX source files now pass the `<=1500` line rule through `scripts/check-source-size.py`. The cleanup was behavior-preserving and split only by existing responsibilities.

## Completed cleanup boundaries
- Server dispatcher: builtin SQL/gRPC/file-cleanup/HTTP processors moved to `crates/tikeo-server/src/tunnel/dispatcher/processors.rs`; dispatcher tests split under `dispatcher/tests/`.
- Server registry: tests moved under `crates/tikeo-server/src/tunnel/registry/registry_tests.rs`.
- HTTP tests: `part_03` split into `part_03_a` and `part_03_b`.
- Storage repository: repository tests moved to `repository/tests.rs` with split include files.
- Workflow repository: runtime/queue/recovery methods moved to `repository/workflow/runtime.rs`.
- Migration: RBAC role-management migration moved to `migration/rbac_role_management.rs`.
- Web API client: workflow/worker API types and functions moved to `web/src/api/workflow.ts` and re-exported from `client.ts`.

## Verification baseline
Before handoff, local validation passed:
- `python3 scripts/check-source-size.py`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `bun run --cwd web lint`
- `bun run --cwd web typecheck`
- `bun run --cwd web test`
- `bun run --cwd web build`
- healthz smoke on a temporary SQLite config

## Next recommended slice
1. Check remote CI/Coverage for the source-size cleanup commit after push.
2. If green, either wire `scripts/check-source-size.py` into CI or start the standalone docs site from `design/docs-site-build-plan.md`.
3. Do not undo the module split boundaries unless a future feature creates a clearer responsibility split.
4. Keep all future source files <=1500 lines; split before adding behavior when approaching the limit.
