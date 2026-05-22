# 072 — Phase 3 script policy engine and non-WASM sandbox runners

## Context
Phase 071 completed explicit script release pointers: dispatch no longer executes mutable script rows and WASM bindings now fail closed unless `scripts.released_version_id/released_version_number` points at an immutable `script_versions` snapshot. Publish/rollback APIs and Web controls exist.

## Objectives
1. Design and implement the next script security slice: policy metadata for capabilities/resources/network/filesystem/secrets that can be attached to script definitions and version snapshots.
2. Add validation that dangerous policies require explicit status/approval gates; keep default-deny behavior.
3. Start Worker-side non-WASM runner abstraction for Python/Node/Shell/PowerShell/Rhai without executing inside the Server process.
4. Preserve immutable release semantics: execution uses released `script_versions` snapshot plus policy snapshot only.
5. Update Web script management to display/edit policy fields safely and show policy diffs between versions.

## Constraints
- Server must never execute user code.
- No database foreign keys; soft id relationships only.
- API response envelope remains `{ code, message, data }`.
- Default network/filesystem/secrets access is denied.
- SDKs must remain independently buildable/publishable.

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
- Create the next `.prompt/073-*.md` before committing.
- Commit with Lore trailers and push.
