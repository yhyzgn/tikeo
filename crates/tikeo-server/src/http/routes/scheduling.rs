use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use tikeo_storage::JobDurationHistory;

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, ErrorResponse, JobSchedulingAdviceApiResponse, JobSchedulingAdviceResponse,
        JobSchedulingHistorySummary, JobSchedulingPrediction, JobSchedulingWorkerCapacity,
    },
    error::ApiError,
};
use crate::tunnel::capability::WorkerRequirement;

/// Return operator-facing scheduling readiness advice for one job.
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors when advice inputs cannot be loaded.
#[utoipa::path(
    get,
    path = "/api/v1/jobs/{job}/scheduling-advice",
    tag = "jobs",
    params(("job" = String, Path, description = "Job identifier")),
    responses(
        (status = 200, description = "Job scheduling advice", body = JobSchedulingAdviceApiResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn job_scheduling_advice(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job): Path<String>,
) -> Result<Json<JobSchedulingAdviceApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "jobs", "read").await?;
    let job_summary = state
        .jobs
        .get(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &job_summary.namespace,
        &job_summary.app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app",
        ));
    }
    let requirement = required_requirement_for_job(&job_summary);
    let eligible_workers = state
        .registry
        .find_ordered_persisted_dispatch_workers(
            &job_summary.namespace,
            &job_summary.app,
            Some(&requirement),
        )
        .await;
    let worker_capacity = worker_capacity(&state, &eligible_workers).await;
    let instances = state
        .instances
        .list_by_job(&job_summary.id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let history = state
        .instances
        .duration_history(&job_summary.id, 500)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let recent_instances = instances.len().min(20);
    let recent_failures = instances
        .iter()
        .take(20)
        .filter(|instance| instance.status.to_string() == "failed")
        .count();
    let (ready, severity, reason) = advice_status(&eligible_workers, recent_failures);
    let history_summary = history_summary(&history);
    let prediction = prediction(&history_summary, worker_capacity);

    Ok(Json(ApiResponse::success(JobSchedulingAdviceResponse {
        ready,
        severity,
        reason,
        required_capability: Some(requirement.display_label()),
        eligible_workers,
        recent_instances: u64::try_from(recent_instances).unwrap_or(u64::MAX),
        recent_failures: u64::try_from(recent_failures).unwrap_or(u64::MAX),
        history: history_summary,
        prediction,
    })))
}

fn required_requirement_for_job(job: &tikeo_storage::JobSummary) -> WorkerRequirement {
    if job
        .script_id
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return WorkerRequirement::ScriptRunner {
            language: "*".to_owned(),
        };
    }
    if let Some(processor_type) = job
        .processor_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "sdk")
    {
        let processor_name = job
            .processor_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&job.name);
        return WorkerRequirement::PluginProcessor {
            processor_type: processor_type.to_owned(),
            processor_name: processor_name.to_owned(),
        };
    }
    let processor = job
        .processor_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&job.name);
    WorkerRequirement::NormalProcessor {
        name: processor.to_owned(),
    }
}

async fn worker_capacity(
    state: &AppState,
    eligible_workers: &[String],
) -> JobSchedulingWorkerCapacity {
    let mut advertised_cpu_cores = 0_u64;
    let mut advertised_memory_mb = 0_u64;
    for worker_id in eligible_workers {
        let Some(worker) = state
            .worker_lifecycle
            .get_online_current_worker(worker_id)
            .await
            .ok()
            .flatten()
        else {
            continue;
        };
        let labels =
            serde_json::from_str::<std::collections::HashMap<String, String>>(&worker.labels_json)
                .unwrap_or_default();
        advertised_cpu_cores = advertised_cpu_cores.saturating_add(label_u64(&labels, "cpu"));
        advertised_memory_mb = advertised_memory_mb.saturating_add(label_u64(&labels, "memory_mb"));
    }
    JobSchedulingWorkerCapacity {
        eligible_worker_count: u64::try_from(eligible_workers.len()).unwrap_or(u64::MAX),
        advertised_cpu_cores,
        advertised_memory_mb,
    }
}

const fn history_summary(history: &JobDurationHistory) -> JobSchedulingHistorySummary {
    JobSchedulingHistorySummary {
        inspected_instances: history.inspected_instances,
        completed_instances: history.completed_instances,
        failed_instances: history.failed_instances,
        average_duration_seconds: history.average_duration_seconds,
        p50_duration_seconds: history.p50_duration_seconds,
        p95_duration_seconds: history.p95_duration_seconds,
        max_duration_seconds: history.max_duration_seconds,
    }
}

fn prediction(
    history: &JobSchedulingHistorySummary,
    worker_capacity: JobSchedulingWorkerCapacity,
) -> JobSchedulingPrediction {
    let estimated_duration_seconds = if history.p95_duration_seconds > 0 {
        history.p95_duration_seconds
    } else {
        history.average_duration_seconds.max(1)
    };
    let recommended_concurrency = if worker_capacity.eligible_worker_count == 0 {
        0
    } else if estimated_duration_seconds >= 300 {
        1
    } else {
        worker_capacity.eligible_worker_count.clamp(1, 4)
    };
    let mut reasons = Vec::new();
    reasons.push(format!(
        "history uses {} completed instance(s); p95={}s average={}s",
        history.completed_instances, history.p95_duration_seconds, history.average_duration_seconds
    ));
    reasons.push(format!(
        "capacity sees {} eligible worker(s), {} cpu core(s), {} MiB memory",
        worker_capacity.eligible_worker_count,
        worker_capacity.advertised_cpu_cores,
        worker_capacity.advertised_memory_mb
    ));
    JobSchedulingPrediction {
        estimated_duration_seconds,
        recommended_concurrency,
        worker_capacity,
        reasons,
    }
}

fn label_u64(labels: &std::collections::HashMap<String, String>, key: &str) -> u64 {
    labels
        .get(key)
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn advice_status(eligible_workers: &[String], recent_failures: usize) -> (bool, String, String) {
    if eligible_workers.is_empty() {
        return (
            false,
            "error".to_owned(),
            "no online worker advertises the required capability".to_owned(),
        );
    }
    if recent_failures > 0 {
        return (
            true,
            "warning".to_owned(),
            format!(
                "{} eligible worker(s), but recent failures exist",
                eligible_workers.len()
            ),
        );
    }
    (
        true,
        "ok".to_owned(),
        format!("{} eligible worker(s) online", eligible_workers.len()),
    )
}
