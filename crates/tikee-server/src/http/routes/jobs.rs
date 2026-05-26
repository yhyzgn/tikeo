use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use tikee_core::ExecutionMode;
use tikee_storage::{CreateJob, CreateJobInstance, UpdateJob};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, CreateJobRequest, DeleteJobApiResponse, EmptyData, ErrorResponse,
        JobApiResponse, JobInstanceApiResponse, JobInstanceAttemptPage,
        JobInstanceAttemptPageApiResponse, JobInstanceAttemptSummary, JobInstanceLogPage,
        JobInstanceLogPageApiResponse, JobInstanceLogSummary, JobInstancePage,
        JobInstancePageApiResponse, JobInstanceSummary, JobPageApiResponse, JobSummary, Page,
        PageQuery, TriggerJobRequest, UpdateJobRequest,
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
    headers: HeaderMap,
    Query(_query): Query<PageQuery>,
) -> Result<Json<JobPageApiResponse>, ApiError> {
    let principal = if has_auth_header(&headers) {
        Some(auth::require_permission(&headers, &state, "jobs", "read").await?)
    } else {
        None
    };
    let items = state
        .jobs
        .list_jobs()
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .filter(|job| {
            principal.as_ref().is_none_or(|principal| {
                crate::http::access_scope::allows_resource(
                    &principal.scope_bindings,
                    &job.namespace,
                    &job.app,
                    None,
                )
            })
        })
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
    let namespace = defaulted(request.namespace, "default");
    let app = defaulted(request.app, "default");
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &namespace,
        &app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app",
        ));
    }
    let schedule_type = parse_schedule_type(request.schedule_type.as_deref().unwrap_or("api"))?;
    if has_concrete_binding(request.processor_name.as_ref())
        && has_concrete_binding(request.script_id.as_ref())
    {
        return Err(ApiError::bad_request(
            "processorName and scriptId are mutually exclusive",
        ));
    }
    let created = state
        .jobs
        .create_job(CreateJob {
            namespace,
            app,
            name: request.name.clone(),
            schedule_type: schedule_type.to_string(),
            schedule_expr: request.schedule_expr.clone(),
            processor_name: request.processor_name.clone(),
            script_id: request.script_id.clone(),
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

    Ok(Json(ApiResponse::success(
        instance_summary_with_latest_log(&state, instance).await?,
    )))
}

/// Update a job.
///
/// # Errors
///
/// Returns validation, authorization, not-found, or storage errors.
#[utoipa::path(
    patch,
    path = "/api/v1/jobs/{job}",
    tag = "jobs",
    params(("job" = String, Path, description = "Job identifier")),
    request_body = UpdateJobRequest,
    responses(
        (status = 200, description = "Updated job", body = JobApiResponse),
        (status = 400, description = "Invalid schedule type", body = ErrorResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn update_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job): Path<String>,
    Json(request): Json<UpdateJobRequest>,
) -> Result<Json<JobApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "jobs", "write").await?;
    let current = state
        .jobs
        .get(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &current.namespace,
        &current.app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app",
        ));
    }
    let schedule_type = request
        .schedule_type
        .as_deref()
        .map(parse_schedule_type)
        .transpose()?
        .map(|value| value.to_string());
    if request
        .processor_name
        .as_ref()
        .and_then(std::option::Option::as_ref)
        .is_some_and(|value| has_concrete_binding(Some(value)))
        && request
            .script_id
            .as_ref()
            .and_then(std::option::Option::as_ref)
            .is_some_and(|value| has_concrete_binding(Some(value)))
    {
        return Err(ApiError::bad_request(
            "processorName and scriptId are mutually exclusive",
        ));
    }
    let updated = state
        .jobs
        .update_job(
            &job,
            UpdateJob {
                name: request.name.clone(),
                schedule_type,
                schedule_expr: request.schedule_expr.clone(),
                processor_name: request.processor_name.clone(),
                script_id: request.script_id.clone(),
                enabled: request.enabled,
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;

    audit(
        &state,
        &principal.username,
        "update",
        "job",
        &job,
        Some(format!("name={}", updated.name)),
        &headers,
    )
    .await;

    Ok(Json(ApiResponse::success(JobSummary::from(updated))))
}

/// Delete a job.
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors.
#[utoipa::path(
    delete,
    path = "/api/v1/jobs/{job}",
    tag = "jobs",
    params(("job" = String, Path, description = "Job identifier")),
    responses(
        (status = 200, description = "Deleted job", body = DeleteJobApiResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn delete_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job): Path<String>,
) -> Result<Json<DeleteJobApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "jobs", "write").await?;
    let current = state
        .jobs
        .get(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &current.namespace,
        &current.app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app",
        ));
    }
    let deleted = state
        .jobs
        .delete_job(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !deleted {
        return Err(ApiError::not_found(format!("job not found: {job}")));
    }

    audit(
        &state,
        &principal.username,
        "delete",
        "job",
        &job,
        Some(format!("name={}", current.name)),
        &headers,
    )
    .await;

    Ok(Json(ApiResponse::success(EmptyData {})))
}

fn has_auth_header(headers: &HeaderMap) -> bool {
    headers.contains_key(axum::http::header::AUTHORIZATION) || headers.contains_key("x-tikee-token")
}

fn has_concrete_binding(value: Option<&String>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
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
    let mut items = Vec::new();
    for instance in state
        .instances
        .list_by_job(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
    {
        items.push(instance_summary_with_latest_log(&state, instance).await?);
    }

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

    Ok(Json(ApiResponse::success(
        instance_summary_with_latest_log(&state, summary).await?,
    )))
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
            script_id: value.script_id,
            enabled: value.enabled,
        }
    }
}

async fn instance_summary_with_latest_log(
    state: &AppState,
    value: tikee_storage::JobInstanceSummary,
) -> Result<JobInstanceSummary, ApiError> {
    let log_count = state
        .logs
        .count_by_instance(&value.id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let latest_log = state
        .logs
        .latest_by_instance(&value.id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .map(JobInstanceLogSummary::from);
    let worker_id = state
        .logs
        .latest_worker_by_instance(&value.id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(JobInstanceSummary {
        id: value.id,
        job_id: value.job_id,
        status: value.status.to_string(),
        trigger_type: value.trigger_type.to_string(),
        execution_mode: value.execution_mode.to_string(),
        created_at: value.created_at,
        updated_at: value.updated_at,
        log_count,
        latest_log,
        worker_id,
    })
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
