use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use tikee_core::ExecutionMode;
use tikee_storage::{CreateJob, CreateJobInstance};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, CreateJobRequest, ErrorResponse, JobApiResponse, JobInstanceApiResponse,
        JobInstanceAttemptPage, JobInstanceAttemptPageApiResponse, JobInstanceAttemptSummary,
        JobInstanceLogPage, JobInstanceLogPageApiResponse, JobInstanceLogSummary, JobInstancePage,
        JobInstancePageApiResponse, JobInstanceSummary, JobPageApiResponse, JobSummary, Page,
        PageQuery, TriggerJobRequest,
    },
    error::ApiError,
};

use super::common::{
    audit, defaulted, parse_execution_mode, parse_schedule_type, parse_trigger_path,
    parse_trigger_type,
};

/// List jobs.
///
/// # Errors
///
/// Returns a storage error envelope when the repository query fails.
#[utoipa::path(
    get,
    path = "/api/v1/jobs",
    tag = "jobs",
    params(PageQuery),
    responses(
        (status = 200, description = "Job page", body = JobPageApiResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn list_jobs(
    State(state): State<Arc<AppState>>,
    Query(_query): Query<PageQuery>,
) -> Result<Json<JobPageApiResponse>, ApiError> {
    let items = state
        .jobs
        .list_jobs()
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(JobSummary::from)
        .collect();

    Ok(Json(ApiResponse::success(Page {
        items,
        next_page_token: None,
    })))
}

/// Create a job.
///
/// # Errors
///
/// Returns a storage error envelope when the job cannot be created.
#[utoipa::path(
    post,
    path = "/api/v1/jobs",
    tag = "jobs",
    request_body = CreateJobRequest,
    responses(
        (status = 200, description = "Created job", body = JobApiResponse),
        (status = 400, description = "Invalid schedule type", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn create_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateJobRequest>,
) -> Result<Json<JobApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "jobs", "write").await?;
    let schedule_type = parse_schedule_type(request.schedule_type.as_deref().unwrap_or("api"))?;
    let created = state
        .jobs
        .create_job(CreateJob {
            namespace: defaulted(request.namespace, "default"),
            app: defaulted(request.app, "default"),
            name: request.name.clone(),
            schedule_type: schedule_type.to_string(),
            schedule_expr: request.schedule_expr.clone(),
            processor_name: request.processor_name.clone(),
            enabled: request.enabled.unwrap_or(true),
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;

    audit(
        &state,
        &principal.username,
        "create",
        "job",
        &created.id,
        Some(format!("name={}", created.name)),
        &headers,
    )
    .await;

    Ok(Json(ApiResponse::success(JobSummary::from(created))))
}

/// Trigger a job and create a pending instance.
///
/// # Errors
///
/// Returns a validation, not-found, or storage error envelope when triggering fails.
#[utoipa::path(
    post,
    path = "/api/v1/jobs/{job}:trigger",
    tag = "jobs",
    params(("job" = String, Path, description = "Job identifier")),
    request_body = TriggerJobRequest,
    responses(
        (status = 200, description = "Created pending job instance", body = JobInstanceApiResponse),
        (status = 400, description = "Invalid trigger or execution mode", body = ErrorResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn trigger_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job_action): Path<String>,
    Json(request): Json<TriggerJobRequest>,
) -> Result<Json<JobInstanceApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "instances", "execute").await?;
    let job = parse_trigger_path(&job_action)?;

    let job_summary = state
        .jobs
        .get(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;

    let trigger_type = parse_trigger_type(request.trigger_type.as_deref().unwrap_or("api"))?;
    let execution_mode =
        parse_execution_mode(request.execution_mode.as_deref().unwrap_or("single"))?;
    let broadcast_worker_ids = if execution_mode == ExecutionMode::Broadcast {
        let worker_ids = state
            .registry
            .find_eligible_workers(&job_summary.namespace, &job_summary.app)
            .await;
        if worker_ids.is_empty() {
            return Err(ApiError::bad_request(
                "broadcast execution requires at least one eligible online worker",
            ));
        }
        Some(worker_ids)
    } else {
        None
    };

    let instance = state
        .instances
        .create_pending(CreateJobInstance {
            job_id: job.clone(),
            trigger_type,
            execution_mode,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;

    if let Some(worker_ids) = broadcast_worker_ids {
        state
            .attempts
            .create_pending_for_workers(&instance.id, &worker_ids)
            .await
            .map_err(|error| ApiError::storage(&error))?;
    }

    audit(
        &state,
        &principal.username,
        "trigger",
        "job",
        &job,
        Some(format!("instance={}", instance.id)),
        &headers,
    )
    .await;

    Ok(Json(ApiResponse::success(JobInstanceSummary::from(
        instance,
    ))))
}

/// List instances for a job.
///
/// # Errors
///
/// Returns a storage error envelope when repository access fails.
#[utoipa::path(
    get,
    path = "/api/v1/jobs/{job}/instances",
    tag = "jobs",
    params(
        ("job" = String, Path, description = "Job identifier"),
        PageQuery,
    ),
    responses(
        (status = 200, description = "Job instance page", body = JobInstancePageApiResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn list_job_instances(
    State(state): State<Arc<AppState>>,
    Path(job): Path<String>,
    Query(_query): Query<PageQuery>,
) -> Result<Json<JobInstancePageApiResponse>, ApiError> {
    let items = state
        .instances
        .list_by_job(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(JobInstanceSummary::from)
        .collect();

    Ok(Json(ApiResponse::success(JobInstancePage {
        items,
        next_page_token: None,
    })))
}

/// Get one job instance.
///
/// # Errors
///
/// Returns a not-found or storage error envelope when lookup fails.
#[utoipa::path(
    get,
    path = "/api/v1/instances/{instance}",
    tag = "jobs",
    params(("instance" = String, Path, description = "Instance identifier")),
    responses(
        (status = 200, description = "Job instance", body = JobInstanceApiResponse),
        (status = 404, description = "Instance not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn get_job_instance(
    State(state): State<Arc<AppState>>,
    Path(instance): Path<String>,
) -> Result<Json<JobInstanceApiResponse>, ApiError> {
    let summary = state
        .instances
        .get(&instance)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("instance not found: {instance}")))?;

    Ok(Json(ApiResponse::success(JobInstanceSummary::from(
        summary,
    ))))
}

/// List broadcast attempts for one job instance.
///
/// # Errors
///
/// Returns a storage error envelope when repository access fails.
#[utoipa::path(
    get,
    path = "/api/v1/instances/{instance}/attempts",
    tag = "jobs",
    params(
        ("instance" = String, Path, description = "Instance identifier"),
        PageQuery,
    ),
    responses(
        (status = 200, description = "Job instance attempt page", body = JobInstanceAttemptPageApiResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn list_instance_attempts(
    State(state): State<Arc<AppState>>,
    Path(instance): Path<String>,
    Query(_query): Query<PageQuery>,
) -> Result<Json<JobInstanceAttemptPageApiResponse>, ApiError> {
    let items = state
        .attempts
        .list_by_instance(&instance)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(JobInstanceAttemptSummary::from)
        .collect();

    Ok(Json(ApiResponse::success(JobInstanceAttemptPage {
        items,
        next_page_token: None,
    })))
}

/// List logs for one job instance.
///
/// # Errors
///
/// Returns a storage error envelope when repository access fails.
#[utoipa::path(
    get,
    path = "/api/v1/instances/{instance}/logs",
    tag = "jobs",
    params(
        ("instance" = String, Path, description = "Instance identifier"),
        PageQuery,
    ),
    responses(
        (status = 200, description = "Job instance log page", body = JobInstanceLogPageApiResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn list_instance_logs(
    State(state): State<Arc<AppState>>,
    Path(instance): Path<String>,
    Query(query): Query<PageQuery>,
) -> Result<Json<JobInstanceLogPageApiResponse>, ApiError> {
    let items = state
        .logs
        .list_by_instance(&instance)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(JobInstanceLogSummary::from)
        .filter(|log| {
            query.page_token.as_deref() != Some("script_execution_governance")
                || log.governance_event.as_deref() == Some("script_execution_governance")
        })
        .collect();

    Ok(Json(ApiResponse::success(JobInstanceLogPage {
        items,
        next_page_token: None,
    })))
}

impl From<tikee_storage::JobSummary> for JobSummary {
    fn from(value: tikee_storage::JobSummary) -> Self {
        Self {
            id: value.id,
            namespace: value.namespace,
            app: value.app,
            name: value.name,
            schedule_type: value.schedule_type,
            schedule_expr: value.schedule_expr,
            processor_name: value.processor_name,
            enabled: value.enabled,
        }
    }
}

impl From<tikee_storage::JobInstanceSummary> for JobInstanceSummary {
    fn from(value: tikee_storage::JobInstanceSummary) -> Self {
        Self {
            id: value.id,
            job_id: value.job_id,
            status: value.status.to_string(),
            trigger_type: value.trigger_type.to_string(),
            execution_mode: value.execution_mode.to_string(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<tikee_storage::JobInstanceAttemptSummary> for JobInstanceAttemptSummary {
    fn from(value: tikee_storage::JobInstanceAttemptSummary) -> Self {
        Self {
            id: value.id,
            instance_id: value.instance_id,
            worker_id: value.worker_id,
            status: value.status.to_string(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<tikee_storage::JobInstanceLogSummary> for JobInstanceLogSummary {
    fn from(value: tikee_storage::JobInstanceLogSummary) -> Self {
        let governance = parse_log_governance(&value.message);
        Self {
            id: value.id,
            instance_id: value.instance_id,
            worker_id: value.worker_id,
            level: value.level,
            message: governance
                .as_ref()
                .and_then(|parsed| parsed.message.clone())
                .unwrap_or(value.message),
            governance_event: governance.as_ref().map(|parsed| parsed.event.clone()),
            governance_failure_class: governance
                .as_ref()
                .and_then(|parsed| parsed.failure_class.clone()),
            governance_message: governance.and_then(|parsed| parsed.message),
            sequence: value.sequence,
            created_at: value.created_at,
        }
    }
}

struct ParsedLogGovernance {
    event: String,
    failure_class: Option<String>,
    message: Option<String>,
}

fn parse_log_governance(message: &str) -> Option<ParsedLogGovernance> {
    let value = serde_json::from_str::<serde_json::Value>(message).ok()?;
    let event = value.get("event")?.as_str()?.to_owned();
    if event != "script_execution_governance" {
        return None;
    }
    Some(ParsedLogGovernance {
        event,
        failure_class: value
            .get("failure_class")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        message: value
            .get("message")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    })
}
