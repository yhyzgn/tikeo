//! `OpenAPI` document generation.

use utoipa::OpenApi;

use super::{auth, dto, routes};

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
        auth::login,
        auth::me,
        auth::logout,
        routes::list_users,
        routes::create_user,
        routes::update_user,
        routes::delete_user,
        routes::list_jobs,
        routes::create_job,
        routes::trigger_job,
        routes::list_job_instances,
        routes::get_job_instance,
        routes::list_instance_attempts,
        routes::list_instance_logs,
    ),
    components(schemas(
        dto::ApiResponse<dto::SystemInfoResponse>,
        dto::ApiResponse<dto::ClusterResponse>,
        dto::ApiResponse<dto::AuthSession>,
        dto::ApiResponse<dto::MeResponse>,
        dto::ApiResponse<dto::EmptyData>,
        dto::ApiResponse<dto::Page>,
        dto::ApiResponse<dto::JobSummary>,
        dto::ApiResponse<dto::JobInstanceSummary>,
        dto::ApiResponse<dto::JobInstancePage>,
        dto::ApiResponse<dto::JobInstanceAttemptPage>,
        dto::ApiResponse<dto::JobInstanceLogPage>,
        dto::ApiResponse<scheduler_storage::UserSummary>,
        dto::ApiResponse<Vec<scheduler_storage::UserSummary>>,
        dto::ApiResponse<dto::ErrorData>,
        dto::CreateUserRequest,
        dto::UpdateUserRequest,
        scheduler_storage::UserSummary,
        dto::ErrorData,
        dto::Page,
        dto::SystemInfoResponse,
        dto::ClusterResponse,
        dto::LoginRequest,
        dto::AuthSession,
        dto::MeResponse,
        dto::EmptyData,
        dto::JobSummary,
        dto::JobInstanceSummary,
        dto::JobInstancePage,
        dto::JobInstanceAttemptSummary,
        dto::JobInstanceAttemptPage,
        dto::JobInstanceLogSummary,
        dto::JobInstanceLogPage,
        dto::CreateJobRequest,
        dto::TriggerJobRequest,
    )),
    tags(
        (name = "system", description = "System and cluster metadata"),
        (name = "auth", description = "Development authentication endpoints"),
        (name = "jobs", description = "Job management endpoints")
    )
)]
pub struct ApiDoc;
