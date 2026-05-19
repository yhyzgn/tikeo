//! HTTP route handlers for the management API.

use axum::{Json, extract::Query};

use super::{
    dto::{
        ApiResponse, ClusterApiResponse, ClusterResponse, CreateJobRequest, ErrorResponse,
        JobPageApiResponse, Page, PageQuery, SystemInfoApiResponse, SystemInfoResponse,
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
#[utoipa::path(
    get,
    path = "/api/v1/jobs",
    tag = "jobs",
    params(PageQuery),
    responses((status = 200, description = "Job page", body = JobPageApiResponse))
)]
pub async fn list_jobs(Query(_query): Query<PageQuery>) -> Json<JobPageApiResponse> {
    Json(ApiResponse::success(Page {
        items: Vec::new(),
        next_page_token: None,
    }))
}

/// Create a job placeholder.
///
/// # Errors
///
/// Always returns a not implemented response until job persistence lands.
#[utoipa::path(
    post,
    path = "/api/v1/jobs",
    tag = "jobs",
    request_body = CreateJobRequest,
    responses(
        (status = 501, description = "Job persistence is not implemented yet", body = ErrorResponse)
    )
)]
pub async fn create_job(
    Json(_request): Json<CreateJobRequest>,
) -> Result<Json<JobPageApiResponse>, ApiError> {
    Err(ApiError::NotImplemented {
        message: "job persistence is not implemented yet".to_owned(),
    })
}
