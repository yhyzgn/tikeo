# 089 — Phase 3 business SLO metrics

## Context
Phase 088 added a deterministic Grafana dashboard template for the Prometheus metrics that already exist locally. The Phase 3 observability roadmap still has richer scheduling-latency and business SLO metrics open.

## Objectives
1. Add the next smallest locally verifiable business/SLO metric slice, preferably scheduling or dispatch latency derived from existing persisted timestamps/state transitions.
2. Keep local development defaults and do not require an external Prometheus, Grafana, or OTLP collector in tests.
3. Preserve `{ code, message, data }` API envelopes, no-foreign-key storage policy, and Worker outbound-only architecture.

## Constraints
- Do not pull Phase 4 items back into Phase 3: Node.js SDK, K8s Helm, PowerJob migration, XXL-JOB migration.
- Prefer metrics emitted from existing Server paths; do not execute user scripts on Server.
- If adding storage fields, keep relationships soft and migrations SQLite/MySQL compatible.

## Expected verification
- Targeted tests for the selected metric behavior.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`
- Web validation if Web files change.

## Completion notes
- Update `design/tikeo-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/090-*.md` before commit.
- Commit with Lore trailers and push.
