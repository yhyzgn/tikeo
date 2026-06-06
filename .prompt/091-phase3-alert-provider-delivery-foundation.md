# 091 — Phase 3 alert provider delivery foundation

## Context
Phase 090 added disabled-by-default OTLP exporter configuration and a redacted observability status endpoint. Phase 3 alerting still has durable rules/events/recovery/summary, but actual provider delivery remains open.

## Objectives
1. Add the next smallest locally verifiable alerting hardening slice, preferably provider/channel delivery readiness or deterministic delivery records.
2. Avoid external email/webhook/Slack/DingTalk/Feishu/PagerDuty dependencies in tests.
3. Preserve `{ code, message, data }`, no DB foreign keys, and local dev defaults.

## Constraints
- Do not pull Phase 4 items back into Phase 3: Node.js SDK, K8s Helm, PowerJob migration, XXL-JOB migration.
- Do not send real network notifications unless explicitly isolated behind disabled-by-default configuration.
- Redact secrets/tokens/webhook URLs from any operator-facing status payload.

## Expected verification
- Targeted tests for the selected alert delivery/readiness behavior.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`
- Web validation if Web files change.

## Completion notes
- Update `design/tikeo-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/092-*.md` before commit.
- Commit with Lore trailers and push.
