//! HTTP DTOs used by the management API.

use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use utoipa::{PartialSchema, ToSchema};

/// `SUCCESS_CODE` constant.
pub const SUCCESS_CODE: i32 = 0;

/// Tri-state string patch field: absent leaves the value unchanged, null clears it, and string sets it.
#[derive(Debug, Clone, Default, ToSchema)]
pub enum NullableStringUpdate {
    /// Field was absent from the PATCH payload.
    #[default]
    Unset,
    /// Field was present as JSON null and should clear the stored value.
    Null,
    /// Field was present as a string and should set the stored value.
    Value(String),
}

impl NullableStringUpdate {
    /// Convert to repository update shape.
    #[must_use]
    pub fn into_option_option(self) -> Option<Option<String>> {
        match self {
            Self::Unset => None,
            Self::Null => Some(None),
            Self::Value(value) => Some(Some(value)),
        }
    }

    /// Resolve this patch against an existing optional string reference.
    #[must_use]
    pub const fn resolve<'a>(&'a self, existing: Option<&'a str>) -> Option<&'a str> {
        match self {
            Self::Unset => existing,
            Self::Null => None,
            Self::Value(value) => Some(value.as_str()),
        }
    }

    /// Clone this patch into an optional value, falling back to an existing owned option.
    #[must_use]
    pub fn clone_or(&self, existing: Option<String>) -> Option<String> {
        match self {
            Self::Unset => existing,
            Self::Null => None,
            Self::Value(value) => Some(value.clone()),
        }
    }
}

impl<'de> Deserialize<'de> for NullableStringUpdate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer)
            .map(|value| value.map_or(Self::Null, Self::Value))
    }
}

/// Tri-state JSON patch field: absent leaves unchanged, null clears, and a JSON value sets it.
#[derive(Debug, Clone, Default, ToSchema)]
pub enum NullableJsonUpdate {
    /// Field was absent from the PATCH payload.
    #[default]
    Unset,
    /// Field was present as JSON null and should clear the stored value.
    Null,
    /// Field was present as a JSON value and should set the stored value.
    Value(serde_json::Value),
}

impl NullableJsonUpdate {
    /// Convert to a repository update with serialized JSON.
    #[must_use]
    pub fn into_json_option_option(self) -> Option<Option<String>> {
        match self {
            Self::Unset => None,
            Self::Null => Some(None),
            Self::Value(value) => Some(Some(
                serde_json::to_string(&value).unwrap_or_else(|_| "null".to_owned()),
            )),
        }
    }

    /// Resolve this patch against an existing optional JSON value.
    #[must_use]
    pub const fn resolve<'a>(
        &'a self,
        existing: Option<&'a serde_json::Value>,
    ) -> Option<&'a serde_json::Value> {
        match self {
            Self::Unset => existing,
            Self::Null => None,
            Self::Value(value) => Some(value),
        }
    }
}

impl<'de> Deserialize<'de> for NullableJsonUpdate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<serde_json::Value>::deserialize(deserializer)
            .map(|value| value.map_or(Self::Null, Self::Value))
    }
}

#[derive(Debug, Clone, Serialize)]
/// `ApiResponse` payload.
pub struct ApiResponse<T>
where
    T: Serialize,
{
    /// code value.
    pub code: i32,
    /// message value.
    pub message: String,
    /// data value.
    pub data: Option<T>,
}

impl<T> utoipa::ToSchema for ApiResponse<T>
where
    T: Serialize + utoipa::__dev::ComposeSchema,
{
    fn name() -> Cow<'static, str> {
        Cow::Borrowed("ApiResponse")
    }
}

impl<T> utoipa::__dev::ComposeSchema for ApiResponse<T>
where
    T: Serialize + utoipa::__dev::ComposeSchema,
{
    fn compose(
        _: Vec<utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>>,
    ) -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::schema::ObjectBuilder::new()
            .property("code", i32::schema())
            .required("code")
            .property("message", String::schema())
            .required("message")
            .property("data", Option::<T>::schema())
            .build()
            .into()
    }
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    /// Success.
    pub fn success(data: T) -> Self {
        Self {
            code: SUCCESS_CODE,
            message: "success".to_owned(),
            data: Some(data),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `EmptyData` payload.
pub struct EmptyData {}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `ErrorData` payload.
pub struct ErrorData {
    /// Trace id value.
    pub trace_id: String,
    /// Details value.
    pub details: Option<serde_json::Value>,
}

/// `ErrorResponse` type alias.
pub type ErrorResponse = ApiResponse<ErrorData>;

/// `LoginApiResponse` type alias.
pub type LoginApiResponse = ApiResponse<AuthSession>;

/// `AuthStatusApiResponse` type alias.
pub type AuthStatusApiResponse = ApiResponse<AuthStatusResponse>;
/// `BootstrapStatusApiResponse` type alias.
pub type BootstrapStatusApiResponse = ApiResponse<BootstrapStatusResponse>;
/// `OidcAuthorizeApiResponse` type alias.
pub type OidcAuthorizeApiResponse = ApiResponse<OidcAuthorizeResponse>;

/// `MeApiResponse` type alias.
pub type MeApiResponse = ApiResponse<MeResponse>;

/// `EmptyApiResponse` type alias.
pub type EmptyApiResponse = ApiResponse<EmptyData>;

/// `SystemInfoApiResponse` type alias.
pub type SystemInfoApiResponse = ApiResponse<SystemInfoResponse>;

/// `ClusterApiResponse` type alias.
pub type ClusterApiResponse = ApiResponse<ClusterResponse>;
/// `ClusterDiagnosticsApiResponse` type alias.
pub type ClusterDiagnosticsApiResponse = ApiResponse<ClusterDiagnosticsResponse>;
/// `TransportSecurityStatusApiResponse` type alias.
pub type TransportSecurityStatusApiResponse = ApiResponse<TransportSecurityStatusResponse>;
/// `ObservabilityStatusApiResponse` type alias.
pub type ObservabilityStatusApiResponse = ApiResponse<ObservabilityStatusResponse>;

/// `JobPageApiResponse` type alias.
pub type JobPageApiResponse = ApiResponse<Page>;

/// `JobApiResponse` type alias.
pub type JobApiResponse = ApiResponse<JobSummary>;

/// `DeleteJobApiResponse` type alias.
pub type DeleteJobApiResponse = ApiResponse<EmptyData>;

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `CreateUserRequest` payload.
pub struct CreateUserRequest {
    /// Username value.
    pub username: String,
    /// Email value.
    pub email: String,
    /// Password value.
    pub password: String,
    /// Role value.
    pub role: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `UpdateUserRequest` payload.
pub struct UpdateUserRequest {
    /// Email value.
    pub email: Option<String>,
    /// Password value.
    pub password: Option<String>,
    /// Role value.
    pub role: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `BootstrapRegisterRequest` payload.
pub struct BootstrapRegisterRequest {
    /// Username value.
    pub username: String,
    /// Email value.
    pub email: String,
    /// Password value.
    pub password: String,
    /// Confirm password value.
    pub confirm_password: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `BootstrapStatusResponse` payload.
pub struct BootstrapStatusResponse {
    /// Initialized value.
    pub initialized: bool,
    /// Registration open value.
    pub registration_open: bool,
    /// Bootstrap admin username value.
    pub bootstrap_admin_username: Option<String>,
}

/// `UserApiResponse` type alias.
pub type UserApiResponse = ApiResponse<tikeo_storage::UserSummary>;
/// `UserListApiResponse` type alias.
pub type UserListApiResponse = ApiResponse<Vec<tikeo_storage::UserSummary>>;

/// `JobInstancePageApiResponse` type alias.
pub type JobInstancePageApiResponse = ApiResponse<JobInstancePage>;

/// `JobInstanceApiResponse` type alias.
pub type JobInstanceApiResponse = ApiResponse<JobInstanceSummary>;
/// `JobInstanceCancelApiResponse` type alias.
pub type JobInstanceCancelApiResponse = ApiResponse<JobInstanceSummary>;

/// `JobInstanceLogPageApiResponse` type alias.
pub type JobInstanceLogPageApiResponse = ApiResponse<JobInstanceLogPage>;

/// `JobInstanceAttemptPageApiResponse` type alias.
pub type JobInstanceAttemptPageApiResponse = ApiResponse<JobInstanceAttemptPage>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `Page` payload.
pub struct Page {
    /// Items value.
    pub items: Vec<JobSummary>,
    /// Next page token value.
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
/// `PageQuery` payload.
pub struct PageQuery {
    /// Page size value.
    pub page_size: Option<u32>,
    /// Page token value.
    pub page_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `CreateAlertRuleRequest` payload.
pub struct CreateAlertRuleRequest {
    /// Name value.
    pub name: String,
    /// Severity value.
    pub severity: String,
    /// Condition value.
    pub condition: serde_json::Value,
    /// Channels value.
    pub channels: Vec<serde_json::Value>,
    /// Boolean state flag.
    pub enabled: bool,
    /// Dedupe seconds value.
    pub dedupe_seconds: Option<u64>,
    /// Silenced until value.
    pub silenced_until: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `AlertRuleSummary` payload.
pub struct AlertRuleSummary {
    /// Id value.
    pub id: String,
    /// Name value.
    pub name: String,
    /// Severity value.
    pub severity: String,
    /// Condition value.
    pub condition: serde_json::Value,
    /// Channels value.
    pub channels: Vec<serde_json::Value>,
    /// Boolean state flag.
    pub enabled: bool,
    /// Dedupe seconds value.
    pub dedupe_seconds: u64,
    /// Silenced until value.
    pub silenced_until: Option<String>,
    /// Created at value.
    pub created_at: String,
    /// Updated at value.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `AlertEventSummary` payload.
pub struct AlertEventSummary {
    /// Id value.
    pub id: String,
    /// Rule id value.
    pub rule_id: String,
    /// Rule name value.
    pub rule_name: String,
    /// Severity value.
    pub severity: String,
    /// Status value.
    pub status: String,
    /// Event type value.
    pub event_type: String,
    /// Resource type value.
    pub resource_type: String,
    /// Resource id value.
    pub resource_id: String,
    /// Failure class value.
    pub failure_class: Option<String>,
    /// Message value.
    pub message: Option<String>,
    /// Dedupe key value.
    pub dedupe_key: String,
    /// Created at value.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `AlertDeliveryStatusResponse` payload.
pub struct AlertDeliveryStatusResponse {
    /// Rule id value.
    pub rule_id: String,
    /// Ready value.
    pub ready: bool,
    /// Channel count value.
    pub channel_count: u64,
    /// Channels value.
    pub channels: Vec<AlertDeliveryChannelStatus>,
    /// Issues value.
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `AlertDeliveryQueueStatusResponse` payload.
pub struct AlertDeliveryQueueStatusResponse {
    /// Total attempts value.
    pub total_attempts: u64,
    /// Delivered value.
    pub delivered: u64,
    /// Retry pending value.
    pub retry_pending: u64,
    /// Dead letter value.
    pub dead_letter: u64,
    /// Retry consumed value.
    pub retry_consumed: u64,
    /// Failed value.
    pub failed: u64,
    /// Recent dead letters value.
    pub recent_dead_letters: Vec<tikeo_storage::AlertDeliveryAttemptSummary>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `AlertDeliveryChannelStatus` payload.
pub struct AlertDeliveryChannelStatus {
    /// Provider value.
    pub provider: String,
    /// Target configured value.
    pub target_configured: bool,
    /// Secret configured value.
    pub secret_configured: bool,
    /// Boolean state flag.
    pub enabled: bool,
    /// Target redacted value.
    pub target_redacted: Option<String>,
    /// Transport security value.
    pub transport_security: Option<String>,
    /// Issues value.
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `AlertNotificationSummary` payload.
pub struct AlertNotificationSummary {
    /// Rule id value.
    pub rule_id: String,
    /// Rule name value.
    pub rule_name: String,
    /// Severity value.
    pub severity: String,
    /// Resource type value.
    pub resource_type: String,
    /// Resource id value.
    pub resource_id: String,
    /// Failure class value.
    pub failure_class: Option<String>,
    /// Latest status value.
    pub latest_status: String,
    /// Latest event type value.
    pub latest_event_type: String,
    /// Latest message value.
    pub latest_message: Option<String>,
    /// Event count value.
    pub event_count: u64,
    /// Firing count value.
    pub firing_count: u64,
    /// Suppressed count value.
    pub suppressed_count: u64,
    /// Silenced count value.
    pub silenced_count: u64,
    /// Recovered count value.
    pub recovered_count: u64,
    /// First seen value.
    pub first_seen: String,
    /// Last seen value.
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `MetricsSummaryResponse` payload.
pub struct MetricsSummaryResponse {
    /// Workers value.
    pub workers: MetricsWorkerSummary,
    /// Instances value.
    pub instances: MetricsInstanceSummary,
    /// Alerts value.
    pub alerts: MetricsAlertSummary,
    /// Governance value.
    pub governance: MetricsGovernanceSummary,
    /// Queue value.
    pub queue: tikeo_storage::DispatchQueueSloSummary,
    /// Outbox value.
    pub outbox: tikeo_storage::WorkerDispatchOutboxSloSummary,
    /// Shard ownership value.
    pub shard_ownership: tikeo_storage::ClusterShardOwnershipSloSummary,
    /// Workflows value.
    pub workflows: tikeo_storage::WorkflowSloSummary,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `MetricsWorkerSummary` payload.
pub struct MetricsWorkerSummary {
    /// Online value.
    pub online: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `MetricsInstanceSummary` payload.
pub struct MetricsInstanceSummary {
    /// Total value.
    pub total: u64,
    /// By status value.
    pub by_status: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `MetricsAlertSummary` payload.
pub struct MetricsAlertSummary {
    /// Total events value.
    pub total_events: u64,
    /// By status value.
    pub by_status: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `MetricsGovernanceSummary` payload.
pub struct MetricsGovernanceSummary {
    /// Script failure events value.
    pub script_failure_events: u64,
    /// By failure class value.
    pub by_failure_class: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
/// `AuditLogQuery` payload.
pub struct AuditLogQuery {
    /// Page size value.
    pub page_size: Option<u32>,
    /// Page token value.
    pub page_token: Option<String>,
    /// Actor value.
    pub actor: Option<String>,
    /// Action value.
    pub action: Option<String>,
    /// Resource type value.
    pub resource_type: Option<String>,
    /// Resource id value.
    pub resource_id: Option<String>,
    /// Failure reason value.
    pub failure_reason: Option<String>,
    /// Format value.
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `SystemInfoResponse` payload.
pub struct SystemInfoResponse {
    /// Name value.
    pub name: &'static str,
    /// Version value.
    pub version: &'static str,
    /// Target value.
    pub target: &'static str,
    /// Git tag value.
    pub git_tag: &'static str,
    /// Git sha value.
    pub git_sha: &'static str,
    /// Build time value.
    pub build_time: &'static str,
    /// Git dirty value.
    pub git_dirty: &'static str,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `ClusterResponse` payload.
pub struct ClusterResponse {
    /// Mode value.
    pub mode: String,
    /// Role value.
    pub role: String,
    /// Node id value.
    pub node_id: String,
    /// Nodes value.
    pub nodes: u32,
    /// Can schedule value.
    pub can_schedule: bool,
    /// Leader fencing token value.
    pub leader_fencing_token: Option<String>,
    /// Detail value.
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `ClusterDiagnosticsResponse` payload.
pub struct ClusterDiagnosticsResponse {
    /// Responding node value.
    pub responding_node: ClusterResponse,
    /// Status value.
    pub status: ClusterResponse,
    /// Scheduling gated value.
    pub scheduling_gated: bool,
    /// Serialized data value.
    pub metadata: Option<RaftMetadataDiagnostic>,
    /// Nodes value.
    pub nodes: Vec<ClusterNodeDiagnostic>,
    /// Members value.
    pub members: Vec<RaftMemberDiagnostic>,
    /// Transport value.
    pub transport: RaftTransportDiagnostic,
    /// Runtime boundary value.
    pub runtime_boundary: String,
    /// Smart gateway value.
    pub smart_gateway: SmartGatewayDiagnostic,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `SmartGatewayDiagnostic` payload.
pub struct SmartGatewayDiagnostic {
    /// Mode value.
    pub mode: &'static str,
    /// Status value.
    pub status: &'static str,
    /// Local gateway node id value.
    pub local_gateway_node_id: String,
    /// Online workers value.
    pub online_workers: u64,
    /// Local gateway workers value.
    pub local_gateway_workers: u64,
    /// Remote gateway workers value.
    pub remote_gateway_workers: u64,
    /// Outbox total value.
    pub outbox_total: u64,
    /// Queued or reroute pending value.
    pub queued_or_reroute_pending: u64,
    /// Oldest queued age seconds value.
    pub oldest_queued_age_seconds: u64,
    /// Safety boundary value.
    pub safety_boundary: &'static str,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `ClusterNodeDiagnostic` payload.
pub struct ClusterNodeDiagnostic {
    /// Node id value.
    pub node_id: String,
    /// Endpoint value.
    pub endpoint: String,
    /// Member status value.
    pub member_status: String,
    /// Current term value.
    pub current_term: Option<i64>,
    /// Commit index value.
    pub commit_index: Option<i64>,
    /// Applied index value.
    pub applied_index: Option<i64>,
    /// Leader fencing token value.
    pub leader_fencing_token: Option<String>,
    /// Is responding node value.
    pub is_responding_node: bool,
    /// Can schedule value.
    pub can_schedule: bool,
    /// Probe status value.
    pub probe_status: String,
    /// Observed role value.
    pub observed_role: Option<String>,
    /// Observed can schedule value.
    pub observed_can_schedule: Option<bool>,
    /// Probe latency ms value.
    pub probe_latency_ms: Option<u64>,
    /// Probe error value.
    pub probe_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `RaftMetadataDiagnostic` payload.
pub struct RaftMetadataDiagnostic {
    /// Cluster id value.
    pub cluster_id: String,
    /// Node id value.
    pub node_id: String,
    /// Current term value.
    pub current_term: i64,
    /// Voted for value.
    pub voted_for: Option<String>,
    /// Commit index value.
    pub commit_index: i64,
    /// Applied index value.
    pub applied_index: i64,
    /// Leader fencing token value.
    pub leader_fencing_token: Option<String>,
    /// Conf state value.
    pub conf_state: Option<String>,
    /// Updated at value.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `RaftMemberDiagnostic` payload.
pub struct RaftMemberDiagnostic {
    /// Node id value.
    pub node_id: String,
    /// Endpoint value.
    pub endpoint: String,
    /// Status value.
    pub status: String,
    /// Updated at value.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `RaftTransportDiagnostic` payload.
pub struct RaftTransportDiagnostic {
    /// Append entries path value.
    pub append_entries_path: &'static str,
    /// Mutating value.
    pub mutating: bool,
    /// Status value.
    pub status: &'static str,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `LoginRequest` payload.
pub struct LoginRequest {
    /// Username value.
    pub username: String,
    /// Password value.
    pub password: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `AuthSession` payload.
pub struct AuthSession {
    /// Token value.
    pub token: String,
    /// Username value.
    pub username: String,
    /// Roles value.
    pub roles: Vec<String>,
    /// Permissions value.
    pub permissions: Vec<tikeo_storage::PermissionSummary>,
    /// Bootstrap admin value.
    pub bootstrap_admin: bool,
    /// Scope limited value.
    pub scope_limited: bool,
    /// Token scopes value.
    pub token_scopes: Vec<String>,
    /// Scope bindings value.
    pub scope_bindings: Vec<AccessScopeBinding>,
    /// Menu keys value.
    pub menu_keys: Vec<String>,
    /// Ui action keys value.
    pub ui_action_keys: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `CreateApiTokenRequest` payload.
pub struct CreateApiTokenRequest {
    /// Name value.
    pub name: String,
    /// Scopes value.
    pub scopes: Option<Vec<String>>,
    /// Scope bindings value.
    pub scope_bindings: Option<Vec<AccessScopeBinding>>,
    /// Expires in seconds value.
    pub expires_in_seconds: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `RotateApiTokenRequest` payload.
pub struct RotateApiTokenRequest {
    /// Name value.
    pub name: Option<String>,
    /// Expires in seconds value.
    pub expires_in_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `ApiTokenSummary` payload.
pub struct ApiTokenSummary {
    /// Id value.
    pub id: String,
    /// Name value.
    pub name: String,
    /// Username value.
    pub username: String,
    /// Scopes value.
    pub scopes: Vec<String>,
    /// Scope bindings value.
    pub scope_bindings: Vec<AccessScopeBinding>,
    /// Expires at value.
    pub expires_at: String,
    /// Created at value.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `CreatedApiToken` payload.
pub struct CreatedApiToken {
    /// Token value.
    pub token: ApiTokenSummary,
    /// Access token value.
    pub access_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
/// `AccessScopeBinding` payload.
pub struct AccessScopeBinding {
    /// Namespace value.
    pub namespace: Option<String>,
    /// App value.
    pub app: Option<String>,
    /// Worker pool value.
    pub worker_pool: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `OidcAuthorizeResponse` payload.
pub struct OidcAuthorizeResponse {
    /// Provider value.
    pub provider: String,
    /// Authorization url value.
    pub authorization_url: String,
    /// Client id value.
    pub client_id: String,
    /// Scopes value.
    pub scopes: Vec<String>,
    /// State required value.
    pub state_required: bool,
    /// Pkce required value.
    pub pkce_required: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `AuthStatusResponse` payload.
pub struct AuthStatusResponse {
    /// Mode value.
    pub mode: String,
    /// Local login enabled value.
    pub local_login_enabled: bool,
    /// Bootstrap required value.
    pub bootstrap_required: bool,
    /// Registration open value.
    pub registration_open: bool,
    /// Oidc value.
    pub oidc: OidcStatus,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `OidcStatus` payload.
pub struct OidcStatus {
    /// Boolean state flag.
    pub enabled: bool,
    /// Issuer url value.
    pub issuer_url: Option<String>,
    /// Client id value.
    pub client_id: Option<String>,
    /// Client secret configured value.
    pub client_secret_configured: bool,
    /// Scopes value.
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `ObservabilityStatusResponse` payload.
pub struct ObservabilityStatusResponse {
    /// Tracing value.
    pub tracing: TracingStatus,
    /// Ready value.
    pub ready: bool,
    /// Issues value.
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `TracingStatus` payload.
pub struct TracingStatus {
    /// Boolean state flag.
    pub enabled: bool,
    /// Exporter value.
    pub exporter: String,
    /// Endpoint configured value.
    pub endpoint_configured: bool,
    /// Header names value.
    pub header_names: Vec<String>,
}

/// `SecurityPostureApiResponse` type alias.
pub type SecurityPostureApiResponse = ApiResponse<SecurityPostureResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `SecurityPostureResponse` payload.
pub struct SecurityPostureResponse {
    /// Overall status value.
    pub overall_status: String,
    /// Checks value.
    pub checks: Vec<SecurityPostureCheck>,
    /// Transport value.
    pub transport: TransportSecurityStatusResponse,
    /// Script governance value.
    pub script_governance: ScriptGovernancePosture,
    /// Notification safety value.
    pub notification_safety: NotificationSafetyPosture,
    /// Cluster transport value.
    pub cluster_transport: ClusterTransportPosture,
    /// Recent denials value.
    pub recent_denials: Vec<SecurityPolicyDenial>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `SecurityPostureCheck` payload.
pub struct SecurityPostureCheck {
    /// Id value.
    pub id: String,
    /// Label value.
    pub label: String,
    /// Status value.
    pub status: String,
    /// Source value.
    pub source: String,
    /// Detail value.
    pub detail: String,
    /// Evidence count value.
    pub evidence_count: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `ScriptGovernancePosture` payload.
pub struct ScriptGovernancePosture {
    /// Total scripts value.
    pub total_scripts: u64,
    /// Safe default deny scripts value.
    pub safe_default_deny_scripts: u64,
    /// Dangerous policy scripts value.
    pub dangerous_policy_scripts: u64,
    /// Released scripts value.
    pub released_scripts: u64,
    /// Signed releases value.
    pub signed_releases: u64,
    /// Releases with grants value.
    pub releases_with_grants: u64,
    /// Release signature required value.
    pub release_signature_required: bool,
    /// Release signature secret configured value.
    pub release_signature_secret_configured: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `NotificationSafetyPosture` payload.
pub struct NotificationSafetyPosture {
    /// Total channels value.
    pub total_channels: u64,
    /// Enabled channels value.
    pub enabled_channels: u64,
    /// Configured targets value.
    pub configured_targets: u64,
    /// Redacted targets value.
    pub redacted_targets: u64,
    /// Channels with safety policy value.
    pub channels_with_safety_policy: u64,
    /// Direct secret values redacted value.
    pub direct_secret_values_redacted: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `ClusterTransportPosture` payload.
pub struct ClusterTransportPosture {
    /// Raft transport token configured value.
    pub raft_transport_token_configured: bool,
    /// Worker tunnel tls ready value.
    pub worker_tunnel_tls_ready: bool,
    /// Http tls ready value.
    pub http_tls_ready: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `SecurityPolicyDenial` payload.
pub struct SecurityPolicyDenial {
    /// Id value.
    pub id: String,
    /// Resource type value.
    pub resource_type: String,
    /// Resource id value.
    pub resource_id: String,
    /// Action value.
    pub action: String,
    /// Failure reason value.
    pub failure_reason: String,
    /// Detail value.
    pub detail: Option<String>,
    /// Created at value.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `TransportSecurityStatusResponse` payload.
pub struct TransportSecurityStatusResponse {
    /// Http value.
    pub http: TlsEndpointStatus,
    /// Worker tunnel value.
    pub worker_tunnel: TlsEndpointStatus,
    /// Ready value.
    pub ready: bool,
    /// Issues value.
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `TlsEndpointStatus` payload.
pub struct TlsEndpointStatus {
    /// Tls enabled value.
    pub tls_enabled: bool,
    /// Mtls required value.
    pub mtls_required: bool,
    /// Certificate and key material configuration status.
    #[serde(flatten)]
    pub material: TlsMaterialStatus,
    /// Listener mode value.
    pub listener_mode: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `TlsMaterialStatus` payload.
pub struct TlsMaterialStatus {
    /// Cert configured value.
    pub cert_configured: bool,
    /// Key configured value.
    pub key_configured: bool,
    /// Ca configured value.
    pub ca_configured: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `MeResponse` payload.
pub struct MeResponse {
    /// Username value.
    pub username: String,
    /// Roles value.
    pub roles: Vec<String>,
    /// Permissions value.
    pub permissions: Vec<tikeo_storage::PermissionSummary>,
    /// Bootstrap admin value.
    pub bootstrap_admin: bool,
    /// Scope limited value.
    pub scope_limited: bool,
    /// Token scopes value.
    pub token_scopes: Vec<String>,
    /// Scope bindings value.
    pub scope_bindings: Vec<AccessScopeBinding>,
    /// Menu keys value.
    pub menu_keys: Vec<String>,
    /// Ui action keys value.
    pub ui_action_keys: Vec<String>,
}

/// `WorkflowApiResponse` type alias.
pub type WorkflowApiResponse = ApiResponse<tikeo_storage::WorkflowSummary>;
/// `WorkflowListApiResponse` type alias.
pub type WorkflowListApiResponse = ApiResponse<Vec<tikeo_storage::WorkflowSummary>>;
/// `WorkflowValidationApiResponse` type alias.
pub type WorkflowValidationApiResponse = ApiResponse<tikeo_storage::WorkflowValidationResult>;
/// `WorkflowInstanceApiResponse` type alias.
pub type WorkflowInstanceApiResponse = ApiResponse<tikeo_storage::WorkflowInstanceSummary>;
/// `WorkflowAdvanceApiResponse` type alias.
pub type WorkflowAdvanceApiResponse = ApiResponse<tikeo_storage::AdvanceWorkflowResult>;
/// `WorkflowMaterializeApiResponse` type alias.
pub type WorkflowMaterializeApiResponse = ApiResponse<tikeo_storage::MaterializeWorkflowNodeResult>;
/// `WorkflowRecoverApiResponse` type alias.
pub type WorkflowRecoverApiResponse = ApiResponse<tikeo_storage::RecoverWorkflowNodeResult>;
/// `WorkflowShardRebalanceApiResponse` type alias.
pub type WorkflowShardRebalanceApiResponse =
    ApiResponse<tikeo_storage::RebalanceWorkflowShardsResult>;
/// `WorkflowShardListApiResponse` type alias.
pub type WorkflowShardListApiResponse = ApiResponse<Vec<tikeo_storage::WorkflowShardSummary>>;
/// `WorkflowShardCompleteApiResponse` type alias.
pub type WorkflowShardCompleteApiResponse = ApiResponse<tikeo_storage::CompleteWorkflowShardResult>;
/// `DispatchQueueApiResponse` type alias.
pub type DispatchQueueApiResponse = ApiResponse<tikeo_storage::QueueOverview>;
/// `DispatchQueueClaimApiResponse` type alias.
pub type DispatchQueueClaimApiResponse = ApiResponse<tikeo_storage::DispatchQueueClaim>;
/// `WorkerListApiResponse` type alias.
pub type WorkerListApiResponse = ApiResponse<WorkerListResponse>;
/// `WorkerLifecycleHistoryApiResponse` type alias.
pub type WorkerLifecycleHistoryApiResponse = ApiResponse<WorkerLifecycleHistoryResponse>;
/// `RaftAppendEntriesApiResponse` type alias.
pub type RaftAppendEntriesApiResponse = ApiResponse<RaftMessageResult>;
/// `RaftMembershipProposalApiResponse` type alias.
pub type RaftMembershipProposalApiResponse = ApiResponse<RaftMembershipProposalResponse>;
/// `WorkflowDryRunApiResponse` type alias.
pub type WorkflowDryRunApiResponse = ApiResponse<WorkflowDryRunResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `WorkflowDryRunResponse` payload.
pub struct WorkflowDryRunResponse {
    /// Validation value.
    pub validation: tikeo_storage::WorkflowValidationResult,
    /// Start nodes value.
    pub start_nodes: Vec<String>,
    /// Node count value.
    pub node_count: usize,
    /// Edge count value.
    pub edge_count: usize,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `WorkerSummary` payload.
pub struct WorkerSummary {
    /// Worker id value.
    pub worker_id: String,
    /// Logical instance id value.
    pub logical_instance_id: String,
    /// Client instance id value.
    pub client_instance_id: Option<String>,
    /// App value.
    pub app: String,
    /// Namespace value.
    pub namespace: String,
    /// Cluster value.
    pub cluster: String,
    /// Region value.
    pub region: String,
    /// Capabilities value.
    pub capabilities: Vec<String>,
    /// Structured capabilities value.
    pub structured_capabilities: WorkerCapabilitiesSummary,
    /// Master value.
    pub master: WorkerMasterSummary,
    /// Generation value.
    pub generation: u64,
    /// Status value.
    pub status: String,
    /// Status reason value.
    pub status_reason: Option<String>,
    /// Replaced by worker id value.
    pub replaced_by_worker_id: Option<String>,
    /// Last sequence value.
    pub last_sequence: u64,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
/// `WorkerMasterSummary` payload.
pub struct WorkerMasterSummary {
    /// Domain value.
    pub domain: String,
    /// Is master value.
    pub is_master: bool,
    /// Master worker id value.
    pub master_worker_id: Option<String>,
    /// Term value.
    pub term: u64,
    /// Fencing token value.
    pub fencing_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
/// `WorkerCapabilitiesSummary` payload.
pub struct WorkerCapabilitiesSummary {
    /// Tags value.
    pub tags: Vec<String>,
    /// Normal processors value.
    pub normal_processors: Vec<WorkerProcessorSummary>,
    /// Script runners value.
    pub script_runners: Vec<WorkerScriptRunnerSummary>,
    /// Plugin processors value.
    pub plugin_processors: Vec<WorkerPluginProcessorSummary>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `WorkerProcessorSummary` payload.
pub struct WorkerProcessorSummary {
    /// Name value.
    pub name: String,
    /// Description value.
    pub description: String,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `WorkerScriptRunnerSummary` payload.
pub struct WorkerScriptRunnerSummary {
    /// Language value.
    pub language: String,
    /// Sandbox backend value.
    pub sandbox_backend: String,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `WorkerPluginProcessorSummary` payload.
pub struct WorkerPluginProcessorSummary {
    /// Record type discriminator.
    pub r#type: String,
    /// Processor names value.
    pub processor_names: Vec<String>,
    /// Processors value.
    pub processors: Vec<WorkerProcessorSummary>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `WorkerListResponse` payload.
pub struct WorkerListResponse {
    /// Online value.
    pub online: usize,
    /// Items value.
    pub items: Vec<WorkerSummary>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `WorkerSessionHistorySummary` payload.
pub struct WorkerSessionHistorySummary {
    /// Worker id value.
    pub worker_id: String,
    /// Logical instance id value.
    pub logical_instance_id: String,
    /// Generation value.
    pub generation: i64,
    /// Status value.
    pub status: String,
    /// Status reason value.
    pub status_reason: Option<String>,
    /// Status evidence value.
    pub status_evidence: Option<String>,
    /// Lease expires at value.
    pub lease_expires_at: String,
    /// Last heartbeat at value.
    pub last_heartbeat_at: String,
    /// Last sequence value.
    pub last_sequence: i64,
    /// Replaced by worker id value.
    pub replaced_by_worker_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `WorkerSessionEventDto` payload.
pub struct WorkerSessionEventDto {
    /// Id value.
    pub id: String,
    /// Worker id value.
    pub worker_id: String,
    /// Logical instance id value.
    pub logical_instance_id: String,
    /// Event type value.
    pub event_type: String,
    /// Reason value.
    pub reason: Option<String>,
    /// Serialized data value.
    pub detail_json: Option<String>,
    /// Created at value.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `WorkerLifecycleHistoryResponse` payload.
pub struct WorkerLifecycleHistoryResponse {
    /// Sessions value.
    pub sessions: Vec<WorkerSessionHistorySummary>,
    /// Events value.
    pub events: Vec<WorkerSessionEventDto>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `RaftAppendEntriesRequest` payload.
pub struct RaftAppendEntriesRequest {
    /// From value.
    pub from: u64,
    /// To value.
    pub to: u64,
    /// Term value.
    pub term: i64,
    /// Message type value.
    pub message_type: String,
    /// Index value.
    pub index: i64,
    /// Log term value.
    pub log_term: i64,
    /// Commit value.
    pub commit: i64,
    /// Snapshot index value.
    pub snapshot_index: Option<i64>,
    /// Snapshot term value.
    pub snapshot_term: Option<i64>,
    /// Entries value.
    pub entries: Vec<RaftWireEntry>,
    /// Context value.
    pub context: Option<String>,
    /// Reject value.
    pub reject: Option<bool>,
    /// Reject hint value.
    pub reject_hint: Option<i64>,
    /// Leader fencing token value.
    pub leader_fencing_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `RaftWireEntry` payload.
pub struct RaftWireEntry {
    /// Entry type value.
    pub entry_type: String,
    /// Index value.
    pub index: i64,
    /// Term value.
    pub term: i64,
    /// Data value.
    pub data: String,
    /// Context value.
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `RaftMessageResult` payload.
pub struct RaftMessageResult {
    /// Accepted value.
    pub accepted: bool,
    /// Reason value.
    pub reason: String,
    /// Local node id value.
    pub local_node_id: String,
    /// Local role value.
    pub local_role: String,
    /// Leader fencing token value.
    pub leader_fencing_token: Option<String>,
    /// Remote addr value.
    pub remote_addr: Option<String>,
    /// Received term value.
    pub received_term: i64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `RaftMembershipProposalRequest` payload.
pub struct RaftMembershipProposalRequest {
    /// Proposal id value.
    pub proposal_id: String,
    /// Action value.
    pub action: String,
    /// Node id value.
    pub node_id: String,
    /// Endpoint value.
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `RaftMembershipProposalResponse` payload.
pub struct RaftMembershipProposalResponse {
    /// Accepted value.
    pub accepted: bool,
    /// Reason value.
    pub reason: String,
    /// Local node id value.
    pub local_node_id: String,
    /// Local role value.
    pub local_role: String,
    /// Leader fencing token value.
    pub leader_fencing_token: Option<String>,
    /// Proposal value.
    pub proposal: Option<tikeo_storage::RaftMembershipProposalSummary>,
}

mod job_models;
pub use job_models::*;
