use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDefinition {
    /// Nodes value.
    pub nodes: Vec<WorkflowNodeSpec>,
    /// Edges value.
    pub edges: Vec<WorkflowEdgeSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowNodeSpec {
    /// Key value.
    pub key: String,
    /// Name value.
    pub name: Option<String>,
    pub kind: Option<String>,
    /// Identifier value.
    pub job_id: Option<String>,
    /// Processor name value.
    pub processor_name: Option<String>,
    /// Identifier value.
    pub child_workflow_id: Option<String>,
    /// Map items value.
    pub map_items: Option<Vec<serde_json::Value>>,
    /// Serialized data value.
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowEdgeSpec {
    /// From value.
    pub from: String,
    /// To value.
    pub to: String,
    /// Condition value.
    pub condition: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateWorkflow {
    /// Name value.
    pub name: String,
    /// Serialized data value.
    pub definition: WorkflowDefinition,
    /// Created by value.
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct UpdateWorkflow {
    /// Name value.
    pub name: String,
    /// Serialized data value.
    pub definition: WorkflowDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSummary {
    /// Identifier value.
    pub id: String,
    /// Name value.
    pub name: String,
    /// Serialized data value.
    pub definition: WorkflowDefinition,
    /// Status value.
    pub status: String,
    /// Created by value.
    pub created_by: String,
    /// Timestamp value.
    pub created_at: String,
    /// Timestamp value.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowValidationResult {
    /// Valid value.
    pub valid: bool,
    /// Errors value.
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdvanceWorkflowInput {
    /// Node key value.
    pub node_key: String,
    /// Status value.
    pub status: String,
    /// Message value.
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdvanceWorkflowResult {
    /// Instance value.
    pub instance: WorkflowInstanceSummary,
    /// Queued nodes value.
    pub queued_nodes: Vec<String>,
    /// Boolean state flag.
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowInstanceSummary {
    /// Identifier value.
    pub id: String,
    /// Identifier value.
    pub workflow_id: String,
    /// Status value.
    pub status: String,
    /// Trigger type value.
    pub trigger_type: String,
    /// Nodes value.
    pub nodes: Vec<WorkflowNodeInstanceSummary>,
    /// Timestamp value.
    pub created_at: String,
    /// Timestamp value.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowNodeInstanceSummary {
    /// Identifier value.
    pub id: String,
    /// Identifier value.
    pub workflow_instance_id: String,
    /// Node key value.
    pub node_key: String,
    /// Status value.
    pub status: String,
    /// Identifier value.
    pub job_instance_id: Option<String>,
    /// Identifier value.
    pub child_workflow_instance_id: Option<String>,
    /// Timestamp value.
    pub created_at: String,
    /// Timestamp value.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowJobBindingSummary {
    /// Node kind value.
    pub node_kind: String,
    /// Processor name value.
    pub processor_name: Option<String>,
    /// Serialized data value.
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowShardSummary {
    /// Identifier value.
    pub id: String,
    /// Identifier value.
    pub workflow_instance_id: String,
    /// Identifier value.
    pub workflow_node_instance_id: String,
    /// Node key value.
    pub node_key: String,
    /// Shard index value.
    pub shard_index: i32,
    /// Status value.
    pub status: String,
    /// Input value.
    pub input: serde_json::Value,
    /// Output value.
    pub output: Option<serde_json::Value>,
    /// Checkpoint value.
    pub checkpoint: Option<serde_json::Value>,
    /// Retry count value.
    pub retry_count: i32,
    /// Identifier value.
    pub job_instance_id: Option<String>,
    /// Timestamp value.
    pub created_at: String,
    /// Timestamp value.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompleteWorkflowShardInput {
    /// Status value.
    pub status: String,
    /// Output value.
    pub output: Option<serde_json::Value>,
    /// Checkpoint value.
    pub checkpoint: Option<serde_json::Value>,
    /// Message value.
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct CompletedShardContext {
    /// Identifier value.
    pub workflow_instance_id: String,
    /// Identifier value.
    pub job_instance_id: Option<String>,
    /// Node key value.
    pub node_key: String,
    /// Updated value.
    pub updated: WorkflowShardSummary,
    /// Has failed value.
    pub has_failed: bool,
    /// All succeeded value.
    pub all_succeeded: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ShardCompletionEventInput {
    /// Identifier value.
    pub workflow_instance_id: String,
    /// Node key value.
    pub node_key: String,
    /// Shard index value.
    pub shard_index: i32,
    /// Status value.
    pub status: String,
    /// Message value.
    pub message: Option<String>,
    /// Output value.
    pub output: Option<String>,
    /// Now value.
    pub now: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompleteWorkflowShardResult {
    /// Shard value.
    pub shard: WorkflowShardSummary,
    /// Node completed value.
    pub node_completed: bool,
    /// Node status value.
    pub node_status: Option<String>,
    /// Advance value.
    pub advance: Option<AdvanceWorkflowResult>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DispatchQueueSloSummary {
    /// Total value.
    pub total: u64,
    /// By status value.
    pub by_status: BTreeMap<String, u64>,
    /// Pending value.
    pub pending: u64,
    /// Running value.
    pub running: u64,
    /// Completed dispatches value.
    pub completed_dispatches: u64,
    /// Average dispatch latency seconds value.
    pub average_dispatch_latency_seconds: u64,
    /// Longest dispatch latency seconds value.
    pub longest_dispatch_latency_seconds: u64,
    /// Oldest pending age seconds value.
    pub oldest_pending_age_seconds: u64,
    /// Average pending age seconds value.
    pub average_pending_age_seconds: u64,
    /// Blocked by quota value.
    pub blocked_by_quota: u64,
    /// Pending by shard owner value.
    pub pending_by_shard_owner: BTreeMap<String, u64>,
    /// Oldest pending age by shard owner value.
    pub oldest_pending_age_by_shard_owner: BTreeMap<String, u64>,
    /// Running by shard owner value.
    pub running_by_shard_owner: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSloSummary {
    /// Instances total value.
    pub instances_total: u64,
    /// Instances by status value.
    pub instances_by_status: BTreeMap<String, u64>,
    /// Completed instances value.
    pub completed_instances: u64,
    /// Instance success ratio value.
    pub instance_success_ratio: f64,
    /// Average instance duration seconds value.
    pub average_instance_duration_seconds: u64,
    /// Longest instance duration seconds value.
    pub longest_instance_duration_seconds: u64,
    /// Shards total value.
    pub shards_total: u64,
    /// Shards by status value.
    pub shards_by_status: BTreeMap<String, u64>,
    /// Completed shards value.
    pub completed_shards: u64,
    /// Shard success ratio value.
    pub shard_success_ratio: f64,
    /// Average shard duration seconds value.
    pub average_shard_duration_seconds: u64,
    /// Longest shard duration seconds value.
    pub longest_shard_duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DispatchQueueSummary {
    /// Identifier value.
    pub id: String,
    /// Identifier value.
    pub job_instance_id: Option<String>,
    /// Identifier value.
    pub workflow_node_instance_id: Option<String>,
    /// Identifier value.
    pub shard_id: Option<i32>,
    /// Shard map version value.
    pub shard_map_version: Option<i64>,
    /// Shard count value.
    pub shard_count: Option<i32>,
    /// Owner epoch value.
    pub owner_epoch: Option<i64>,
    /// Owner fencing token value.
    pub owner_fencing_token: Option<String>,
    /// Priority value.
    pub priority: i32,
    /// Run after value.
    pub run_after: String,
    /// Status value.
    pub status: String,
    /// Attempt value.
    pub attempt: i32,
    /// Lease owner value.
    pub lease_owner: Option<String>,
    /// Timestamp value.
    pub lease_until: Option<String>,
    /// Fencing token value.
    pub fencing_token: Option<String>,
    /// Worker selector value.
    pub worker_selector: Option<String>,
    /// Timestamp value.
    pub created_at: String,
    /// Timestamp value.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DispatchQueueClaim {
    /// Item value.
    pub item: DispatchQueueSummary,
    /// Lease owner value.
    pub lease_owner: String,
    /// Timestamp value.
    pub lease_until: String,
    /// Fencing token value.
    pub fencing_token: String,
}

#[derive(Debug, Clone)]
pub struct DispatchQueueShardOwner {
    /// Identifier value.
    pub shard_id: i32,
    /// Shard map version value.
    pub shard_map_version: i64,
    /// Shard count value.
    pub shard_count: i32,
    /// Identifier value.
    pub owner_node_id: String,
    /// Owner epoch value.
    pub owner_epoch: i64,
    /// Owner fencing token value.
    pub owner_fencing_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueueOverview {
    /// Pending value.
    pub pending: usize,
    /// Running value.
    pub running: usize,
    /// Done value.
    pub done: usize,
    /// Failed value.
    pub failed: usize,
    /// Items value.
    pub items: Vec<DispatchQueueSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecoverWorkflowNodeInput {
    /// Node key value.
    pub node_key: String,
    /// Action value.
    pub action: String,
    /// Message value.
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RebalanceWorkflowShardsInput {
    /// Node key value.
    pub node_key: Option<String>,
    /// Statuses value.
    pub statuses: Option<Vec<String>>,
    /// Message value.
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RebalanceWorkflowShardsResult {
    /// Requeued shards value.
    pub requeued_shards: Vec<WorkflowShardSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaterializeWorkflowNodeResult {
    /// Instance value.
    pub instance: WorkflowInstanceSummary,
    /// Node value.
    pub node: WorkflowNodeInstanceSummary,
    /// Shards value.
    pub shards: Vec<WorkflowShardSummary>,
    /// Queue item value.
    pub queue_item: DispatchQueueSummary,
}

#[derive(Debug, Clone)]
pub struct WorkflowJobResultOutcome {
    /// Identifier value.
    pub workflow_instance_id: String,
    /// Node key value.
    pub node_key: String,
    /// Status value.
    pub status: String,
    /// Queued nodes value.
    pub queued_nodes: Vec<String>,
    /// Boolean state flag.
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecoverWorkflowNodeResult {
    /// Instance value.
    pub instance: WorkflowInstanceSummary,
    /// Queued nodes value.
    pub queued_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstanceEventSummary {
    /// Identifier value.
    pub id: String,
    /// Identifier value.
    pub instance_id: String,
    /// Instance type value.
    pub instance_type: String,
    /// Event type value.
    pub event_type: String,
    /// Message value.
    pub message: String,
    /// Serialized data value.
    pub payload: Option<String>,
    /// Timestamp value.
    pub created_at: String,
}
