//! HTTP DTOs used by the management API.

#![allow(clippy::option_if_let_else)]

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Successful API code.
pub const SUCCESS_CODE: i32 = 0;

/// Generic API envelope. All business HTTP APIs must return this shape.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    /// Business status code. `0` means success; non-zero values mean failure.
    pub code: i32,
    /// Human-readable response information.
    pub message: String,
    /// Response data. This field is always present, even when it is `null`.
    pub data: Option<T>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    /// Build a successful response with non-null data.
    pub fn success(data: T) -> Self {
        Self {
            code: SUCCESS_CODE,
            message: "success".to_owned(),
            data: Some(data),
        }
    }
}

/// Empty response payload for operations that intentionally return no data.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EmptyData {}

/// Error details payload nested in the API envelope `data` field for failures.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ErrorData {
    /// Trace identifier used to correlate logs and client errors.
    pub trace_id: String,
    /// Optional structured error details.
    pub details: Option<serde_json::Value>,
}

/// Standard error envelope.
pub type ErrorResponse = ApiResponse<ErrorData>;

/// Login API envelope.
pub type LoginApiResponse = ApiResponse<AuthSession>;

/// Current principal API envelope.
pub type MeApiResponse = ApiResponse<MeResponse>;

/// Empty successful API envelope.
pub type EmptyApiResponse = ApiResponse<EmptyData>;

/// System information API envelope.
pub type SystemInfoApiResponse = ApiResponse<SystemInfoResponse>;

/// Cluster status API envelope.
pub type ClusterApiResponse = ApiResponse<ClusterResponse>;

/// Job page API envelope.
pub type JobPageApiResponse = ApiResponse<Page>;

/// Created job API envelope.
pub type JobApiResponse = ApiResponse<JobSummary>;

/// DTO for creating a new user via API.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    /// Unique username.
    pub username: String,
    /// Plaintext password.
    pub password: String,
    /// User role (e.g. "admin", "operator", "viewer").
    pub role: String,
}

/// DTO for updating a user via API.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    /// Optional plaintext password update.
    pub password: Option<String>,
    /// Optional role update.
    pub role: Option<String>,
}

/// User Management API response envelope.
pub type UserApiResponse = ApiResponse<scheduler_storage::UserSummary>;
/// User list API response envelope.
pub type UserListApiResponse = ApiResponse<Vec<scheduler_storage::UserSummary>>;

/// Job instance page API envelope.
pub type JobInstancePageApiResponse = ApiResponse<JobInstancePage>;

/// Job instance API envelope.
pub type JobInstanceApiResponse = ApiResponse<JobInstanceSummary>;

/// Job instance log page API envelope.
pub type JobInstanceLogPageApiResponse = ApiResponse<JobInstanceLogPage>;

/// Job instance attempt page API envelope.
pub type JobInstanceAttemptPageApiResponse = ApiResponse<JobInstanceAttemptPage>;

/// Generic page response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Page {
    /// Page items.
    pub items: Vec<JobSummary>,
    /// Token for the next page when more data is available.
    pub next_page_token: Option<String>,
}

/// Common list query parameters.
#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PageQuery {
    /// Maximum number of items to return.
    pub page_size: Option<u32>,
    /// Opaque page token returned by a previous list call.
    pub page_token: Option<String>,
}

/// System information shown by the management API.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SystemInfoResponse {
    /// API service name.
    pub name: &'static str,
    /// Server crate version.
    pub version: &'static str,
    /// Rust package target environment.
    pub target: &'static str,
}

/// Cluster status placeholder.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ClusterResponse {
    /// Cluster operating mode.
    pub mode: &'static str,
    /// Current node role.
    pub role: &'static str,
    /// Known server node count.
    pub nodes: u32,
}

/// Login request for the development admin account.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Username.
    pub username: String,
    /// Password.
    pub password: String,
}

/// Authenticated session returned by login.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AuthSession {
    /// Bearer token.
    pub token: String,
    /// Username.
    pub username: String,
    /// Granted roles.
    pub roles: Vec<String>,
    /// Granted permissions.
    pub permissions: Vec<scheduler_storage::PermissionSummary>,
}

/// Current authenticated principal.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MeResponse {
    /// Username.
    pub username: String,
    /// Granted roles.
    pub roles: Vec<String>,
    /// Granted permissions.
    pub permissions: Vec<scheduler_storage::PermissionSummary>,
}

/// Workflow definition API envelope.
pub type WorkflowApiResponse = ApiResponse<scheduler_storage::WorkflowSummary>;
/// Workflow list API envelope.
pub type WorkflowListApiResponse = ApiResponse<Vec<scheduler_storage::WorkflowSummary>>;
/// Workflow validation API envelope.
pub type WorkflowValidationApiResponse = ApiResponse<scheduler_storage::WorkflowValidationResult>;
/// Workflow instance API envelope.
pub type WorkflowInstanceApiResponse = ApiResponse<scheduler_storage::WorkflowInstanceSummary>;
/// Workflow advance API envelope.
pub type WorkflowAdvanceApiResponse = ApiResponse<scheduler_storage::AdvanceWorkflowResult>;
/// Workflow dry-run API envelope.
pub type WorkflowDryRunApiResponse = ApiResponse<WorkflowDryRunResponse>;

/// Workflow dry-run response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WorkflowDryRunResponse {
    /// DAG validation result.
    pub validation: scheduler_storage::WorkflowValidationResult,
    /// Nodes without incoming edges.
    pub start_nodes: Vec<String>,
    /// Total node count.
    pub node_count: usize,
    /// Total edge count.
    pub edge_count: usize,
}

/// Request to run a workflow.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct WorkflowRunRequest {
    /// Trigger type. Defaults to `api`.
    pub trigger_type: Option<String>,
}

/// Job summary DTO.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobSummary {
    /// Job identifier.
    pub id: String,
    /// Namespace name.
    pub namespace: String,
    /// Application name.
    pub app: String,
    /// Display name.
    pub name: String,
    /// Schedule type, for example `api`, `cron`, or `fixed_rate`.
    pub schedule_type: String,
    /// Optional schedule expression.
    pub schedule_expr: Option<String>,
    /// Job enabled flag.
    pub enabled: bool,
}

/// Create job request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateJobRequest {
    /// Namespace name. Defaults to `default` when omitted.
    pub namespace: Option<String>,
    /// Application name. Defaults to `default` when omitted.
    pub app: Option<String>,
    /// Display name.
    pub name: String,
    /// Schedule type. Defaults to `api` when omitted.
    pub schedule_type: Option<String>,
    /// Optional schedule expression for CRON/fixed-rate modes.
    pub schedule_expr: Option<String>,
    /// Enabled flag. Defaults to `true` when omitted.
    pub enabled: Option<bool>,
}

/// Trigger job request.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct TriggerJobRequest {
    /// Optional trigger source. Defaults to `api`.
    pub trigger_type: Option<String>,
    /// Optional execution mode. Defaults to `single`; `broadcast` fans out to all online workers.
    pub execution_mode: Option<String>,
}

/// Job instance page response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobInstancePage {
    /// Page items.
    pub items: Vec<JobInstanceSummary>,
    /// Token for the next page when more data is available.
    pub next_page_token: Option<String>,
}

/// Job instance summary DTO.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobInstanceSummary {
    /// Instance identifier.
    pub id: String,
    /// Parent job identifier.
    pub job_id: String,
    /// Current instance status.
    pub status: String,
    /// Trigger source.
    pub trigger_type: String,
    /// Execution mode.
    pub execution_mode: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Job instance attempt page response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobInstanceAttemptPage {
    /// Page items.
    pub items: Vec<JobInstanceAttemptSummary>,
    /// Token for the next page when more data is available.
    pub next_page_token: Option<String>,
}

/// Per-worker job instance attempt summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobInstanceAttemptSummary {
    /// Attempt identifier.
    pub id: String,
    /// Parent instance identifier.
    pub instance_id: String,
    /// Target worker identifier.
    pub worker_id: String,
    /// Current attempt status.
    pub status: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Job instance log page response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobInstanceLogPage {
    /// Page items.
    pub items: Vec<JobInstanceLogSummary>,
    /// Token for the next page when more data is available.
    pub next_page_token: Option<String>,
}

/// Script page API envelope.
pub type ScriptPageApiResponse = ApiResponse<ScriptPage>;

/// Script API envelope.
pub type ScriptApiResponse = ApiResponse<scheduler_storage::ScriptSummary>;

/// Script page response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScriptPage {
    /// Page items.
    pub items: Vec<scheduler_storage::ScriptSummary>,
    /// Token for the next page when more data is available.
    pub next_page_token: Option<String>,
}

/// Create script request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateScriptRequest {
    /// Display name.
    pub name: String,
    /// Script language.
    pub language: String,
    /// Semantic version.
    pub version: String,
    /// Script source content.
    pub content: String,
    /// Optional timeout seconds.
    pub timeout_seconds: Option<i64>,
    /// Optional max memory bytes.
    pub max_memory_bytes: Option<i64>,
    /// Whether network access is allowed.
    pub allow_network: Option<bool>,
    /// Allowed environment variable names.
    pub allowed_env_vars: Option<Vec<String>>,
}

/// Update script request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateScriptRequest {
    /// Optional name update.
    pub name: Option<String>,
    /// Optional language update.
    pub language: Option<String>,
    /// Optional version update.
    pub version: Option<String>,
    /// Optional content update.
    pub content: Option<String>,
    /// Optional status update.
    pub status: Option<String>,
    /// Optional timeout seconds update.
    pub timeout_seconds: Option<i64>,
    /// Optional max memory bytes update.
    pub max_memory_bytes: Option<i64>,
    /// Optional network policy update.
    pub allow_network: Option<bool>,
    /// Optional allowed environment variable names update.
    pub allowed_env_vars: Option<Vec<String>>,
}

/// Job instance log summary DTO.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobInstanceLogSummary {
    /// Log identifier.
    pub id: String,
    /// Parent instance identifier.
    pub instance_id: String,
    /// Worker identifier.
    pub worker_id: String,
    /// Log level.
    pub level: String,
    /// Log message.
    pub message: String,
    /// Worker-local monotonic sequence.
    pub sequence: i64,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
}

/// Response for a single script version.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScriptVersionApiResponse {
    /// Version data.
    #[schema(value_type = Object)]
    pub data: scheduler_storage::ScriptVersionSummary,
}

/// Response for a list of script versions.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScriptVersionListApiResponse {
    /// List of versions.
    #[schema(value_type = Vec<Object>)]
    pub data: Vec<scheduler_storage::ScriptVersionSummary>,
}

/// Response for a script diff.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScriptDiffApiResponse {
    /// Diff result containing content and policy differences.
    pub data: ScriptDiffResult,
}

/// Diff result between two script versions.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScriptDiffResult {
    /// Unified diff of script content.
    pub content_diff: String,
    /// Policy field changes.
    pub policy_diff: Vec<FieldChange>,
}

/// A single field change between versions.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FieldChange {
    /// Field name.
    pub field: String,
    /// Value in version 1.
    pub before: String,
    /// Value in version 2.
    pub after: String,
}

/// Audit log page response.
pub type AuditLogPageApiResponse = ApiResponse<AuditLogPage>;

/// Paginated audit log list.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuditLogPage {
    /// Audit log entries.
    pub items: Vec<AuditLogSummary>,
    /// Opaque token for the next page.
    pub next_page_token: Option<String>,
}

/// Audit log summary for API responses.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuditLogSummary {
    /// Audit log identifier.
    pub id: String,
    /// Actor who performed the action.
    pub actor: String,
    /// Action performed.
    pub action: String,
    /// Resource type.
    pub resource_type: String,
    /// Resource identifier.
    pub resource_id: String,
    /// Optional detail.
    pub detail: Option<String>,
    /// Client IP address.
    pub ip_address: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
}

impl From<scheduler_storage::AuditLogSummary> for AuditLogSummary {
    fn from(value: scheduler_storage::AuditLogSummary) -> Self {
        Self {
            id: value.id,
            actor: value.actor,
            action: value.action,
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            detail: value.detail,
            ip_address: value.ip_address,
            created_at: value.created_at,
        }
    }
}
