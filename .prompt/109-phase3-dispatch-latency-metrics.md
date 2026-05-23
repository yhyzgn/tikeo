# 109 — Phase 3 dispatch latency metrics

## Goal
Close the local end-to-end dispatch latency observability gap by exposing completed dispatch queue latency rollups and Prometheus histogram snapshots.

## Scope
- Extend `DispatchQueueSloSummary` with completed dispatch count plus average/longest dispatch latency seconds.
- Treat terminal dispatch queue rows (`done` / `failed`) as completed dispatches and calculate latency from queue creation to terminal update time.
- Record `tikee_dispatch_queue_dispatch_latency_seconds` histogram snapshots and a completed dispatch gauge during `GET /api/v1/metrics/summary`.
- Update the Phase 3 Grafana template and regression coverage to include the dispatch latency metric.

## Out of scope
- Live Prometheus recording-rule validation.
- Real collector/exporter smoke.
- Per-worker dispatch latency percentile persistence beyond Prometheus snapshots.
