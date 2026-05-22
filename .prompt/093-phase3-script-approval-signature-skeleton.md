# 093 — Phase 3 script approval/signature skeleton

## Context
Phase 092 added OIDC authorize/callback skeleton endpoints that fail closed without real token verification. Phase 3 dynamic script governance still has publish/rollback policy gates, but full multi-level approval/signing/production gate remains open.

## Objectives
1. Add the next smallest locally verifiable script approval/signature hardening slice.
2. Prefer metadata/status gates or audit-visible approval requirements over adding runtime execution behavior.
3. Preserve Server-does-not-execute-user-code, Worker outbound-only, no DB foreign keys, and `{ code, message, data }`.

## Constraints
- Do not pull Phase 4 items back into Phase 3: Node.js SDK, K8s Helm, PowerJob migration, XXL-JOB migration.
- Do not add real signing services, KMS, or external approval providers unless disabled-by-default and locally testable.
- Keep dangerous script capabilities fail-closed until explicit grant/signature support exists.

## Expected verification
- Targeted tests for the selected approval/signature behavior.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`
- Web validation if Web files change.

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/094-*.md` before commit.
- Commit with Lore trailers and push.
