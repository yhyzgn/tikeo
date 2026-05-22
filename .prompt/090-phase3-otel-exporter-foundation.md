# 090 — Phase 3 OTLP exporter foundation

## Context
Phase 089 extended the deterministic metrics summary with dispatch queue SLO fields. Phase 3 OpenTelemetry work still has only HTTP trace-id propagation and local spans; real OTLP collector integration remains open.

## Objectives
1. Add the next smallest locally verifiable OpenTelemetry hardening slice, preferably OTLP exporter configuration/readiness metadata.
2. Preserve local development defaults: no collector should be required for tests or `cargo run -- --help`.
3. Keep HTTP responses in `{ code, message, data }`, avoid credentials in status payloads, and do not change Worker inbound network posture.

## Constraints
- Do not pull Phase 4 items back into Phase 3: Node.js SDK, K8s Helm, PowerJob migration, XXL-JOB migration.
- Prefer configuration/status plumbing before adding network exporter side effects.
- If adding dependencies, justify them and keep verification deterministic.

## Expected verification
- Targeted tests for configured/default OTLP readiness metadata.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`
- Web validation if Web files change.

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/091-*.md` before commit.
- Commit with Lore trailers and push.
