# 084 — OpenTelemetry tracing foundation

## Context
Phase 083 added a deterministic `/api/v1/metrics/summary` management API for dashboard/SLO groundwork without requiring Prometheus or Grafana in tests. The Phase 3 roadmap still has OpenTelemetry distributed tracing open.

## Objectives
1. Add a minimal OpenTelemetry/tracing foundation that keeps local tests deterministic.
2. Propagate or generate request trace identifiers through HTTP responses/audit paths where useful without changing the `{ code, message, data }` envelope.
3. Document how OTLP/exporter configuration will be enabled later without requiring an external collector in CI.

## Constraints
- Do not require an OTLP collector, Jaeger, Tempo, or network smoke in verification.
- Preserve existing `tracing_subscriber` behavior for local CLI output.
- Keep modules split by responsibility; no monolithic tracing setup in route handlers.
- Do not add Swagger.

## Expected verification
- Targeted Rust tests for trace-id propagation or tracing configuration behavior.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo run -- --help`

## Completion notes
- Update `design/tikee-architecture-design.md`, `.memory/progress.md`, `.memory/session-log.md`, `.memory/next.md`, and `.memory/risks.md` if risks change.
- Create the next `.prompt/085-*.md` before commit.
- Commit with Lore trailers and push.
