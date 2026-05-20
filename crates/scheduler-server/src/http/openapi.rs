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
        routes::system::system_info,
        routes::system::cluster_status,
        auth::login,
        auth::me,
        auth::logout,
        routes::users::list_users,
        routes::users::create_user,
        routes::users::update_user,
        routes::users::delete_user,
        routes::scripts::list_scripts,
        routes::scripts::create_script,
        routes::scripts::get_script,
        routes::scripts::update_script,
        routes::scripts::delete_script,
        routes::jobs::list_jobs,
        routes::jobs::create_job,
        routes::jobs::trigger_job,
        routes::jobs::list_job_instances,
        routes::jobs::get_job_instance,
        routes::jobs::list_instance_attempts,
        routes::jobs::list_instance_logs,
        routes::audit::list_audit_logs,
    ),
    components(schemas(
        dto::ApiResponse<dto::SystemInfoResponse>,
        dto::ApiResponse<dto::ClusterResponse>,
        dto::ApiResponse<dto::AuthSession>,
        dto::ApiResponse<dto::MeResponse>,
        dto::ApiResponse<dto::AuditLogPage>,
        dto::ApiResponse<dto::EmptyData>,
        dto::ApiResponse<dto::Page>,
        dto::ApiResponse<dto::JobSummary>,
        dto::ApiResponse<dto::JobInstanceSummary>,
        dto::ApiResponse<dto::JobInstancePage>,
        dto::ApiResponse<dto::JobInstanceAttemptPage>,
        dto::ApiResponse<dto::JobInstanceLogPage>,
        dto::ApiResponse<dto::ScriptPage>,
        dto::ApiResponse<scheduler_storage::ScriptSummary>,
        dto::ApiResponse<scheduler_storage::UserSummary>,
        dto::ApiResponse<Vec<scheduler_storage::UserSummary>>,
        dto::ApiResponse<dto::ErrorData>,
        dto::CreateUserRequest,
        dto::UpdateUserRequest,
        dto::CreateScriptRequest,
        dto::UpdateScriptRequest,
        scheduler_storage::UserSummary,
        scheduler_storage::ScriptSummary,
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
        dto::ScriptPage,
        dto::CreateJobRequest,
        dto::AuditLogSummary,
        dto::AuditLogPage,
        dto::TriggerJobRequest,
    )),
    tags(
        (name = "system", description = "System and cluster metadata"),
        (name = "auth", description = "Development authentication endpoints"),
        (name = "scripts", description = "Script management endpoints"),
        (name = "jobs", description = "Job management endpoints"),
        (name = "audit", description = "Audit log endpoints")
    )
)]
pub struct ApiDoc;
