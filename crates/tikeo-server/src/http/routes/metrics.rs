#![allow(missing_docs, clippy::missing_errors_doc)]

use std::{collections::BTreeMap, sync::Arc};

use axum::{Extension, Json, extract::State, http::HeaderMap};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, MetricsAlertSummary, MetricsGovernanceSummary, MetricsInstanceSummary,
        MetricsSummaryResponse, MetricsWorkerSummary,
    },
    error::ApiError,
};

#[utoipa::path(get, path = "/api/v1/metrics/summary", tag = "metrics")]
pub async fn metrics_summary(
    State(state): State<Arc<AppState>>,
    Extension(recorder): Extension<Arc<metrics_exporter_prometheus::PrometheusRecorder>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<MetricsSummaryResponse>>, ApiError> {
    auth::require_permission(&headers, &state, "system", "read").await?;
    let workers_online = state
        .worker_lifecycle
        .list_online_workers(500)
        .await
        .map(|workers| u64::try_from(workers.len()).unwrap_or(u64::MAX))
        .map_err(|error| ApiError::storage(&error))?;
    let instances = state
        .instances
        .count_by_status()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let alert_counts = state
        .alerts
        .count_events()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let queue = state
        .workflows
        .dispatch_queue_slo_summary()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let outbox = state
        .worker_dispatch_outbox
        .summary()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let shard_ownership = state
        .shard_ownership
        .summary()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let workflows = state
        .workflows
        .workflow_slo_summary()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    metrics::with_local_recorder(&*recorder, || {
        record_dispatch_queue_metrics(&queue);
        record_worker_dispatch_outbox_metrics(&outbox);
        record_shard_ownership_metrics(&shard_ownership);
        record_business_slo_metrics(
            workers_online,
            instances.total,
            &instances.by_status,
            &alert_counts.by_status,
            alert_counts.script_failure_events,
            &alert_counts.by_failure_class,
        );
        record_workflow_slo_metrics(&workflows);
    });

    Ok(Json(ApiResponse::success(MetricsSummaryResponse {
        workers: MetricsWorkerSummary {
            online: workers_online,
        },
        instances: MetricsInstanceSummary {
            total: instances.total,
            by_status: instances.by_status,
        },
        alerts: MetricsAlertSummary {
            total_events: alert_counts.total_events,
            by_status: alert_counts.by_status,
        },
        governance: MetricsGovernanceSummary {
            script_failure_events: alert_counts.script_failure_events,
            by_failure_class: alert_counts.by_failure_class,
        },
        queue,
        outbox,
        shard_ownership,
        workflows,
    })))
}

fn record_shard_ownership_metrics(summary: &tikeo_storage::ClusterShardOwnershipSloSummary) {
    metrics::gauge!("tikeo_cluster_shard_ownership_rows_total")
        .set(u64_metric_value(summary.total));
    metrics::gauge!("tikeo_cluster_shard_ownership_active_total")
        .set(u64_metric_value(summary.active));
    metrics::gauge!("tikeo_cluster_shard_ownership_max_epoch")
        .set(summary.max_epoch.to_string().parse::<f64>().unwrap_or(0.0));
    for (owner, count) in &summary.active_by_owner {
        metrics::gauge!(
            "tikeo_cluster_shard_ownership_active_by_owner",
            "owner_node_id" => owner.clone()
        )
        .set(u64_metric_value(*count));
    }
}

fn record_worker_dispatch_outbox_metrics(outbox: &tikeo_storage::WorkerDispatchOutboxSloSummary) {
    metrics::gauge!("tikeo_worker_dispatch_outbox_rows_total").set(u64_metric_value(outbox.total));
    metrics::gauge!("tikeo_worker_dispatch_outbox_oldest_queued_age_seconds")
        .set(u64_metric_value(outbox.oldest_queued_age_seconds));
    for (status, count) in &outbox.by_status {
        metrics::gauge!("tikeo_worker_dispatch_outbox_rows", "status" => status.clone())
            .set(u64_metric_value(*count));
    }
}

fn record_dispatch_queue_metrics(queue: &tikeo_storage::DispatchQueueSloSummary) {
    let oldest = std::time::Duration::from_secs(queue.oldest_pending_age_seconds).as_secs_f64();
    let average = std::time::Duration::from_secs(queue.average_pending_age_seconds).as_secs_f64();
    metrics::histogram!("tikeo_dispatch_queue_pending_age_seconds", "stat" => "oldest")
        .record(oldest);
    metrics::histogram!("tikeo_dispatch_queue_pending_age_seconds", "stat" => "average")
        .record(average);
    metrics::histogram!("tikeo_dispatch_queue_dispatch_latency_seconds", "stat" => "average")
        .record(std::time::Duration::from_secs(
            queue.average_dispatch_latency_seconds,
        ));
    metrics::histogram!("tikeo_dispatch_queue_dispatch_latency_seconds", "stat" => "longest")
        .record(std::time::Duration::from_secs(
            queue.longest_dispatch_latency_seconds,
        ));
    metrics::gauge!("tikeo_dispatch_queue_completed_total")
        .set(u64_metric_value(queue.completed_dispatches));
    metrics::gauge!("tikeo_dispatch_queue_items_total", "status" => "pending")
        .set(u64_metric_value(queue.pending));
    metrics::gauge!("tikeo_dispatch_queue_items_total", "status" => "running")
        .set(u64_metric_value(queue.running));
}

fn record_business_slo_metrics(
    workers_online: u64,
    instances_total: u64,
    instances_by_status: &BTreeMap<String, u64>,
    alerts_by_status: &BTreeMap<String, u64>,
    script_failure_events: u64,
    script_failures_by_class: &BTreeMap<String, u64>,
) {
    metrics::gauge!("tikeo_workers_online_current").set(u64_metric_value(workers_online));
    metrics::gauge!("tikeo_job_instances_current", "status" => "all")
        .set(u64_metric_value(instances_total));
    for (status, count) in instances_by_status {
        metrics::gauge!("tikeo_job_instances_current", "status" => status.clone())
            .set(u64_metric_value(*count));
    }
    metrics::gauge!("tikeo_job_instance_success_ratio")
        .set(instance_success_ratio(instances_by_status));

    for (status, count) in alerts_by_status {
        metrics::gauge!("tikeo_alert_events_current", "status" => status.clone())
            .set(u64_metric_value(*count));
    }
    metrics::gauge!("tikeo_script_governance_failures_current", "failure_class" => "all")
        .set(u64_metric_value(script_failure_events));
    for (failure_class, count) in script_failures_by_class {
        metrics::gauge!("tikeo_script_governance_failures_current", "failure_class" => failure_class.clone())
            .set(u64_metric_value(*count));
    }
}

fn record_workflow_slo_metrics(workflows: &tikeo_storage::WorkflowSloSummary) {
    metrics::gauge!("tikeo_workflow_instances_current", "status" => "all")
        .set(u64_metric_value(workflows.instances_total));
    for (status, count) in &workflows.instances_by_status {
        metrics::gauge!("tikeo_workflow_instances_current", "status" => status.clone())
            .set(u64_metric_value(*count));
    }
    metrics::gauge!("tikeo_workflow_instance_success_ratio").set(workflows.instance_success_ratio);
    metrics::histogram!("tikeo_workflow_instance_duration_seconds", "stat" => "average").record(
        std::time::Duration::from_secs(workflows.average_instance_duration_seconds),
    );
    metrics::histogram!("tikeo_workflow_instance_duration_seconds", "stat" => "longest").record(
        std::time::Duration::from_secs(workflows.longest_instance_duration_seconds),
    );

    metrics::gauge!("tikeo_workflow_shards_current", "status" => "all")
        .set(u64_metric_value(workflows.shards_total));
    for (status, count) in &workflows.shards_by_status {
        metrics::gauge!("tikeo_workflow_shards_current", "status" => status.clone())
            .set(u64_metric_value(*count));
    }
    metrics::gauge!("tikeo_workflow_shard_success_ratio").set(workflows.shard_success_ratio);
    metrics::histogram!("tikeo_workflow_shard_duration_seconds", "stat" => "average").record(
        std::time::Duration::from_secs(workflows.average_shard_duration_seconds),
    );
    metrics::histogram!("tikeo_workflow_shard_duration_seconds", "stat" => "longest").record(
        std::time::Duration::from_secs(workflows.longest_shard_duration_seconds),
    );
}

fn instance_success_ratio(instances_by_status: &BTreeMap<String, u64>) -> f64 {
    let succeeded = *instances_by_status.get("succeeded").unwrap_or(&0);
    let failed = instances_by_status
        .get("failed")
        .copied()
        .unwrap_or(0)
        .saturating_add(
            instances_by_status
                .get("partial_failed")
                .copied()
                .unwrap_or(0),
        );
    let terminal = succeeded.saturating_add(failed);
    if terminal == 0 {
        return 1.0;
    }
    u64_metric_value(succeeded) / u64_metric_value(terminal)
}

#[allow(clippy::cast_precision_loss)]
const fn u64_metric_value(value: u64) -> f64 {
    // The metrics crate exposes gauge values as f64; Prometheus queue gauges are
    // operational signals, so very large u64 counts may be rounded at scrape
    // time rather than rejected or saturated.
    value as f64
}
