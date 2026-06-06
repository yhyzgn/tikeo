# 070 — Phase 3 WASM distribution integrity and Gradle 10 cleanup

## Context
Phase 069 completed SDK-side WASM dispatch handling: Rust Worker SDK executes `processor_binding.wasm` only behind the explicit `wasm` feature, while Java SDK returns a clear unsupported result without invoking normal processors. Server still only distributes approved metadata/module bytes and must not execute user code.

## Objectives
1. Add WASM module integrity metadata to the script binding contract and worker validation path. Prefer SHA-256 digest first; design signature hooks without requiring production PKI yet.
2. Bind dispatched WASM modules to immutable script versions/releases rather than ambiguous mutable script rows where possible.
3. Expose sandbox policy metadata clearly in API/Web views so operators can see timeout/memory/fuel/network settings before approval/run.
4. Clean Java Gradle deprecation warnings reported by Gradle 9.5.1 and keep the Java SDK ready for Gradle 10.

## Constraints
- Do not add foreign keys. Use soft id/version relationships only.
- Keep SDKs independently publishable; no path dependencies from SDK crates/modules to server crates.
- Server Docker must not build SDKs.
- API responses must keep `{ code, message, data }`.
- No Swagger UI.
- Default WASM policy remains deny network/filesystem unless explicitly designed and tested later.

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
- Update `design/tikeo-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, and create `.prompt/071-*.md`.
- Commit with Lore trailers and push.
