//! HTTP route handlers for the management API.

use std::sync::Arc;

use axum::{Json, extract::Query, extract::State};
use scheduler_storage::CreateJob;

use super::{
    AppState,
    dto::{
        ApiResponse, ClusterApiResponse, ClusterResponse, CreateJobRequest, ErrorResponse,
        JobApiResponse, JobPageApiResponse, JobSummary, Page, PageQuery, SystemInfoApiResponse,
        SystemInfoResponse,
    },
    error::ApiError,
};

/// Return scheduler server build and API metadata.
#[utoipa::path(
    get,
    path = "/api/v1/system/info",
    tag = "system",
    responses((status = 200, description = "System info", body = SystemInfoApiResponse))
)]
pub async fn system_info() -> Json<SystemInfoApiResponse> {
    Json(ApiResponse::success(SystemInfoResponse {
        name: "scheduler",
        version: env!("CARGO_PKG_VERSION"),
        target: std::env::consts::OS,
    }))
}

/// Return the current cluster status placeholder.
#[utoipa::path(
    get,
    path = "/api/v1/cluster",
    tag = "system",
    responses((status = 200, description = "Cluster status", body = ClusterApiResponse))
)]
pub async fn cluster_status() -> Json<ClusterApiResponse> {
    Json(ApiResponse::success(ClusterResponse {
        mode: "standalone",
        role: "leader",
        nodes: 1,
    }))
}

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
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn create_job(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateJobRequest>,
) -> Result<Json<JobApiResponse>, ApiError> {
    let created = state
        .jobs
        .create_job(CreateJob {
            namespace: defaulted(request.namespace, "default"),
            app: defaulted(request.app, "default"),
            name: request.name,
            schedule_type: defaulted(request.schedule_type, "api"),
            schedule_expr: request.schedule_expr,
            enabled: request.enabled.unwrap_or(true),
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;

    Ok(Json(ApiResponse::success(JobSummary::from(created))))
}

impl From<scheduler_storage::JobSummary> for JobSummary {
    fn from(value: scheduler_storage::JobSummary) -> Self {
        Self {
            id: value.id,
            namespace: value.namespace,
            app: value.app,
            name: value.name,
            schedule_type: value.schedule_type,
            schedule_expr: value.schedule_expr,
            enabled: value.enabled,
        }
    }
}

fn defaulted(value: Option<String>, default: &str) -> String {
    value
        .filter(|item| !item.trim().is_empty())
        .unwrap_or_else(|| default.to_owned())
}
