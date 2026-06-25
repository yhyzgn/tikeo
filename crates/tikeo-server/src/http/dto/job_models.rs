use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};
use tikeo_storage::JobRetryPolicy;
use utoipa::ToSchema;

use super::ApiResponse;

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `WorkflowRunRequest` payload.
pub struct WorkflowRunRequest {
    /// Trigger type value.
    pub trigger_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobSummary` payload.
pub struct JobSummary {
    /// Version number value.
    pub version_number: i64,
    /// Id value.
    pub id: String,
    /// Namespace value.
    pub namespace: String,
    /// App value.
    pub app: String,
    /// Name value.
    pub name: String,
    /// Schedule type value.
    pub schedule_type: String,
    /// Schedule expr value.
    pub schedule_expr: Option<String>,
    /// Misfire policy value.
    pub misfire_policy: String,
    /// Schedule start at value.
    pub schedule_start_at: Option<String>,
    /// Schedule end at value.
    pub schedule_end_at: Option<String>,
    /// Schedule calendar value.
    pub schedule_calendar: Option<serde_json::Value>,
    /// Processor name value.
    pub processor_name: Option<String>,
    /// Processor type value.
    pub processor_type: Option<String>,
    /// Script id value.
    pub script_id: Option<String>,
    /// Boolean state flag.
    pub enabled: bool,
    /// Canary job id value.
    pub canary_job_id: Option<String>,
    /// Canary percent value.
    pub canary_percent: i32,
    /// Canary policy value.
    pub canary_policy: tikeo_storage::JobCanaryPolicy,
    /// Retry policy value.
    pub retry_policy: JobRetryPolicy,
}

/// `JobSchedulingAdviceApiResponse` type alias.
pub type JobSchedulingAdviceApiResponse = ApiResponse<JobSchedulingAdviceResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobSchedulingAdviceResponse` payload.
pub struct JobSchedulingAdviceResponse {
    /// Ready value.
    pub ready: bool,
    /// Severity value.
    pub severity: String,
    /// Reason value.
    pub reason: String,
    /// Required capability value.
    pub required_capability: Option<String>,
    /// Eligible workers value.
    pub eligible_workers: Vec<String>,
    /// Recent instances value.
    pub recent_instances: u64,
    /// Recent failures value.
    pub recent_failures: u64,
    /// History value.
    pub history: JobSchedulingHistorySummary,
    /// Prediction value.
    pub prediction: JobSchedulingPrediction,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobSchedulingHistorySummary` payload.
pub struct JobSchedulingHistorySummary {
    /// Inspected instances value.
    pub inspected_instances: u64,
    /// Completed instances value.
    pub completed_instances: u64,
    /// Failed instances value.
    pub failed_instances: u64,
    /// Average duration seconds value.
    pub average_duration_seconds: u64,
    /// P50 duration seconds value.
    pub p50_duration_seconds: u64,
    /// P95 duration seconds value.
    pub p95_duration_seconds: u64,
    /// Max duration seconds value.
    pub max_duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobSchedulingPrediction` payload.
pub struct JobSchedulingPrediction {
    /// Estimated duration seconds value.
    pub estimated_duration_seconds: u64,
    /// Recommended concurrency value.
    pub recommended_concurrency: u64,
    /// Worker capacity value.
    pub worker_capacity: JobSchedulingWorkerCapacity,
    /// Reasons value.
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobSchedulingWorkerCapacity` payload.
pub struct JobSchedulingWorkerCapacity {
    /// Eligible worker count value.
    pub eligible_worker_count: u64,
    /// Advertised cpu cores value.
    pub advertised_cpu_cores: u64,
    /// Advertised memory mb value.
    pub advertised_memory_mb: u64,
}

/// `JobTopologyApiResponse` type alias.
pub type JobTopologyApiResponse = ApiResponse<JobTopologyResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobTopologyResponse` payload.
pub struct JobTopologyResponse {
    /// Nodes value.
    pub nodes: Vec<JobTopologyNode>,
    /// Edges value.
    pub edges: Vec<JobTopologyEdge>,
    /// Unresolved value.
    pub unresolved: Vec<JobTopologyUnresolvedRef>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobTopologyNode` payload.
pub struct JobTopologyNode {
    /// Id value.
    pub id: String,
    #[serde(rename = "type")]
    /// Node type value.
    pub node_type: String,
    /// Label value.
    pub label: String,
    /// Namespace value.
    pub namespace: Option<String>,
    /// App value.
    pub app: Option<String>,
    /// Serialized data value.
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobTopologyEdge` payload.
pub struct JobTopologyEdge {
    /// Id value.
    pub id: String,
    /// From value.
    pub from: String,
    /// To value.
    pub to: String,
    #[serde(rename = "type")]
    /// Edge type value.
    pub edge_type: String,
    /// Label value.
    pub label: Option<String>,
    /// Workflow id value.
    pub workflow_id: Option<String>,
    /// Workflow name value.
    pub workflow_name: Option<String>,
    /// Condition value.
    pub condition: Option<String>,
    /// Serialized data value.
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobTopologyUnresolvedRef` payload.
pub struct JobTopologyUnresolvedRef {
    /// Workflow id value.
    pub workflow_id: String,
    /// Workflow name value.
    pub workflow_name: String,
    /// Node key value.
    pub node_key: String,
    /// Missing job id value.
    pub missing_job_id: String,
    /// Reason value.
    pub reason: String,
}

/// `JobImpactApiResponse` type alias.
pub type JobImpactApiResponse = ApiResponse<JobImpactResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobImpactResponse` payload.
pub struct JobImpactResponse {
    /// Target job value.
    pub target_job: JobImpactJobRef,
    /// Referencing workflows value.
    pub referencing_workflows: Vec<JobImpactWorkflowRef>,
    /// Upstream jobs value.
    pub upstream_jobs: Vec<JobImpactJobRef>,
    /// Downstream jobs value.
    pub downstream_jobs: Vec<JobImpactJobRef>,
    /// Risk summary value.
    pub risk_summary: JobImpactRiskSummary,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobImpactJobRef` payload.
pub struct JobImpactJobRef {
    /// Id value.
    pub id: String,
    /// Name value.
    pub name: String,
    /// Namespace value.
    pub namespace: String,
    /// App value.
    pub app: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobImpactWorkflowRef` payload.
pub struct JobImpactWorkflowRef {
    /// Id value.
    pub id: String,
    /// Name value.
    pub name: String,
    /// Node keys value.
    pub node_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobImpactRiskSummary` payload.
pub struct JobImpactRiskSummary {
    /// Workflow count value.
    pub workflow_count: u64,
    /// Upstream count value.
    pub upstream_count: u64,
    /// Downstream count value.
    pub downstream_count: u64,
    /// Unresolved count value.
    pub unresolved_count: u64,
    /// Risk level value.
    pub risk_level: String,
    /// Reasons value.
    pub reasons: Vec<String>,
}

/// `WorkflowReplayApiResponse` type alias.
pub type WorkflowReplayApiResponse = ApiResponse<WorkflowReplayResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `WorkflowReplayResponse` payload.
pub struct WorkflowReplayResponse {
    /// Instance value.
    pub instance: tikeo_storage::WorkflowInstanceSummary,
    /// Workflow value.
    pub workflow: tikeo_storage::WorkflowSummary,
    /// Events value.
    pub events: Vec<tikeo_storage::InstanceEventSummary>,
    /// Graph value.
    pub graph: JobTopologyResponse,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `CreateJobRequest` payload.
pub struct CreateJobRequest {
    /// Namespace value.
    pub namespace: Option<String>,
    /// App value.
    pub app: Option<String>,
    /// Name value.
    pub name: String,
    /// Schedule type value.
    pub schedule_type: Option<String>,
    /// Schedule expr value.
    pub schedule_expr: Option<String>,
    /// Misfire policy value.
    pub misfire_policy: Option<String>,
    /// Schedule start at value.
    pub schedule_start_at: Option<String>,
    /// Schedule end at value.
    pub schedule_end_at: Option<String>,
    /// Schedule calendar value.
    pub schedule_calendar: Option<serde_json::Value>,
    /// Processor name value.
    pub processor_name: Option<String>,
    /// Processor type value.
    pub processor_type: Option<String>,
    /// Script id value.
    pub script_id: Option<String>,
    /// Boolean state flag.
    pub enabled: Option<bool>,
    /// Canary job id value.
    pub canary_job_id: Option<String>,
    /// Canary percent value.
    pub canary_percent: Option<i32>,
    /// Canary policy value.
    pub canary_policy: Option<tikeo_storage::JobCanaryPolicy>,
    /// Retry policy value.
    pub retry_policy: Option<JobRetryPolicy>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `UpdateJobRequest` payload.
pub struct UpdateJobRequest {
    /// Namespace value.
    pub namespace: Option<String>,
    /// App value.
    pub app: Option<String>,
    /// Name value.
    pub name: Option<String>,
    /// Schedule type value.
    pub schedule_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    /// Schedule expr value.
    pub schedule_expr: NullableUpdate<String>,
    /// Misfire policy value.
    pub misfire_policy: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    /// Schedule start at value.
    pub schedule_start_at: NullableUpdate<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    /// Schedule end at value.
    pub schedule_end_at: NullableUpdate<String>,
    #[serde(default)]
    /// Schedule calendar value.
    pub schedule_calendar: Option<Option<serde_json::Value>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    /// Processor name value.
    pub processor_name: NullableUpdate<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    /// Processor type value.
    pub processor_type: NullableUpdate<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    /// Script id value.
    pub script_id: NullableUpdate<String>,
    /// Boolean state flag.
    pub enabled: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    /// Canary job id value.
    pub canary_job_id: NullableUpdate<String>,
    /// Canary percent value.
    pub canary_percent: Option<i32>,
    /// Canary policy value.
    pub canary_policy: Option<tikeo_storage::JobCanaryPolicy>,
    /// Retry policy value.
    pub retry_policy: Option<JobRetryPolicy>,
}

/// `NullableUpdate` type alias.
pub type NullableUpdate<T> = Option<Option<T>>;

fn deserialize_nullable_update<'de, D, T>(deserializer: D) -> Result<NullableUpdate<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// `JobVersionPageApiResponse` type alias.
pub type JobVersionPageApiResponse = ApiResponse<JobVersionPage>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobVersionPage` payload.
pub struct JobVersionPage {
    /// Items value.
    pub items: Vec<tikeo_storage::JobVersionSummary>,
    /// Next page token value.
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `RollbackJobRequest` payload.
pub struct RollbackJobRequest {
    /// Version number value.
    pub version_number: i64,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `BroadcastSelectorRequest` payload.
pub struct BroadcastSelectorRequest {
    /// Tags value.
    pub tags: Option<Vec<String>>,
    /// Region value.
    pub region: Option<String>,
    /// Cluster value.
    pub cluster: Option<String>,
    /// Labels value.
    pub labels: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `TriggerJobRequest` payload.
pub struct TriggerJobRequest {
    /// Trigger type value.
    pub trigger_type: Option<String>,
    /// Execution mode value.
    pub execution_mode: Option<String>,
    /// Broadcast selector value.
    pub broadcast_selector: Option<BroadcastSelectorRequest>,
}

/// `InboundWebhookTriggerApiResponse` type alias.
pub type InboundWebhookTriggerApiResponse = ApiResponse<InboundWebhookTriggerResponse>;

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `InboundWebhookTriggerRequest` payload.
pub struct InboundWebhookTriggerRequest {
    /// Source value.
    pub source: Option<String>,
    /// Event type value.
    pub event_type: Option<String>,
    /// Serialized data value.
    pub payload: Option<serde_json::Value>,
    /// Signature value.
    pub signature: Option<String>,
    /// Timestamp value.
    pub timestamp: Option<i64>,
    /// Nonce value.
    pub nonce: Option<String>,
    /// Secret ref value.
    pub secret_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `InboundWebhookTriggerResponse` payload.
pub struct InboundWebhookTriggerResponse {
    /// Accepted value.
    pub accepted: bool,
    /// Instance id value.
    pub instance_id: String,
    /// Job id value.
    pub job_id: String,
    /// Status value.
    pub status: String,
    /// Trigger type value.
    pub trigger_type: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `CanaryRoutingSummary` payload.
pub struct CanaryRoutingSummary {
    /// Boolean state flag.
    pub enabled: bool,
    /// Routed value.
    pub routed: bool,
    /// Original job id value.
    pub original_job_id: String,
    /// Routed job id value.
    pub routed_job_id: String,
    /// Percent value.
    pub percent: i32,
    /// Boolean state flag.
    pub rolled_back: bool,
    /// Metrics gate value.
    pub metrics_gate: Option<CanaryMetricsGateSummary>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `CanaryMetricsGateSummary` payload.
pub struct CanaryMetricsGateSummary {
    /// Status value.
    pub status: String,
    /// Inspected samples value.
    pub inspected_samples: u64,
    /// Failed samples value.
    pub failed_samples: u64,
    /// Failure rate value.
    pub failure_rate: f64,
    /// Threshold value.
    pub threshold: f64,
    /// Reason value.
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobInstancePage` payload.
pub struct JobInstancePage {
    /// Items value.
    pub items: Vec<JobInstanceSummary>,
    /// Next page token value.
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobInstanceResult` payload.
pub struct JobInstanceResult {
    /// Worker id value.
    pub worker_id: String,
    /// Success value.
    pub success: bool,
    /// Message value.
    pub message: String,
    /// Completed at value.
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobInstanceSummary` payload.
pub struct JobInstanceSummary {
    /// Id value.
    pub id: String,
    /// Job id value.
    pub job_id: String,
    /// Status value.
    pub status: String,
    /// Trigger type value.
    pub trigger_type: String,
    /// Execution mode value.
    pub execution_mode: String,
    /// Created at value.
    pub created_at: String,
    /// Updated at value.
    pub updated_at: String,
    /// Log count value.
    pub log_count: u64,
    /// Latest log value.
    pub latest_log: Option<JobInstanceLogSummary>,
    /// Worker id value.
    pub worker_id: Option<String>,
    /// Result value.
    pub result: Option<JobInstanceResult>,
    /// Canary routing value.
    pub canary_routing: Option<CanaryRoutingSummary>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobInstanceAttemptPage` payload.
pub struct JobInstanceAttemptPage {
    /// Items value.
    pub items: Vec<JobInstanceAttemptSummary>,
    /// Next page token value.
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobInstanceAttemptSummary` payload.
pub struct JobInstanceAttemptSummary {
    /// Id value.
    pub id: String,
    /// Instance id value.
    pub instance_id: String,
    /// Worker id value.
    pub worker_id: String,
    /// Status value.
    pub status: String,
    /// Result value.
    pub result: Option<JobInstanceResult>,
    /// Created at value.
    pub created_at: String,
    /// Updated at value.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobInstanceLogPage` payload.
pub struct JobInstanceLogPage {
    /// Items value.
    pub items: Vec<JobInstanceLogSummary>,
    /// Next page token value.
    pub next_page_token: Option<String>,
}

/// `ScriptPageApiResponse` type alias.
pub type ScriptPageApiResponse = ApiResponse<ScriptPage>;

/// `ScriptApiResponse` type alias.
pub type ScriptApiResponse = ApiResponse<tikeo_storage::ScriptSummary>;

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `ScriptPage` payload.
pub struct ScriptPage {
    /// Items value.
    pub items: Vec<tikeo_storage::ScriptSummary>,
    /// Next page token value.
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `CreateScriptRequest` payload.
pub struct CreateScriptRequest {
    /// Name value.
    pub name: String,
    /// Language value.
    pub language: String,
    /// Version value.
    pub version: String,
    /// Content value.
    pub content: String,
    /// Timeout seconds value.
    pub timeout_seconds: Option<i64>,
    /// Max memory bytes value.
    pub max_memory_bytes: Option<i64>,
    /// Allow network value.
    pub allow_network: Option<bool>,
    /// Allowed env vars value.
    pub allowed_env_vars: Option<Vec<String>>,
    /// Policy value.
    pub policy: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `UpdateScriptRequest` payload.
pub struct UpdateScriptRequest {
    /// Name value.
    pub name: Option<String>,
    /// Language value.
    pub language: Option<String>,
    /// Version value.
    pub version: Option<String>,
    /// Content value.
    pub content: Option<String>,
    /// Status value.
    pub status: Option<String>,
    /// Timeout seconds value.
    pub timeout_seconds: Option<i64>,
    /// Max memory bytes value.
    pub max_memory_bytes: Option<i64>,
    /// Allow network value.
    pub allow_network: Option<bool>,
    /// Allowed env vars value.
    pub allowed_env_vars: Option<Vec<String>>,
    /// Policy value.
    pub policy: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
/// `ScriptReleaseRequest` payload.
pub struct ScriptReleaseRequest {
    /// Version number value.
    pub version_number: Option<i64>,
    /// Approval ticket value.
    pub approval_ticket: Option<String>,
    /// Signature value.
    pub signature: Option<String>,
    /// Grants value.
    pub grants: Option<ScriptReleaseGrants>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
/// `ScriptReleaseGrants` payload.
pub struct ScriptReleaseGrants {
    #[serde(default)]
    /// Url value.
    pub url: Vec<String>,
    #[serde(default)]
    /// File read value.
    pub file_read: Vec<String>,
    #[serde(default)]
    /// File write value.
    pub file_write: Vec<String>,
    #[serde(default)]
    /// Secret value.
    pub secret: Vec<String>,
}

impl From<ScriptReleaseGrants> for tikeo_core::ScriptReleaseGrantSet {
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
/// `ScriptReleaseGateQuery` payload.
pub struct ScriptReleaseGateQuery {
    /// Version number value.
    pub version_number: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
/// `ScriptReleaseGateResponse` payload.
pub struct ScriptReleaseGateResponse {
    /// Script id value.
    pub script_id: String,
    /// Version number value.
    pub version_number: i64,
    /// Version id value.
    pub version_id: String,
    /// Content sha256 value.
    pub content_sha256: String,
    /// Releasable value.
    pub releasable: bool,
    /// Blocking reasons value.
    pub blocking_reasons: Vec<String>,
    /// Required actions value.
    pub required_actions: Vec<String>,
    /// Signature verification enabled value.
    pub signature_verification_enabled: bool,
}

/// `ScriptReleaseGateApiResponse` type alias.
pub type ScriptReleaseGateApiResponse = ApiResponse<ScriptReleaseGateResponse>;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `JobInstanceLogSummary` payload.
pub struct JobInstanceLogSummary {
    /// Id value.
    pub id: String,
    /// Instance id value.
    pub instance_id: String,
    /// Worker id value.
    pub worker_id: String,
    /// Level value.
    pub level: String,
    /// Message value.
    pub message: String,
    /// Governance event value.
    pub governance_event: Option<String>,
    /// Governance failure class value.
    pub governance_failure_class: Option<String>,
    /// Governance message value.
    pub governance_message: Option<String>,
    /// Sequence value.
    pub sequence: i64,
    /// Created at value.
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
/// `ScriptVersionApiResponse` payload.
pub struct ScriptVersionApiResponse {
    #[schema(value_type = Object)]
    /// Data value.
    pub data: tikeo_storage::ScriptVersionSummary,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
/// `ScriptVersionListApiResponse` payload.
pub struct ScriptVersionListApiResponse {
    #[schema(value_type = Vec<Object>)]
    /// Data value.
    pub data: Vec<tikeo_storage::ScriptVersionSummary>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
/// `ScriptDiffApiResponse` payload.
pub struct ScriptDiffApiResponse {
    /// Data value.
    pub data: ScriptDiffResult,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
/// `ScriptDiffResult` payload.
pub struct ScriptDiffResult {
    /// Content diff value.
    pub content_diff: String,
    /// Policy diff value.
    pub policy_diff: Vec<FieldChange>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
/// `FieldChange` payload.
pub struct FieldChange {
    /// Field value.
    pub field: String,
    /// Before value.
    pub before: String,
    /// After value.
    pub after: String,
}

/// `AuditLogExportApiResponse` type alias.
pub type AuditLogExportApiResponse = ApiResponse<AuditLogExport>;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
/// `AuditLogExport` payload.
pub struct AuditLogExport {
    /// Format value.
    pub format: String,
    /// Items value.
    pub items: Vec<AuditLogSummary>,
    /// Exported value.
    pub exported: u64,
    /// Max rows value.
    pub max_rows: u64,
    /// Redacted value.
    pub redacted: bool,
    /// Governance value.
    pub governance: String,
}

/// `AuditLogPageApiResponse` type alias.
pub type AuditLogPageApiResponse = ApiResponse<AuditLogPage>;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
/// `AuditLogPage` payload.
pub struct AuditLogPage {
    /// Items value.
    pub items: Vec<AuditLogSummary>,
    /// Total value.
    pub total: u64,
    /// Next page token value.
    pub next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
/// `AuditLogSummary` payload.
pub struct AuditLogSummary {
    /// Id value.
    pub id: String,
    /// Actor value.
    pub actor: String,
    /// Action value.
    pub action: String,
    /// Resource type value.
    pub resource_type: String,
    /// Resource id value.
    pub resource_id: String,
    /// Detail value.
    pub detail: Option<String>,
    /// Before value.
    pub before: Option<String>,
    /// After value.
    pub after: Option<String>,
    /// Trace id value.
    pub trace_id: Option<String>,
    /// Result value.
    pub result: String,
    /// Failure reason value.
    pub failure_reason: Option<String>,
    /// Ip address value.
    pub ip_address: Option<String>,
    /// Created at value.
    pub created_at: String,
}

impl From<tikeo_storage::AuditLogSummary> for AuditLogSummary {
    fn from(value: tikeo_storage::AuditLogSummary) -> Self {
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

/// `GitOpsManifestApiResponse` type alias.
pub type GitOpsManifestApiResponse = ApiResponse<GitOpsManifestResponse>;
/// `GitOpsDiffApiResponse` type alias.
pub type GitOpsDiffApiResponse = ApiResponse<GitOpsDiffResponse>;

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
/// `GitOpsExportQuery` payload.
pub struct GitOpsExportQuery {
    /// Namespace value.
    pub namespace: Option<String>,
    /// App value.
    pub app: Option<String>,
    /// Format value.
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `GitOpsManifestResponse` payload.
pub struct GitOpsManifestResponse {
    /// Manifest value.
    pub manifest: GitOpsManifest,
    /// Format value.
    pub format: String,
    /// Manifest yaml value.
    pub manifest_yaml: Option<String>,
    /// Checksum value.
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `GitOpsManifest` payload.
pub struct GitOpsManifest {
    /// Api version value.
    pub api_version: String,
    /// Kind value.
    pub kind: String,
    /// Scope value.
    pub scope: GitOpsScope,
    /// Resources value.
    pub resources: Vec<GitOpsResource>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `GitOpsScope` payload.
pub struct GitOpsScope {
    /// Namespace value.
    pub namespace: Option<String>,
    /// App value.
    pub app: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `GitOpsResource` payload.
pub struct GitOpsResource {
    /// Kind value.
    pub kind: String,
    /// Serialized data value.
    pub metadata: GitOpsMetadata,
    /// Spec value.
    pub spec: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `GitOpsMetadata` payload.
pub struct GitOpsMetadata {
    /// Id value.
    pub id: Option<String>,
    /// Name value.
    pub name: String,
    /// Namespace value.
    pub namespace: Option<String>,
    /// App value.
    pub app: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `GitOpsDiffRequest` payload.
pub struct GitOpsDiffRequest {
    /// Manifest value.
    pub manifest: GitOpsManifest,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `GitOpsDiffResponse` payload.
pub struct GitOpsDiffResponse {
    /// Current checksum value.
    pub current_checksum: String,
    /// Desired checksum value.
    pub desired_checksum: String,
    /// Summary value.
    pub summary: BTreeMap<String, u64>,
    /// Changes value.
    pub changes: Vec<GitOpsDiffChange>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// `GitOpsDiffChange` payload.
pub struct GitOpsDiffChange {
    /// Action value.
    pub action: String,
    /// Key value.
    pub key: String,
    /// Kind value.
    pub kind: String,
    /// Name value.
    pub name: String,
    /// Before value.
    pub before: Option<GitOpsResource>,
    /// After value.
    pub after: Option<GitOpsResource>,
    /// Diff value.
    pub diff: String,
}
