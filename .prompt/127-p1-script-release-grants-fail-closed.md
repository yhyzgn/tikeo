# 127 — P1 script release grant payload boundary

## Context
Continue P1 script production governance after signed release metadata persistence. Preserve the hard source hygiene rule: every Rust/Web/SDK/example source file must stay `<=1500` lines, and `mod.rs` / `lib.rs` must remain module entry/re-export surfaces.

## Completed in this slice
- Added an explicit `ScriptReleaseGrantSet` domain model for URL/File/Secret release grants:
  - `url`
  - `file_read`
  - `file_write`
  - `secret`
- Added HTTP `ScriptReleaseRequest.grants` / OpenAPI DTO shape for publish/rollback requests.
- Grant payloads are accepted as explicit request shape but remain fail-closed: any non-empty grant returns a business error until verified grant enforcement is implemented.
- Added core/server tests proving empty grants are accepted while non-empty URL/File/Secret grants are rejected before release pointer movement.
- Extended the Web API client types so future UI flows can pass the same explicit grant payload shape without hand-written drift.

## Validation evidence
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help >/tmp/tikeo-help.out`
- `cd web && bun run typecheck && bun run lint && bun test && bun run build`
- Source line count check excluding generated/vendor build folders: max remains `1495` lines.

## Next recommended slice
Implement persisted signed grant evidence for successful grant verification, still without enabling Worker-side URL/File/Secret access until a concrete verifier/KMS policy is configured. Keep Server as scheduler/governance only; execution stays Worker-side.
