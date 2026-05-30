#![allow(missing_docs)]

//! HTTP DTOs used by the management API.

#![allow(clippy::option_if_let_else)]

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

pub const SUCCESS_CODE: i32 = 0;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn success(data: T) -> Self {
        Self {
            code: SUCCESS_CODE,
            message: "success".to_owned(),
            data: Some(data),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EmptyData {}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ErrorData {
    pub trace_id: String,
    pub details: Option<serde_json::Value>,
}

pub type ErrorResponse = ApiResponse<ErrorData>;

pub type LoginApiResponse = ApiResponse<AuthSession>;

pub type AuthStatusApiResponse = ApiResponse<AuthStatusResponse>;
pub type OidcAuthorizeApiResponse = ApiResponse<OidcAuthorizeResponse>;

pub type MeApiResponse = ApiResponse<MeResponse>;

pub type EmptyApiResponse = ApiResponse<EmptyData>;

pub type SystemInfoApiResponse = ApiResponse<SystemInfoResponse>;

pub type ClusterApiResponse = ApiResponse<ClusterResponse>;
pub type ClusterDiagnosticsApiResponse = ApiResponse<ClusterDiagnosticsResponse>;
pub type TransportSecurityStatusApiResponse = ApiResponse<TransportSecurityStatusResponse>;
pub type ObservabilityStatusApiResponse = ApiResponse<ObservabilityStatusResponse>;

pub type JobPageApiResponse = ApiResponse<Page>;

pub type JobApiResponse = ApiResponse<JobSummary>;

pub type DeleteJobApiResponse = ApiResponse<EmptyData>;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub password: Option<String>,
    pub role: Option<String>,
}

pub type UserApiResponse = ApiResponse<tikee_storage::UserSummary>;
pub type UserListApiResponse = ApiResponse<Vec<tikee_storage::UserSummary>>;

pub type JobInstancePageApiResponse = ApiResponse<JobInstancePage>;

pub type JobInstanceApiResponse = ApiResponse<JobInstanceSummary>;
pub type JobInstanceCancelApiResponse = ApiResponse<JobInstanceSummary>;

pub type JobInstanceLogPageApiResponse = ApiResponse<JobInstanceLogPage>;

pub type JobInstanceAttemptPageApiResponse = ApiResponse<JobInstanceAttemptPage>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub items: Vec<JobSummary>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PageQuery {
    pub page_size: Option<u32>,
    pub page_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateAlertRuleRequest {
    pub name: String,
    pub severity: String,
    pub condition: serde_json::Value,
    pub channels: Vec<serde_json::Value>,
    pub enabled: bool,
    pub dedupe_seconds: Option<u64>,
    pub silenced_until: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlertRuleSummary {
    pub id: String,
    pub name: String,
    pub severity: String,
    pub condition: serde_json::Value,
    pub channels: Vec<serde_json::Value>,
    pub enabled: bool,
    pub dedupe_seconds: u64,
    pub silenced_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlertEventSummary {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: String,
    pub status: String,
    pub event_type: String,
    pub resource_type: String,
    pub resource_id: String,
    pub failure_class: Option<String>,
    pub message: Option<String>,
    pub dedupe_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlertDeliveryStatusResponse {
    pub rule_id: String,
    pub ready: bool,
    pub channel_count: u64,
    pub channels: Vec<AlertDeliveryChannelStatus>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlertDeliveryQueueStatusResponse {
    pub total_attempts: u64,
    pub delivered: u64,
    pub retry_pending: u64,
    pub dead_letter: u64,
    pub retry_consumed: u64,
    pub failed: u64,
    pub recent_dead_letters: Vec<tikee_storage::AlertDeliveryAttemptSummary>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlertDeliveryChannelStatus {
    pub provider: String,
    pub target_configured: bool,
    pub secret_configured: bool,
    pub enabled: bool,
    pub target_redacted: Option<String>,
    pub transport_security: Option<String>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlertNotificationSummary {
    pub rule_id: String,
    pub rule_name: String,
    pub severity: String,
    pub resource_type: String,
    pub resource_id: String,
    pub failure_class: Option<String>,
    pub latest_status: String,
    pub latest_event_type: String,
    pub latest_message: Option<String>,
    pub event_count: u64,
    pub firing_count: u64,
    pub suppressed_count: u64,
    pub silenced_count: u64,
    pub recovered_count: u64,
    pub first_seen: String,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MetricsSummaryResponse {
    pub workers: MetricsWorkerSummary,
    pub instances: MetricsInstanceSummary,
    pub alerts: MetricsAlertSummary,
    pub governance: MetricsGovernanceSummary,
    pub queue: tikee_storage::DispatchQueueSloSummary,
    pub workflows: tikee_storage::WorkflowSloSummary,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MetricsWorkerSummary {
    pub online: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MetricsInstanceSummary {
    pub total: u64,
    pub by_status: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MetricsAlertSummary {
    pub total_events: u64,
    pub by_status: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MetricsGovernanceSummary {
    pub script_failure_events: u64,
    pub by_failure_class: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AuditLogQuery {
    pub page_size: Option<u32>,
    pub page_token: Option<String>,
    pub actor: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub failure_reason: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SystemInfoResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub target: &'static str,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ClusterResponse {
    pub mode: String,
    pub role: String,
    pub node_id: String,
    pub nodes: u32,
    pub can_schedule: bool,
    pub leader_fencing_token: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ClusterDiagnosticsResponse {
    pub status: ClusterResponse,
    pub scheduling_gated: bool,
    pub metadata: Option<RaftMetadataDiagnostic>,
    pub members: Vec<RaftMemberDiagnostic>,
    pub transport: RaftTransportDiagnostic,
    pub runtime_boundary: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RaftMetadataDiagnostic {
    pub cluster_id: String,
    pub node_id: String,
    pub current_term: i64,
    pub voted_for: Option<String>,
    pub commit_index: i64,
    pub applied_index: i64,
    pub leader_fencing_token: Option<String>,
    pub conf_state: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RaftMemberDiagnostic {
    pub node_id: String,
    pub endpoint: String,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RaftTransportDiagnostic {
    pub append_entries_path: &'static str,
    pub mutating: bool,
    pub status: &'static str,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AuthSession {
    pub token: String,
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<tikee_storage::PermissionSummary>,
    pub scope_limited: bool,
    pub token_scopes: Vec<String>,
    pub scope_bindings: Vec<AccessScopeBinding>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateApiTokenRequest {
    pub name: String,
    pub scopes: Option<Vec<String>>,
    pub scope_bindings: Option<Vec<AccessScopeBinding>>,
    pub expires_in_seconds: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RotateApiTokenRequest {
    pub name: Option<String>,
    pub expires_in_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiTokenSummary {
    pub id: String,
    pub name: String,
    pub username: String,
    pub scopes: Vec<String>,
    pub scope_bindings: Vec<AccessScopeBinding>,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreatedApiToken {
    pub token: ApiTokenSummary,
    pub access_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AccessScopeBinding {
    pub namespace: Option<String>,
    pub app: Option<String>,
    pub worker_pool: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OidcAuthorizeResponse {
    pub provider: String,
    pub authorization_url: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub state_required: bool,
    pub pkce_required: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AuthStatusResponse {
    pub mode: String,
    pub local_login_enabled: bool,
    pub oidc: OidcStatus,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OidcStatus {
    pub enabled: bool,
    pub issuer_url: Option<String>,
    pub client_id: Option<String>,
    pub client_secret_configured: bool,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ObservabilityStatusResponse {
    pub tracing: TracingStatus,
    pub ready: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TracingStatus {
    pub enabled: bool,
    pub exporter: String,
    pub endpoint_configured: bool,
    pub header_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TransportSecurityStatusResponse {
    pub http: TlsEndpointStatus,
    pub worker_tunnel: TlsEndpointStatus,
    pub ready: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct TlsEndpointStatus {
    pub tls_enabled: bool,
    pub mtls_required: bool,
    pub cert_configured: bool,
    pub key_configured: bool,
    pub ca_configured: bool,
    pub listener_mode: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MeResponse {
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<tikee_storage::PermissionSummary>,
    pub scope_limited: bool,
    pub token_scopes: Vec<String>,
    pub scope_bindings: Vec<AccessScopeBinding>,
}

pub type WorkflowApiResponse = ApiResponse<tikee_storage::WorkflowSummary>;
pub type WorkflowListApiResponse = ApiResponse<Vec<tikee_storage::WorkflowSummary>>;
pub type WorkflowValidationApiResponse = ApiResponse<tikee_storage::WorkflowValidationResult>;
pub type WorkflowInstanceApiResponse = ApiResponse<tikee_storage::WorkflowInstanceSummary>;
pub type WorkflowAdvanceApiResponse = ApiResponse<tikee_storage::AdvanceWorkflowResult>;
pub type WorkflowMaterializeApiResponse = ApiResponse<tikee_storage::MaterializeWorkflowNodeResult>;
pub type WorkflowRecoverApiResponse = ApiResponse<tikee_storage::RecoverWorkflowNodeResult>;
pub type WorkflowShardRebalanceApiResponse =
    ApiResponse<tikee_storage::RebalanceWorkflowShardsResult>;
pub type WorkflowShardListApiResponse = ApiResponse<Vec<tikee_storage::WorkflowShardSummary>>;
pub type WorkflowShardCompleteApiResponse = ApiResponse<tikee_storage::CompleteWorkflowShardResult>;
pub type DispatchQueueApiResponse = ApiResponse<tikee_storage::QueueOverview>;
pub type DispatchQueueClaimApiResponse = ApiResponse<tikee_storage::DispatchQueueClaim>;
pub type WorkerListApiResponse = ApiResponse<WorkerListResponse>;
pub type WorkerLifecycleHistoryApiResponse = ApiResponse<WorkerLifecycleHistoryResponse>;
pub type RaftAppendEntriesApiResponse = ApiResponse<RaftMessageResult>;
pub type RaftMembershipProposalApiResponse = ApiResponse<RaftMembershipProposalResponse>;
pub type WorkflowDryRunApiResponse = ApiResponse<WorkflowDryRunResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDryRunResponse {
    pub validation: tikee_storage::WorkflowValidationResult,
    pub start_nodes: Vec<String>,
    pub node_count: usize,
    pub edge_count: usize,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerSummary {
    pub worker_id: String,
    pub logical_instance_id: String,
    pub client_instance_id: Option<String>,
    pub app: String,
    pub namespace: String,
    pub cluster: String,
    pub region: String,
    pub capabilities: Vec<String>,
    pub structured_capabilities: WorkerCapabilitiesSummary,
    pub master: WorkerMasterSummary,
    pub generation: u64,
    pub status: String,
    pub status_reason: Option<String>,
    pub replaced_by_worker_id: Option<String>,
    pub last_sequence: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkerMasterSummary {
    pub domain: String,
    pub is_master: bool,
    pub master_worker_id: Option<String>,
    pub term: u64,
    pub fencing_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkerCapabilitiesSummary {
    pub tags: Vec<String>,
    pub sdk_processors: Vec<String>,
    pub script_runners: Vec<WorkerScriptRunnerSummary>,
    pub plugin_processors: Vec<WorkerPluginProcessorSummary>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerScriptRunnerSummary {
    pub language: String,
    pub sandbox_backend: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerPluginProcessorSummary {
    pub r#type: String,
    pub processor_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerListResponse {
    pub online: usize,
    pub items: Vec<WorkerSummary>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerSessionHistorySummary {
    pub worker_id: String,
    pub logical_instance_id: String,
    pub generation: i64,
    pub status: String,
    pub status_reason: Option<String>,
    pub status_evidence: Option<String>,
    pub lease_expires_at: String,
    pub last_heartbeat_at: String,
    pub last_sequence: i64,
    pub replaced_by_worker_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerSessionEventDto {
    pub id: String,
    pub worker_id: String,
    pub logical_instance_id: String,
    pub event_type: String,
    pub reason: Option<String>,
    pub detail_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerLifecycleHistoryResponse {
    pub sessions: Vec<WorkerSessionHistorySummary>,
    pub events: Vec<WorkerSessionEventDto>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RaftAppendEntriesRequest {
    pub from: u64,
    pub to: u64,
    pub term: i64,
    pub message_type: String,
    pub index: i64,
    pub log_term: i64,
    pub commit: i64,
    pub snapshot_index: Option<i64>,
    pub snapshot_term: Option<i64>,
    pub entries: Vec<RaftWireEntry>,
    pub context: Option<String>,
    pub reject: Option<bool>,
    pub reject_hint: Option<i64>,
    pub leader_fencing_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RaftWireEntry {
    pub entry_type: String,
    pub index: i64,
    pub term: i64,
    pub data: String,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RaftMessageResult {
    pub accepted: bool,
    pub reason: String,
    pub local_node_id: String,
    pub local_role: String,
    pub leader_fencing_token: Option<String>,
    pub remote_addr: Option<String>,
    pub received_term: i64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RaftMembershipProposalRequest {
    pub proposal_id: String,
    pub action: String,
    pub node_id: String,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RaftMembershipProposalResponse {
    pub accepted: bool,
    pub reason: String,
    pub local_node_id: String,
    pub local_role: String,
    pub leader_fencing_token: Option<String>,
    pub proposal: Option<tikee_storage::RaftMembershipProposalSummary>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct WorkflowRunRequest {
    pub trigger_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobSummary {
    pub version_number: i64,
    pub id: String,
    pub namespace: String,
    pub app: String,
    pub name: String,
    pub schedule_type: String,
    pub schedule_expr: Option<String>,
    pub misfire_policy: String,
    pub schedule_start_at: Option<String>,
    pub schedule_end_at: Option<String>,
    pub schedule_calendar: Option<serde_json::Value>,
    pub processor_name: Option<String>,
    pub processor_type: Option<String>,
    pub script_id: Option<String>,
    pub enabled: bool,
    pub canary_job_id: Option<String>,
    pub canary_percent: i32,
}

pub type JobSchedulingAdviceApiResponse = ApiResponse<JobSchedulingAdviceResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobSchedulingAdviceResponse {
    pub ready: bool,
    pub severity: String,
    pub reason: String,
    pub required_capability: Option<String>,
    pub eligible_workers: Vec<String>,
    pub recent_instances: u64,
    pub recent_failures: u64,
    pub history: JobSchedulingHistorySummary,
    pub prediction: JobSchedulingPrediction,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobSchedulingHistorySummary {
    pub inspected_instances: u64,
    pub completed_instances: u64,
    pub failed_instances: u64,
    pub average_duration_seconds: u64,
    pub p50_duration_seconds: u64,
    pub p95_duration_seconds: u64,
    pub max_duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobSchedulingPrediction {
    pub estimated_duration_seconds: u64,
    pub recommended_concurrency: u64,
    pub worker_capacity: JobSchedulingWorkerCapacity,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobSchedulingWorkerCapacity {
    pub eligible_worker_count: u64,
    pub advertised_cpu_cores: u64,
    pub advertised_memory_mb: u64,
}

pub type JobTopologyApiResponse = ApiResponse<JobTopologyResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobTopologyResponse {
    pub nodes: Vec<JobTopologyNode>,
    pub edges: Vec<JobTopologyEdge>,
    pub unresolved: Vec<JobTopologyUnresolvedRef>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobTopologyNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    pub namespace: Option<String>,
    pub app: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobTopologyEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub edge_type: String,
    pub label: Option<String>,
    pub workflow_id: Option<String>,
    pub workflow_name: Option<String>,
    pub condition: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobTopologyUnresolvedRef {
    pub workflow_id: String,
    pub workflow_name: String,
    pub node_key: String,
    pub missing_job_id: String,
    pub reason: String,
}

pub type JobImpactApiResponse = ApiResponse<JobImpactResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobImpactResponse {
    pub target_job: JobImpactJobRef,
    pub referencing_workflows: Vec<JobImpactWorkflowRef>,
    pub upstream_jobs: Vec<JobImpactJobRef>,
    pub downstream_jobs: Vec<JobImpactJobRef>,
    pub risk_summary: JobImpactRiskSummary,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobImpactJobRef {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub app: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobImpactWorkflowRef {
    pub id: String,
    pub name: String,
    pub node_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobImpactRiskSummary {
    pub workflow_count: u64,
    pub upstream_count: u64,
    pub downstream_count: u64,
    pub unresolved_count: u64,
    pub risk_level: String,
    pub reasons: Vec<String>,
}

pub type WorkflowReplayApiResponse = ApiResponse<WorkflowReplayResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowReplayResponse {
    pub instance: tikee_storage::WorkflowInstanceSummary,
    pub workflow: tikee_storage::WorkflowSummary,
    pub events: Vec<tikee_storage::InstanceEventSummary>,
    pub graph: JobTopologyResponse,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobRequest {
    pub namespace: Option<String>,
    pub app: Option<String>,
    pub name: String,
    pub schedule_type: Option<String>,
    pub schedule_expr: Option<String>,
    pub misfire_policy: Option<String>,
    pub schedule_start_at: Option<String>,
    pub schedule_end_at: Option<String>,
    pub schedule_calendar: Option<serde_json::Value>,
    pub processor_name: Option<String>,
    pub processor_type: Option<String>,
    pub script_id: Option<String>,
    pub enabled: Option<bool>,
    pub canary_job_id: Option<String>,
    pub canary_percent: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJobRequest {
    pub name: Option<String>,
    pub schedule_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub schedule_expr: Option<Option<String>>,
    pub misfire_policy: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub schedule_start_at: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub schedule_end_at: Option<Option<String>>,
    #[serde(default)]
    pub schedule_calendar: Option<Option<serde_json::Value>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub processor_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub processor_type: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub script_id: Option<Option<String>>,
    pub enabled: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub canary_job_id: Option<Option<String>>,
    pub canary_percent: Option<i32>,
}

fn deserialize_nullable_update<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

pub type JobVersionPageApiResponse = ApiResponse<JobVersionPage>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobVersionPage {
    pub items: Vec<tikee_storage::JobVersionSummary>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RollbackJobRequest {
    pub version_number: i64,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastSelectorRequest {
    pub tags: Option<Vec<String>>,
    pub region: Option<String>,
    pub cluster: Option<String>,
    pub labels: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TriggerJobRequest {
    pub trigger_type: Option<String>,
    pub execution_mode: Option<String>,
    pub broadcast_selector: Option<BroadcastSelectorRequest>,
}

pub type InboundWebhookTriggerApiResponse = ApiResponse<InboundWebhookTriggerResponse>;

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InboundWebhookTriggerRequest {
    pub source: Option<String>,
    pub event_type: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub signature: Option<String>,
    pub timestamp: Option<i64>,
    pub nonce: Option<String>,
    pub secret_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InboundWebhookTriggerResponse {
    pub accepted: bool,
    pub instance_id: String,
    pub job_id: String,
    pub status: String,
    pub trigger_type: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CanaryRoutingSummary {
    pub enabled: bool,
    pub routed: bool,
    pub original_job_id: String,
    pub routed_job_id: String,
    pub percent: i32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobInstancePage {
    pub items: Vec<JobInstanceSummary>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobInstanceSummary {
    pub id: String,
    pub job_id: String,
    pub status: String,
    pub trigger_type: String,
    pub execution_mode: String,
    pub created_at: String,
    pub updated_at: String,
    pub log_count: u64,
    pub latest_log: Option<JobInstanceLogSummary>,
    pub worker_id: Option<String>,
    pub canary_routing: Option<CanaryRoutingSummary>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobInstanceAttemptPage {
    pub items: Vec<JobInstanceAttemptSummary>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobInstanceAttemptSummary {
    pub id: String,
    pub instance_id: String,
    pub worker_id: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobInstanceLogPage {
    pub items: Vec<JobInstanceLogSummary>,
    pub next_page_token: Option<String>,
}

pub type ScriptPageApiResponse = ApiResponse<ScriptPage>;

pub type ScriptApiResponse = ApiResponse<tikee_storage::ScriptSummary>;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScriptPage {
    pub items: Vec<tikee_storage::ScriptSummary>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateScriptRequest {
    pub name: String,
    pub language: String,
    pub version: String,
    pub content: String,
    pub timeout_seconds: Option<i64>,
    pub max_memory_bytes: Option<i64>,
    pub allow_network: Option<bool>,
    pub allowed_env_vars: Option<Vec<String>>,
    pub policy: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateScriptRequest {
    pub name: Option<String>,
    pub language: Option<String>,
    pub version: Option<String>,
    pub content: Option<String>,
    pub status: Option<String>,
    pub timeout_seconds: Option<i64>,
    pub max_memory_bytes: Option<i64>,
    pub allow_network: Option<bool>,
    pub allowed_env_vars: Option<Vec<String>>,
    pub policy: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScriptReleaseRequest {
    pub version_number: Option<i64>,
    pub approval_ticket: Option<String>,
    pub signature: Option<String>,
    pub grants: Option<ScriptReleaseGrants>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct ScriptReleaseGrants {
    #[serde(default)]
    pub url: Vec<String>,
    #[serde(default)]
    pub file_read: Vec<String>,
    #[serde(default)]
    pub file_write: Vec<String>,
    #[serde(default)]
    pub secret: Vec<String>,
}

impl From<ScriptReleaseGrants> for tikee_core::ScriptReleaseGrantSet {
    fn from(value: ScriptReleaseGrants) -> Self {
        Self {
            url: value.url,
            file_read: value.file_read,
            file_write: value.file_write,
            secret: value.secret,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ScriptReleaseGateQuery {
    pub version_number: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScriptReleaseGateResponse {
    pub script_id: String,
    pub version_number: i64,
    pub version_id: String,
    pub content_sha256: String,
    pub releasable: bool,
    pub blocking_reasons: Vec<String>,
    pub required_actions: Vec<String>,
    pub signature_verification_enabled: bool,
}

pub type ScriptReleaseGateApiResponse = ApiResponse<ScriptReleaseGateResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobInstanceLogSummary {
    pub id: String,
    pub instance_id: String,
    pub worker_id: String,
    pub level: String,
    pub message: String,
    pub governance_event: Option<String>,
    pub governance_failure_class: Option<String>,
    pub governance_message: Option<String>,
    pub sequence: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScriptVersionApiResponse {
    #[schema(value_type = Object)]
    pub data: tikee_storage::ScriptVersionSummary,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScriptVersionListApiResponse {
    #[schema(value_type = Vec<Object>)]
    pub data: Vec<tikee_storage::ScriptVersionSummary>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScriptDiffApiResponse {
    pub data: ScriptDiffResult,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScriptDiffResult {
    pub content_diff: String,
    pub policy_diff: Vec<FieldChange>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FieldChange {
    pub field: String,
    pub before: String,
    pub after: String,
}

pub type AuditLogExportApiResponse = ApiResponse<AuditLogExport>;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuditLogExport {
    pub format: String,
    pub items: Vec<AuditLogSummary>,
    pub exported: u64,
    pub max_rows: u64,
    pub redacted: bool,
    pub governance: String,
}

pub type AuditLogPageApiResponse = ApiResponse<AuditLogPage>;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuditLogPage {
    pub items: Vec<AuditLogSummary>,
    pub total: u64,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuditLogSummary {
    pub id: String,
    pub actor: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub detail: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub trace_id: Option<String>,
    pub result: String,
    pub failure_reason: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
}

impl From<tikee_storage::AuditLogSummary> for AuditLogSummary {
    fn from(value: tikee_storage::AuditLogSummary) -> Self {
        Self {
            id: value.id,
            actor: value.actor,
            action: value.action,
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            detail: value.detail,
            before: value.before,
            after: value.after,
            trace_id: value.trace_id,
            result: value.result,
            failure_reason: value.failure_reason,
            ip_address: value.ip_address,
            created_at: value.created_at,
        }
    }
}

pub type GitOpsManifestApiResponse = ApiResponse<GitOpsManifestResponse>;
pub type GitOpsDiffApiResponse = ApiResponse<GitOpsDiffResponse>;

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct GitOpsExportQuery {
    pub namespace: Option<String>,
    pub app: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitOpsManifestResponse {
    pub manifest: GitOpsManifest,
    pub format: String,
    pub manifest_yaml: Option<String>,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitOpsManifest {
    pub api_version: String,
    pub kind: String,
    pub scope: GitOpsScope,
    pub resources: Vec<GitOpsResource>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitOpsScope {
    pub namespace: Option<String>,
    pub app: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitOpsResource {
    pub kind: String,
    pub metadata: GitOpsMetadata,
    pub spec: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitOpsMetadata {
    pub id: Option<String>,
    pub name: String,
    pub namespace: Option<String>,
    pub app: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitOpsDiffRequest {
    pub manifest: GitOpsManifest,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitOpsDiffResponse {
    pub current_checksum: String,
    pub desired_checksum: String,
    pub summary: BTreeMap<String, u64>,
    pub changes: Vec<GitOpsDiffChange>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitOpsDiffChange {
    pub action: String,
    pub key: String,
    pub kind: String,
    pub name: String,
    pub before: Option<GitOpsResource>,
    pub after: Option<GitOpsResource>,
    pub diff: String,
}
