# 129 — P1 local signed release grants

## Context
User asked to move faster and complete P1 script governance items. This slice turns the previous fail-closed grant payload/evidence boundary into a local verified path while still keeping Server as scheduler/governance and not enabling Worker-side URL/File/Secret access.

## Completed in this slice
- Extended local env-secret release signatures to bind canonical release grants in the signed payload.
- `ScriptReleaseRequest.grants` can now be accepted when `script_governance.release_signature_secret_ref` is configured and the signature matches script id, immutable version, content SHA-256, approval ticket, and grants JSON.
- Verified grants are persisted into release pointer evidence (`release_grants`) alongside signature metadata.
- Unconfigured systems remain fail-closed for non-empty grants and approval/signature metadata.
- Worker-side URL/File/Secret access remains disabled; this only completes release governance evidence for local verification.

## Validation evidence
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help >/tmp/tikee-help.out`
- `cd web && bun run typecheck && bun run lint && bun test && bun run build`
- Source line count check excluding generated/vendor build folders: max remains `1495` lines.

## Next recommended slice
Mark the script approval/signature/grants P1 subitem complete in roadmap if no KMS/PKI is required for P1, then move to the next P1 task: OIDC tenant/app/role binding policy and advanced tenant isolation UI.
