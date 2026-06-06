# 128 — P1 script release grant evidence persistence

## Context
Continue P1 script production governance after explicit URL/File/Secret grant request payloads. Preserve the hard source hygiene rule: every Rust/Web/SDK/example source file must stay `<=1500` lines, and `mod.rs` / `lib.rs` must remain module entry/re-export surfaces.

## Completed in this slice
- Added persisted release-pointer evidence fields for verified URL/File/Secret grants:
  - `release_grants_json`
  - `release_grants_verified_at`
  - `release_grants_verified_by`
- Added storage DTOs for verified grant evidence exposed through `ScriptSummary`.
- Added SQLite compatibility migration for existing dev databases.
- Added repository-level persistence test for verified grant evidence.
- Web Scripts detail can display verified grant evidence when future verification paths produce it.
- HTTP publish/rollback still pass no verified grant evidence and non-empty release grants remain fail-closed; no Worker-side URL/File/Secret access is enabled.

## Validation evidence
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help >/tmp/tikeo-help.out`
- `cd web && bun run typecheck && bun run lint && bun test && bun run build`
- Source line count check excluding generated/vendor build folders: max remains `1495` lines.

## Next recommended slice
Introduce a concrete verified grant verifier boundary (local first, KMS/PKI later) that can convert a signed release grant request into persisted grant evidence without enabling execution until Worker-side enforcement is separately implemented and tested.
