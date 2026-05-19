//! `OpenAPI` document generation.

use utoipa::OpenApi;

use super::{dto, routes};

/// scheduler management `OpenAPI` document.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "scheduler Management API",
        version = env!("CARGO_PKG_VERSION"),
        description = "HTTP management API for the scheduler platform."
    ),
    paths(
        routes::system_info,
        routes::cluster_status,
        routes::list_jobs,
        routes::create_job,
    ),
    components(schemas(
        dto::ApiResponse<dto::SystemInfoResponse>,
        dto::ApiResponse<dto::ClusterResponse>,
        dto::ApiResponse<dto::Page>,
        dto::ApiResponse<dto::JobSummary>,
        dto::ApiResponse<dto::ErrorData>,
        dto::ErrorData,
        dto::Page,
        dto::SystemInfoResponse,
        dto::ClusterResponse,
        dto::JobSummary,
        dto::CreateJobRequest,
    )),
    tags(
        (name = "system", description = "System and cluster metadata"),
        (name = "jobs", description = "Job management endpoints")
    )
)]
pub struct ApiDoc;
