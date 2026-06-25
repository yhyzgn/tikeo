//! Regression coverage for the committed Grafana dashboard template.

use serde_json::Value;

const DASHBOARD_JSON: &str =
    include_str!("../../../observability/grafana/tikeo-phase3-dashboard.json");

#[test]
fn phase3_grafana_dashboard_is_valid_and_covers_core_metrics() -> Result<(), serde_json::Error> {
    let dashboard: Value = serde_json::from_str(DASHBOARD_JSON)?;

    assert_eq!(
        dashboard.get("title").and_then(Value::as_str),
        Some("tikeo Phase 3 Operations")
    );

    let panels = dashboard.get("panels").and_then(Value::as_array);
    assert!(panels.is_some(), "dashboard must declare panels");
    assert!(
        panels.is_some_and(|panels| !panels.is_empty()),
        "dashboard must include at least one panel"
    );

    let dashboard_text = DASHBOARD_JSON;
    for query in [
        "tikeo:http_requests:rate5m",
        "tikeo:http_latency:p95_5m",
        "tikeo_worker_connected_total",
        "tikeo_worker_dispatch_total",
        "tikeo:dispatch_pending_age:p95_5m",
        "tikeo:dispatch_latency:p95_5m",
        "tikeo:workflow_instance_duration:p95_5m",
        "tikeo:workflow_shard_duration:p95_5m",
        "tikeo:workflow_instance_success:ratio",
    ] {
        assert!(
            dashboard_text.contains(query),
            "dashboard must include query for {query}"
        );
    }

    Ok(())
}

const RECORDING_RULES_YAML: &str =
    include_str!("../../../observability/prometheus/tikeo-recording-rules.yml");
const PROMETHEUS_CONFIG_YAML: &str =
    include_str!("../../../observability/prometheus/prometheus.yml");

const ALERT_RULES_YAML: &str =
    include_str!("../../../observability/prometheus/tikeo-alert-rules.yml");

#[test]
fn prometheus_alert_rules_cover_raft_ha_owner_pressure() {
    for alert in [
        "TikeoRaftNoSchedulableLeader",
        "TikeoRaftShardOwnershipMissing",
        "TikeoRaftShardOwnershipSkewHigh",
        "TikeoDispatchQueueOwnerBacklogHigh",
        "TikeoWorkerDispatchOutboxBacklogHigh",
    ] {
        assert!(
            ALERT_RULES_YAML.contains(&format!("alert: {alert}")),
            "alert rules must define {alert}"
        );
    }
    for metric in [
        "tikeo_cluster_can_schedule",
        "tikeo_cluster_shard_ownership_active_total",
        "tikeo_cluster_shard_ownership_owner_count",
        "tikeo_cluster_shard_ownership_skew",
        "tikeo_dispatch_queue_oldest_pending_age_by_owner_seconds",
        "tikeo_worker_dispatch_outbox_oldest_queued_age_seconds",
    ] {
        assert!(
            ALERT_RULES_YAML.contains(metric),
            "alert rules must reference emitted HA metric {metric}"
        );
    }
}

#[test]
fn grafana_dashboard_covers_raft_ha_owner_pressure() {
    for query in [
        "tikeo_cluster_shard_ownership_owner_count",
        "tikeo_cluster_shard_ownership_skew",
        "tikeo_dispatch_queue_pending_by_owner",
        "tikeo_dispatch_queue_oldest_pending_age_by_owner_seconds",
        "tikeo_worker_dispatch_outbox_oldest_queued_age_seconds",
    ] {
        assert!(
            DASHBOARD_JSON.contains(query),
            "dashboard must include HA owner pressure query {query}"
        );
    }
}

#[test]
fn prometheus_recording_rules_cover_dashboard_slo_series() {
    let required_rules = [
        "tikeo:http_requests:rate5m",
        "tikeo:http_errors:ratio5m",
        "tikeo:http_latency:p95_5m",
        "tikeo:dispatch_pending_age:p95_5m",
        "tikeo:dispatch_latency:p95_5m",
        "tikeo:workflow_instance_duration:p95_5m",
        "tikeo:workflow_shard_duration:p95_5m",
        "tikeo:job_instance_success:ratio",
        "tikeo:workflow_instance_success:ratio",
        "tikeo:workflow_shard_success:ratio",
        "tikeo:workers_online:current",
        "tikeo:script_governance_failures:current",
    ];
    for rule in required_rules {
        assert!(
            RECORDING_RULES_YAML.contains(&format!("record: {rule}")),
            "recording rules must define {rule}"
        );
    }

    for emitted_metric in [
        "tikeo_http_requests_total",
        "tikeo_http_request_duration_seconds_bucket",
        "tikeo_dispatch_queue_pending_age_seconds_bucket",
        "tikeo_dispatch_queue_dispatch_latency_seconds_bucket",
        "tikeo_workflow_instance_duration_seconds_bucket",
        "tikeo_workflow_shard_duration_seconds_bucket",
        "tikeo_job_instance_success_ratio",
        "tikeo_workflow_instance_success_ratio",
        "tikeo_workflow_shard_success_ratio",
        "tikeo_workers_online_current",
        "tikeo_script_governance_failures_current",
    ] {
        assert!(
            RECORDING_RULES_YAML.contains(emitted_metric),
            "recording rules must reference emitted metric {emitted_metric}"
        );
    }

    assert!(
        PROMETHEUS_CONFIG_YAML.contains("metrics_path: /metrics")
            && PROMETHEUS_CONFIG_YAML.contains("tikeo-recording-rules.yml")
            && PROMETHEUS_CONFIG_YAML.contains("tikeo-alert-rules.yml")
            && PROMETHEUS_CONFIG_YAML.contains("tikeo-server:9090"),
        "Prometheus config must scrape tikeo-server and load recording rules"
    );
}

#[test]
fn grafana_dashboard_uses_recording_rules_for_slo_panels() {
    for recording_series in [
        "tikeo:http_requests:rate5m",
        "tikeo:http_latency:p95_5m",
        "tikeo:http_errors:ratio5m",
        "tikeo:dispatch_pending_age:p95_5m",
        "tikeo:dispatch_latency:p95_5m",
        "tikeo:workflow_instance_duration:p95_5m",
        "tikeo:workflow_shard_duration:p95_5m",
        "tikeo:workflow_instance_success:ratio",
        "tikeo:workflow_shard_success:ratio",
    ] {
        assert!(
            DASHBOARD_JSON.contains(recording_series),
            "dashboard should query recording series {recording_series}"
        );
    }
}
