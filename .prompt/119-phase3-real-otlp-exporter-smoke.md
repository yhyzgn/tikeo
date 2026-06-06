# 119 — Phase 3 real OTLP exporter smoke

## Goal
Complete the OpenTelemetry distributed tracing Phase 3 item by wiring a real OTLP HTTP span exporter into server startup and proving it can export spans to a live local collector endpoint.

## Completed scope
- Added `tikeo_server::observability::tracing::TracingRuntime` as a focused runtime module instead of embedding exporter setup in route handlers.
- Server startup now initializes tracing after loading config, preserving disabled-by-default local CLI output and enabling OTLP export when `observability.tracing.enabled=true` plus an endpoint is configured.
- OTLP exporter uses OpenTelemetry SDK + `tracing-opentelemetry` over OTLP/HTTP protobuf with configured header names redacted from status APIs and represented as smoke-test headers.
- Added a local Axum collector smoke test that receives a non-empty OTLP trace payload and verifies configured headers are sent.

## Verification
- `rtk cargo test -p tikeo-server --test otel_exporter_smoke --all-features`
- `rtk cargo test -p tikeo-server observability_status_reports_default_and_configured_otlp_without_collector --all-features`
- Targeted fmt/clippy for `tikeo-server`.

## Remaining observability follow-ups
- Live Prometheus/Grafana recording-rule validation for metrics remains separate from OpenTelemetry tracing.
