# 077 — Script execution governance after tikeo rename

## Context
The repository/product has been renamed from the previous project identity to tikeo. Rust crates, binary name, Docker/Compose identifiers, protobuf package namespace, Rust SDK crate, Java Gradle modules, Java package names, and Spring Boot properties now use tikeo naming. Java SDK package prefix is `net.tikeo`.

Phase 075 previously added Rust SDK `ContainerScriptRunner` as an explicit Worker-side opt-in runner for non-WASM scripts. It builds Docker-compatible default-deny runtime commands and validates released snapshots before spawn. The Server still never executes user code.

## Objectives
1. Add script-bound execution governance visibility for failure classes: no eligible worker capability, missing worker runner, policy rejection, digest mismatch, timeout, output limit, and runtime unavailable.
2. Surface governance in audit/result data where the current schema supports it, without adding database foreign keys.
3. Add optional live smoke tooling for `ContainerScriptRunner` when Docker/compatible runtime is available; deterministic unit tests must still pass without Docker.
4. Document how operators should deploy script-capable worker pools in Docker/K8s and which capabilities (`script`, with legacy `script:<language>` / `script:*` compatibility) they should advertise.

## Naming constraints
- Use product/binary/crate prefix `tikeo`, not the previous project name.
- Java packages must stay under `net.tikeo`.
- Rust SDK path/crate is `sdks/rust/tikeo` / `tikeo`.
- Java SDK modules are `tikeo`, `tikeo-spring`, and `tikeo-spring-boot-starter`.
- Protobuf package is `tikeo.worker.v1`.
- Environment variables use `TIKEO_` / `TIKEO__` prefixes.
- Internal Raft transport header is `x-tikeo-raft-token`.

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
- Create the next `.prompt/078-*.md` before commit.
- Mark completed roadmap items in `design/` using `[x]` only, no ✅.
- Commit with Lore trailers and push.
