# 126 — P1 script release signature metadata persistence

## Context
Continue P1 script production governance after local signature verification. Preserve the hard source hygiene rule: every Rust/Web/SDK/example source file must stay `<=1500` lines, and `mod.rs` / `lib.rs` must remain module entry/re-export surfaces.

## Completed in this slice
- Persist verified script release signature metadata on the `scripts` release pointer:
  - `release_approval_ticket`
  - `release_signature`
  - `release_signature_verified_at`
  - `release_signature_verified_by`
- Expose the metadata through `ScriptSummary` for list/detail/publish/rollback responses.
- Keep unsigned releases possible for currently safe local/dev flows; signed metadata is stored only after configured verification succeeds.
- Display release signature state and details in the Web Scripts page.
- Maintain SQLite compatibility migration for existing dev databases.

## Validation evidence
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help >/tmp/tikee-help.out`
- `cd web && bun run typecheck && bun run lint && bun test && bun run build`
- Source line count check excluding generated/vendor build folders: max remains `1495` lines.

## Next recommended slice
Design and implement signed URL/File/Secret grant payloads as explicit request/response DTOs and persisted policy metadata. They must remain fail-closed until verified by approval/signature/KMS policy, and Server must continue to only schedule metadata, never execute user scripts.
