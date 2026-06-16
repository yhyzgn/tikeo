use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDefinition {
    pub nodes: Vec<WorkflowNodeSpec>,
    pub edges: Vec<WorkflowEdgeSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowNodeSpec {
    pub key: String,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub job_id: Option<String>,
    pub processor_name: Option<String>,
    pub child_workflow_id: Option<String>,
    pub map_items: Option<Vec<serde_json::Value>>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowEdgeSpec {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateWorkflow {
    pub name: String,
    pub definition: WorkflowDefinition,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct UpdateWorkflow {
    pub name: String,
    pub definition: WorkflowDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub definition: WorkflowDefinition,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdvanceWorkflowInput {
    pub node_key: String,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdvanceWorkflowResult {
    pub instance: WorkflowInstanceSummary,
    pub queued_nodes: Vec<String>,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowInstanceSummary {
    pub id: String,
    pub workflow_id: String,
    pub status: String,
    pub trigger_type: String,
    pub nodes: Vec<WorkflowNodeInstanceSummary>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowNodeInstanceSummary {
    pub id: String,
    pub workflow_instance_id: String,
    pub node_key: String,
    pub status: String,
    pub job_instance_id: Option<String>,
    pub child_workflow_instance_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowJobBindingSummary {
    pub node_kind: String,
    pub processor_name: Option<String>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowShardSummary {
    pub id: String,
    pub workflow_instance_id: String,
    pub workflow_node_instance_id: String,
    pub node_key: String,
    pub shard_index: i32,
    pub status: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub checkpoint: Option<serde_json::Value>,
    pub retry_count: i32,
    pub job_instance_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompleteWorkflowShardInput {
    pub status: String,
    pub output: Option<serde_json::Value>,
    pub checkpoint: Option<serde_json::Value>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct CompletedShardContext {
    pub workflow_instance_id: String,
    pub job_instance_id: Option<String>,
    pub node_key: String,
    pub updated: WorkflowShardSummary,
    pub has_failed: bool,
    pub all_succeeded: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ShardCompletionEventInput {
    pub workflow_instance_id: String,
    pub node_key: String,
    pub shard_index: i32,
    pub status: String,
    pub message: Option<String>,
    pub output: Option<String>,
    pub now: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompleteWorkflowShardResult {
    pub shard: WorkflowShardSummary,
    pub node_completed: bool,
    pub node_status: Option<String>,
    pub advance: Option<AdvanceWorkflowResult>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DispatchQueueSloSummary {
    pub total: u64,
    pub by_status: BTreeMap<String, u64>,
    pub pending: u64,
    pub running: u64,
    pub completed_dispatches: u64,
    pub average_dispatch_latency_seconds: u64,
    pub longest_dispatch_latency_seconds: u64,
    pub oldest_pending_age_seconds: u64,
    pub average_pending_age_seconds: u64,
    pub blocked_by_quota: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSloSummary {
    pub instances_total: u64,
    pub instances_by_status: BTreeMap<String, u64>,
    pub completed_instances: u64,
    pub instance_success_ratio: f64,
    pub average_instance_duration_seconds: u64,
    pub longest_instance_duration_seconds: u64,
    pub shards_total: u64,
    pub shards_by_status: BTreeMap<String, u64>,
    pub completed_shards: u64,
    pub shard_success_ratio: f64,
    pub average_shard_duration_seconds: u64,
    pub longest_shard_duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DispatchQueueSummary {
    pub id: String,
    pub job_instance_id: Option<String>,
    pub workflow_node_instance_id: Option<String>,
    pub shard_id: Option<i32>,
    pub owner_epoch: Option<i64>,
    pub owner_fencing_token: Option<String>,
    pub priority: i32,
    pub run_after: String,
    pub status: String,
    pub attempt: i32,
    pub lease_owner: Option<String>,
    pub lease_until: Option<String>,
    pub fencing_token: Option<String>,
    pub worker_selector: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DispatchQueueClaim {
    pub item: DispatchQueueSummary,
    pub lease_owner: String,
    pub lease_until: String,
    pub fencing_token: String,
}

#[derive(Debug, Clone)]
pub struct DispatchQueueShardOwner {
    pub shard_id: i32,
    pub owner_node_id: String,
    pub owner_epoch: i64,
    pub owner_fencing_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueueOverview {
    pub pending: usize,
    pub running: usize,
    pub done: usize,
    pub failed: usize,
    pub items: Vec<DispatchQueueSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecoverWorkflowNodeInput {
    pub node_key: String,
    pub action: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RebalanceWorkflowShardsInput {
    pub node_key: Option<String>,
    pub statuses: Option<Vec<String>>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RebalanceWorkflowShardsResult {
    pub requeued_shards: Vec<WorkflowShardSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaterializeWorkflowNodeResult {
    pub instance: WorkflowInstanceSummary,
    pub node: WorkflowNodeInstanceSummary,
    pub shards: Vec<WorkflowShardSummary>,
    pub queue_item: DispatchQueueSummary,
}

#[derive(Debug, Clone)]
pub struct WorkflowJobResultOutcome {
    pub workflow_instance_id: String,
    pub node_key: String,
    pub status: String,
    pub queued_nodes: Vec<String>,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecoverWorkflowNodeResult {
    pub instance: WorkflowInstanceSummary,
    pub queued_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstanceEventSummary {
    pub id: String,
    pub instance_id: String,
    pub instance_type: String,
    pub event_type: String,
    pub message: String,
    pub payload: Option<String>,
    pub created_at: String,
}
