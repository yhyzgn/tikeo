# 088 — Phase 3 remaining hardening

## Context
Phase 087 added deterministic script publish/rollback policy gates for dangerous legacy policy snapshots and failed audit materialization. The largest remaining Phase 3 items are now production-hardening continuations: full multi-level approval/signing, real OIDC login callback, real TLS listeners, Grafana dashboard templates, and richer business SLO metrics.

## Objectives
1. Pick the next smallest Phase 3 hardening slice that can be completed and verified locally.
2. Preserve existing API envelopes, soft relationships, and local dev defaults.
3. Continue updating the roadmap and `.memory` after each slice.

## Constraints
- Do not pull Phase 4 items back into Phase 3: Node.js SDK, K8s Helm, PowerJob migration, XXL-JOB migration.
- Avoid external IdP, certificate, Grafana, or webhook dependencies unless the slice explicitly adds a deterministic local substitute.
- Server must not execute user scripts.

## Expected verification
- Targeted tests for the selected hardening slice.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`
- Web validation if Web files change.

## Completion notes
- Update `design/tikeo-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/089-*.md` before commit.
- Commit with Lore trailers and push.
