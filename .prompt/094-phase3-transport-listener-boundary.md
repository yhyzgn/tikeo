# 094 — Phase 3 transport listener boundary

## Context
Phase 093 added fail-closed script release approval/signature metadata gates. Phase 3 mTLS work still has config/status diagnostics, but real TLS listener wiring remains open.

## Objectives
1. Add the next smallest locally verifiable transport-security hardening slice, preferably listener startup/config validation boundaries.
2. Keep local development plaintext defaults working.
3. Avoid needing real certificates in normal CI unless generated locally by the test itself.

## Constraints
- Do not pull Phase 4 items back into Phase 3: Node.js SDK, K8s Helm, PowerJob migration, XXL-JOB migration.
- Do not expose Worker inbound ports; Worker remains outbound-only.
- Keep status APIs redacted and avoid logging private key contents.

## Expected verification
- Targeted tests for the selected transport boundary behavior.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`
- Web validation if Web files change.

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/095-*.md` before commit.
- Commit with Lore trailers and push.
