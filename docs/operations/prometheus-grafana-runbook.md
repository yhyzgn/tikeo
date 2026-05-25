# Prometheus / Grafana operations runbook

## Goal
Validate that tikee exposes Prometheus metrics, recording rules load, and Grafana dashboards use either raw metrics or stable `tikee:*` recording series.

## Compose smoke

```bash
cp deploy/compose/tikee.env.example .env
DOCKER_BUILDKIT=1 docker compose --profile observability --env-file .env up -d --build
curl -fsS http://127.0.0.1:${TIKEE_HTTP_PORT:-9090}/metrics | grep tikee_http_requests_total
curl -fsS http://127.0.0.1:${TIKEE_PROMETHEUS_PORT:-9091}/-/ready
curl -fsS 'http://127.0.0.1:${TIKEE_PROMETHEUS_PORT:-9091}/api/v1/query?query=tikee:http_requests:rate5m'
```

## Required scrape path

Prometheus must scrape the server HTTP endpoint:

```yaml
scrape_configs:
  - job_name: tikee
    metrics_path: /metrics
    static_configs:
      - targets: ["tikee:9090"]
```

## Recording-rule checks

Rules live at `observability/prometheus/tikee-recording-rules.yml` and are mounted by the Compose observability profile. Validate syntax with `promtool check rules` when Prometheus tooling is available. CI also checks the file structurally and ensures each rule references an emitted `tikee_` metric.

## Triage

- `/metrics` empty or missing expected series: call authenticated `GET /api/v1/metrics/summary` once to refresh snapshot gauges/histograms for queue, workflow, alert, and governance SLOs.
- Prometheus target down: confirm container DNS can resolve `tikee` and that `TIKEE_HTTP_PORT` maps to server port 9090.
- Grafana panel `N/A`: query the matching recording series first, then fall back to the raw metric expression in the dashboard JSON.
