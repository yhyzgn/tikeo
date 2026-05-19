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

/// System information API envelope.
pub type SystemInfoApiResponse = ApiResponse<SystemInfoResponse>;

/// Cluster status API envelope.
pub type ClusterApiResponse = ApiResponse<ClusterResponse>;

/// Job page API envelope.
pub type JobPageApiResponse = ApiResponse<Page>;

/// Created job API envelope.
pub type JobApiResponse = ApiResponse<JobSummary>;

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

/// Create job request placeholder.
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
