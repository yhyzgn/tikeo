# 073 — Phase 3 concrete script sandbox runner implementations

## Context
Phase 072 added default-deny `ScriptExecutionPolicy` metadata, persisted policy snapshots on scripts and immutable `script_versions`, Web policy editing/visibility, API-side policy validation, Rust SDK non-WASM runner abstractions, and Vite/Rolldown chunk splitting.

## Objectives
1. Implement the first concrete non-WASM sandbox runner behind explicit opt-in, starting with the safest local subprocess boundary for Shell/Python/Node where available.
2. Enforce `ScriptRunnerPolicy` limits before execution and keep network/filesystem/secrets denied unless a future policy grant exists.
3. Ensure all execution binds to released immutable `script_versions` snapshots, never mutable draft content.
4. Add tests for runner rejection paths, timeout/output limits, and unavailable runtime behavior.
5. Update Web/design/docs to explain what is actually executable versus planned.

## Constraints
- Server must never execute user code.
- No database foreign keys; soft relationships only.
- API responses remain `{ code, message, data }`.
- SDKs remain independently publishable/buildable.
- Avoid enabling dangerous host capabilities without explicit user-approved design.

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
- Create the next `.prompt/074-*.md` before commit.
- Commit with Lore trailers and push.
