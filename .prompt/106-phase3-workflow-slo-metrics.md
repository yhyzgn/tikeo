# 106 — Phase 3 workflow SLO metrics

## Goal
Move Phase 3 business SLO coverage beyond summary-backed gauges by adding locally verifiable workflow and map-shard SLA metrics.

## Scope
- Extend `GET /api/v1/metrics/summary` with workflow instance and workflow shard SLO summaries.
- Emit Prometheus workflow instance/shard status gauges, success-ratio gauges, and duration histograms through the local recorder.
- Update the Phase 3 Grafana template to reference the new real workflow SLA metrics instead of treating workflow SLO as only a placeholder.

## Out of scope
- Live Prometheus recording-rule validation against an external server.
- Real OTLP collector smoke.
- Queue claim-to-worker-end dispatch latency beyond current queue pending-age and workflow duration snapshots.
