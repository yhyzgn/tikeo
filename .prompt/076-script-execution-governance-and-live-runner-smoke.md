# 076 — Script execution governance and live runner smoke

## Context
Phase 075 added Rust SDK `ContainerScriptRunner` as an explicit Worker-side opt-in runner for non-WASM scripts. It builds Docker-compatible default-deny runtime commands and validates released snapshots before spawn. The Server still never executes user code.

## Objectives
1. Add script-bound execution governance visibility for failure classes: no eligible worker capability, missing worker runner, policy rejection, digest mismatch, timeout, output limit, and runtime unavailable.
2. Surface governance in audit/result data where the current schema supports it, without adding database foreign keys.
3. Add optional live smoke tooling for `ContainerScriptRunner` when Docker/compatible runtime is available; deterministic unit tests must still pass without Docker.
4. Document how operators should deploy script-capable worker pools in Docker/K8s and which capabilities (`script`, with legacy `script:<language>` / `script:*` compatibility) they should advertise.

## Constraints
- Server must never execute user scripts or require Docker/K8s privileges.
- Dynamic scripts execute only on Worker-side registered runners and only from released immutable `script_versions` snapshots.
- API responses remain `{ code, message, data }`.
- DB relationships remain soft-only; do not add foreign keys.
- SDKs remain independently buildable/publishable.
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
- Update `design/tikeo-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`.
- Create `.prompt/077-*.md` before commit.
- Mark completed roadmap items in `design/` using `[x]` only, no ✅.
- Commit with Lore trailers and push.
