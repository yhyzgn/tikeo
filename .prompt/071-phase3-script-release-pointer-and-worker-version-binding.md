# 071 — Phase 3 script release pointer and worker version binding

## Context
Phase 070 added WASM module SHA-256 integrity metadata, script version snapshot digests, Rust Worker SDK digest validation, Web sandbox-policy visibility, and Gradle 10 deprecation cleanup. Dispatch can include immutable version metadata when a matching version snapshot exists, but the system still primarily uses the mutable script row as the dispatch source.

## Objectives
1. Add an explicit script release/publish pointer so approved execution binds to an immutable `script_versions` row, not the mutable `scripts.content` row.
2. Add rollback and publish APIs that move the release pointer by soft id/version relationship only; do not create database foreign keys.
3. Update dispatcher so `script:<id>` WASM bindings always use the released immutable version snapshot and fail closed when no released version exists.
4. Update Web script management to show current draft vs released version, publish/rollback actions, and immutable digest/version metadata.
5. Keep audit logging for publish/rollback/status changes.

## Constraints
- No database foreign keys; soft relationships only.
- API responses must keep `{ code, message, data }`.
- Server must never execute user code.
- Default WASM policy remains deny network/filesystem unless explicitly changed by a later verified phase.
- SDKs remain independently publishable.

## Expected verification
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -- --help`
- `cd web && bun run typecheck && bun test && bun run build`
- `cargo test --manifest-path sdks/rust/scheduler-worker-sdk/Cargo.toml`
- `cargo test --manifest-path sdks/rust/scheduler-worker-sdk/Cargo.toml --features wasm`
- `cargo clippy --manifest-path sdks/rust/scheduler-worker-sdk/Cargo.toml --all-targets --all-features -- -D warnings`
- `cd sdks/java && ./gradlew test --warning-mode all --no-daemon`

## Completion notes
- Update `design/scheduler-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, and create `.prompt/072-*.md`.
- Commit with Lore trailers and push.
