# 074 — Non-WASM script runner protocol and UI binding

## Context
Phase 073 added Rust SDK `LocalSubprocessScriptRunner` as an explicit opt-in local subprocess boundary. It validates released immutable script snapshot metadata, content SHA-256, default-deny policy, timeout/output limits, and unavailable runtime behavior. The tikee Server still never executes user code.

## Objectives
1. Extend Worker Tunnel processor binding metadata for non-WASM scripts without breaking existing WASM binding behavior.
2. Ensure dispatcher sends non-WASM script bindings only from released immutable `script_versions` snapshots and only when the script is approved/released.
3. Add SDK adapter routing so workers can explicitly register/choose non-WASM runners by language/capability.
4. Keep network/filesystem/secrets denied unless a future signed grant model exists.
5. Update Web/docs to state which script languages are actually executable and what worker capability is required.

## Constraints
- Server must never execute user code.
- DB relationships remain soft-only; do not add foreign keys.
- API responses remain `{ code, message, data }`.
- SDKs remain independently buildable/publishable.
- Dynamic script execution must bind to immutable released `script_versions`, never draft content.

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
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`.
- Create the next `.prompt/075-*.md` before commit.
- Commit with Lore trailers and push.
