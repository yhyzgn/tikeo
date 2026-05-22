#![allow(missing_docs)]

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
/// Cluster diagnostics API envelope.
pub type ClusterDiagnosticsApiResponse = ApiResponse<ClusterDiagnosticsResponse>;

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
pub type UserApiResponse = ApiResponse<tikee_storage::UserSummary>;
/// User list API response envelope.
pub type UserListApiResponse = ApiResponse<Vec<tikee_storage::UserSummary>>;

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

/// Alert rule create request.
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

/// Alert rule summary.
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

/// Alert event summary.
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

/// Alert notification history summary.
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

/// Audit log list query parameters.
#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AuditLogQuery {
    /// Maximum number of items to return.
    pub page_size: Option<u32>,
    /// Opaque page token returned by a previous list call.
    pub page_token: Option<String>,
    /// Filter by actor.
    pub actor: Option<String>,
    /// Filter by action.
    pub action: Option<String>,
    /// Filter by resource type.
    pub resource_type: Option<String>,
    /// Filter by resource id.
    pub resource_id: Option<String>,
    /// Filter by failure reason.
    pub failure_reason: Option<String>,
    /// Export format for governed export endpoint; currently only `json` is supported.
    pub format: Option<String>,
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

/// Cluster status response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ClusterResponse {
    /// Cluster operating mode.
    pub mode: String,
    /// Current node role.
    pub role: String,
    /// Stable current node identifier.
    pub node_id: String,
    /// Known server node count.
    pub nodes: u32,
    /// Whether this node may own tikee/dispatcher loops.
    pub can_schedule: bool,
    /// Optional leader fencing token; null until real consensus establishes leadership.
    pub leader_fencing_token: Option<String>,
    /// Human-readable implementation note.
    pub detail: String,
}

/// Operator diagnostics for cluster runtime readiness.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ClusterDiagnosticsResponse {
    /// Current coordinator status.
    pub status: ClusterResponse,
    /// Whether tikee/dispatcher ownership loops are currently gated off.
    pub scheduling_gated: bool,
    /// Local persisted Raft metadata when present.
    pub metadata: Option<RaftMetadataDiagnostic>,
    /// Configured Raft peers/members.
    pub members: Vec<RaftMemberDiagnostic>,
    /// Reserved transport endpoint status.
    pub transport: RaftTransportDiagnostic,
    /// Runtime boundary decision for this phase.
    pub runtime_boundary: String,
}

/// Local Raft metadata diagnostic fields.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RaftMetadataDiagnostic {
    /// Logical cluster identifier.
    pub cluster_id: String,
    /// Stable local node id.
    pub node_id: String,
    /// Last known term.
    pub current_term: i64,
    /// Vote target in current term, when any.
    pub voted_for: Option<String>,
    /// Last committed index.
    pub commit_index: i64,
    /// Last applied index.
    pub applied_index: i64,
    /// Leader fencing token, null until real consensus establishes leadership.
    pub leader_fencing_token: Option<String>,
    /// Base64-encoded raft-rs `ConfState`, null until committed membership apply.
    pub conf_state: Option<String>,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Raft member diagnostic fields.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RaftMemberDiagnostic {
    /// Stable member node id.
    pub node_id: String,
    /// Peer endpoint reachable through container/K8s networking.
    pub endpoint: String,
    /// Member lifecycle status.
    pub status: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Reserved Raft transport diagnostic state.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RaftTransportDiagnostic {
    /// Reserved `AppendEntries` endpoint path.
    pub append_entries_path: &'static str,
    /// Whether the transport can submit messages to a local consensus runtime.
    pub mutating: bool,
    /// Human-readable transport status.
    pub status: &'static str,
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
    pub permissions: Vec<tikee_storage::PermissionSummary>,
}

/// Current authenticated principal.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MeResponse {
    /// Username.
    pub username: String,
    /// Granted roles.
    pub roles: Vec<String>,
    /// Granted permissions.
    pub permissions: Vec<tikee_storage::PermissionSummary>,
}

/// Workflow definition API envelope.
pub type WorkflowApiResponse = ApiResponse<tikee_storage::WorkflowSummary>;
/// Workflow list API envelope.
pub type WorkflowListApiResponse = ApiResponse<Vec<tikee_storage::WorkflowSummary>>;
/// Workflow validation API envelope.
pub type WorkflowValidationApiResponse = ApiResponse<tikee_storage::WorkflowValidationResult>;
/// Workflow instance API envelope.
pub type WorkflowInstanceApiResponse = ApiResponse<tikee_storage::WorkflowInstanceSummary>;
/// Workflow advance API envelope.
pub type WorkflowAdvanceApiResponse = ApiResponse<tikee_storage::AdvanceWorkflowResult>;
/// Workflow node materialization API envelope.
pub type WorkflowMaterializeApiResponse = ApiResponse<tikee_storage::MaterializeWorkflowNodeResult>;
/// Workflow node recovery API envelope.
pub type WorkflowRecoverApiResponse = ApiResponse<tikee_storage::RecoverWorkflowNodeResult>;
/// Workflow shard list API envelope.
pub type WorkflowShardListApiResponse = ApiResponse<Vec<tikee_storage::WorkflowShardSummary>>;
/// Workflow shard completion API envelope.
pub type WorkflowShardCompleteApiResponse = ApiResponse<tikee_storage::CompleteWorkflowShardResult>;
/// Dispatch queue API envelope.
pub type DispatchQueueApiResponse = ApiResponse<tikee_storage::QueueOverview>;
/// Dispatch queue claim API envelope.
pub type DispatchQueueClaimApiResponse = ApiResponse<tikee_storage::DispatchQueueClaim>;
/// Worker list API envelope.
pub type WorkerListApiResponse = ApiResponse<WorkerListResponse>;
/// Raft `AppendEntries` API envelope.
pub type RaftAppendEntriesApiResponse = ApiResponse<RaftMessageResult>;
/// Raft membership proposal API envelope.
pub type RaftMembershipProposalApiResponse = ApiResponse<RaftMembershipProposalResponse>;
/// Workflow dry-run API envelope.
pub type WorkflowDryRunApiResponse = ApiResponse<WorkflowDryRunResponse>;

/// Workflow dry-run response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WorkflowDryRunResponse {
    /// DAG validation result.
    pub validation: tikee_storage::WorkflowValidationResult,
    /// Nodes without incoming edges.
    pub start_nodes: Vec<String>,
    /// Total node count.
    pub node_count: usize,
    /// Total edge count.
    pub edge_count: usize,
}

/// Online worker summary shown by management UI.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WorkerSummary {
    /// Worker id.
    pub worker_id: String,
    /// Worker app selector.
    pub app: String,
    /// Worker namespace selector.
    pub namespace: String,
    /// Worker cluster.
    pub cluster: String,
    /// Worker region.
    pub region: String,
    /// Runtime capabilities.
    pub capabilities: Vec<String>,
    /// Last heartbeat sequence.
    pub last_sequence: u64,
}

/// Online worker list payload.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WorkerListResponse {
    /// Online worker count.
    pub online: usize,
    /// Online workers.
    pub items: Vec<WorkerSummary>,
}

/// Transport request shape aligned with raft-rs `eraftpb::Message`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RaftAppendEntriesRequest {
    /// Sender raft node id as derived by the server node-id mapping.
    pub from: u64,
    /// Target raft node id.
    pub to: u64,
    /// Sender term.
    pub term: i64,
    /// raft-rs message type name, e.g. `MsgAppend`, `MsgHeartbeat`, or `MsgRequestVote`.
    pub message_type: String,
    /// Log index carried by raft-rs message.
    pub index: i64,
    /// Log term carried by raft-rs message.
    pub log_term: i64,
    /// Commit index carried by raft-rs message.
    pub commit: i64,
    /// Candidate snapshot index, when a snapshot message is carried out-of-band.
    pub snapshot_index: Option<i64>,
    /// Candidate snapshot term, when a snapshot message is carried out-of-band.
    pub snapshot_term: Option<i64>,
    /// Entries carried by append messages. Payloads are base64 strings.
    pub entries: Vec<RaftWireEntry>,
    /// Optional base64 message context.
    pub context: Option<String>,
    /// Whether this is a rejection response.
    pub reject: Option<bool>,
    /// Rejection hint index from raft-rs.
    pub reject_hint: Option<i64>,
    /// Optional fencing token carried by a real leader. Ignored until consensus runtime exists.
    pub leader_fencing_token: Option<String>,
}

/// Wire representation of a raft-rs log entry.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RaftWireEntry {
    /// Entry type name, e.g. `EntryNormal` or `EntryConfChange`.
    pub entry_type: String,
    /// Entry log index.
    pub index: i64,
    /// Entry term.
    pub term: i64,
    /// Base64-encoded entry data.
    pub data: String,
    /// Base64-encoded entry context.
    pub context: Option<String>,
}

/// Response for inbound Raft transport message submission.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RaftMessageResult {
    /// Whether the message was accepted by the local raft-rs runtime inbox.
    pub accepted: bool,
    /// Human-readable reason.
    pub reason: String,
    /// Local node id.
    pub local_node_id: String,
    /// Local cluster role.
    pub local_role: String,
    /// Local leader fencing token, null until real consensus establishes leadership.
    pub leader_fencing_token: Option<String>,
    /// Remote address as reported by proxy headers, when present.
    pub remote_addr: Option<String>,
    /// Received sender term.
    pub received_term: i64,
}

/// Request to create an intentionally gated Raft membership proposal.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RaftMembershipProposalRequest {
    /// Client-provided idempotency key.
    pub proposal_id: String,
    /// Membership action: `add_voter` or `remove_voter`.
    pub action: String,
    /// Target tikee node id.
    pub node_id: String,
    /// Target endpoint for `add_voter`.
    pub endpoint: Option<String>,
}

/// Response for a gated Raft membership proposal intent.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RaftMembershipProposalResponse {
    /// Whether proposal intent was accepted and stored.
    pub accepted: bool,
    /// Human-readable result.
    pub reason: String,
    /// Local node id.
    pub local_node_id: String,
    /// Local cluster role.
    pub local_role: String,
    /// Persisted local leader fencing token.
    pub leader_fencing_token: Option<String>,
    /// Stored proposal summary when the intent was accepted for later `ConfChange` wiring.
    pub proposal: Option<tikee_storage::RaftMembershipProposalSummary>,
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
    /// Optional worker processor binding.
    pub processor_name: Option<String>,
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
    /// Optional worker processor binding.
    pub processor_name: Option<String>,
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
pub type ScriptApiResponse = ApiResponse<tikee_storage::ScriptSummary>;

/// Script page response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScriptPage {
    /// Page items.
    pub items: Vec<tikee_storage::ScriptSummary>,
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
    /// Optional execution policy snapshot. Dangerous capabilities remain rejected in this phase.
    pub policy: Option<serde_json::Value>,
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
    /// Optional execution policy snapshot update. Dangerous capabilities remain rejected in this phase.
    pub policy: Option<serde_json::Value>,
}

/// Publish or rollback script release pointer request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScriptReleaseRequest {
    /// Immutable script version number to release. Defaults to latest version when omitted.
    pub version_number: Option<i64>,
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
    /// Structured governance event name parsed from JSON logs.
    pub governance_event: Option<String>,
    /// Structured governance failure class parsed from JSON logs.
    pub governance_failure_class: Option<String>,
    /// Human-readable governance message parsed from JSON logs.
    pub governance_message: Option<String>,
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
    pub data: tikee_storage::ScriptVersionSummary,
}

/// Response for a list of script versions.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScriptVersionListApiResponse {
    /// List of versions.
    #[schema(value_type = Vec<Object>)]
    pub data: Vec<tikee_storage::ScriptVersionSummary>,
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

/// Audit log export response.
pub type AuditLogExportApiResponse = ApiResponse<AuditLogExport>;

/// Governed audit log export payload.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuditLogExport {
    /// Export format. Currently `json`.
    pub format: String,
    /// Exported audit log entries.
    pub items: Vec<AuditLogSummary>,
    /// Number of exported items.
    pub exported: u64,
    /// Maximum rows allowed in one export.
    pub max_rows: u64,
    /// Whether sensitive snapshot/detail fields were redacted.
    pub redacted: bool,
    /// Governance note for operators.
    pub governance: String,
}

/// Audit log page response.
pub type AuditLogPageApiResponse = ApiResponse<AuditLogPage>;

/// Paginated audit log list.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuditLogPage {
    /// Audit log entries.
    pub items: Vec<AuditLogSummary>,
    /// Total matching row count before pagination.
    pub total: u64,
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
    /// Optional JSON snapshot before the action.
    pub before: Option<String>,
    /// Optional JSON snapshot after the action.
    pub after: Option<String>,
    /// Request trace id.
    pub trace_id: Option<String>,
    /// Result status (`success` or `failed`).
    pub result: String,
    /// Optional failure reason.
    pub failure_reason: Option<String>,
    /// Client IP address.
    pub ip_address: Option<String>,
    /// Creation timestamp.
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
