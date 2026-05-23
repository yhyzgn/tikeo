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
    for metric in [
        "tikee_http_requests_total",
        "tikee_http_request_duration_seconds",
        "tikee_worker_connected_total",
        "tikee_worker_dispatch_total",
        "tikee_dispatch_queue_pending_age_seconds",
        "tikee_workflow_instance_duration_seconds",
        "tikee_workflow_shard_duration_seconds",
        "tikee_workflow_instance_success_ratio",
    ] {
        assert!(
            dashboard_text.contains(metric),
            "dashboard must include query for {metric}"
        );
    }

    Ok(())
}
