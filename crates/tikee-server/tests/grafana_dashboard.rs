//! Regression coverage for the committed Grafana dashboard template.

use serde_json::Value;

const DASHBOARD_JSON: &str =
    include_str!("../../../observability/grafana/tikee-phase3-dashboard.json");

#[test]
fn phase3_grafana_dashboard_is_valid_and_covers_core_metrics() -> Result<(), serde_json::Error> {
    let dashboard: Value = serde_json::from_str(DASHBOARD_JSON)?;

    assert_eq!(
        dashboard.get("title").and_then(Value::as_str),
        Some("tikee Phase 3 Operations")
    );

    let panels = dashboard.get("panels").and_then(Value::as_array);
    assert!(panels.is_some(), "dashboard must declare panels");
    assert!(
        panels.is_some_and(|panels| !panels.is_empty()),
        "dashboard must include at least one panel"
    );

    let dashboard_text = DASHBOARD_JSON;
    for query in [
        "tikee:http_requests:rate5m",
        "tikee:http_latency:p95_5m",
        "tikee_worker_connected_total",
        "tikee_worker_dispatch_total",
        "tikee:dispatch_pending_age:p95_5m",
        "tikee:dispatch_latency:p95_5m",
        "tikee:workflow_instance_duration:p95_5m",
        "tikee:workflow_shard_duration:p95_5m",
        "tikee:workflow_instance_success:ratio",
    ] {
        assert!(
            dashboard_text.contains(query),
            "dashboard must include query for {query}"
        );
    }

    Ok(())
}

const RECORDING_RULES_YAML: &str =
    include_str!("../../../observability/prometheus/tikee-recording-rules.yml");
const PROMETHEUS_CONFIG_YAML: &str =
    include_str!("../../../observability/prometheus/prometheus.yml");

#[test]
fn prometheus_recording_rules_cover_dashboard_slo_series() {
    let required_rules = [
        "tikee:http_requests:rate5m",
        "tikee:http_errors:ratio5m",
        "tikee:http_latency:p95_5m",
        "tikee:dispatch_pending_age:p95_5m",
        "tikee:dispatch_latency:p95_5m",
        "tikee:workflow_instance_duration:p95_5m",
        "tikee:workflow_shard_duration:p95_5m",
        "tikee:job_instance_success:ratio",
        "tikee:workflow_instance_success:ratio",
        "tikee:workflow_shard_success:ratio",
        "tikee:workers_online:current",
        "tikee:script_governance_failures:current",
    ];
    for rule in required_rules {
        assert!(
            RECORDING_RULES_YAML.contains(&format!("record: {rule}")),
            "recording rules must define {rule}"
        );
    }

    for emitted_metric in [
        "tikee_http_requests_total",
        "tikee_http_request_duration_seconds_bucket",
        "tikee_dispatch_queue_pending_age_seconds_bucket",
        "tikee_dispatch_queue_dispatch_latency_seconds_bucket",
        "tikee_workflow_instance_duration_seconds_bucket",
        "tikee_workflow_shard_duration_seconds_bucket",
        "tikee_job_instance_success_ratio",
        "tikee_workflow_instance_success_ratio",
        "tikee_workflow_shard_success_ratio",
        "tikee_workers_online_current",
        "tikee_script_governance_failures_current",
    ] {
        assert!(
            RECORDING_RULES_YAML.contains(emitted_metric),
            "recording rules must reference emitted metric {emitted_metric}"
        );
    }

    assert!(
        PROMETHEUS_CONFIG_YAML.contains("metrics_path: /metrics")
            && PROMETHEUS_CONFIG_YAML.contains("tikee-recording-rules.yml")
            && PROMETHEUS_CONFIG_YAML.contains("tikee:9090"),
        "Prometheus config must scrape tikee and load recording rules"
    );
}

#[test]
fn grafana_dashboard_uses_recording_rules_for_slo_panels() {
    for recording_series in [
        "tikee:http_requests:rate5m",
        "tikee:http_latency:p95_5m",
        "tikee:http_errors:ratio5m",
        "tikee:dispatch_pending_age:p95_5m",
        "tikee:dispatch_latency:p95_5m",
        "tikee:workflow_instance_duration:p95_5m",
        "tikee:workflow_shard_duration:p95_5m",
        "tikee:workflow_instance_success:ratio",
        "tikee:workflow_shard_success:ratio",
    ] {
        assert!(
            DASHBOARD_JSON.contains(recording_series),
            "dashboard should query recording series {recording_series}"
        );
    }
}
