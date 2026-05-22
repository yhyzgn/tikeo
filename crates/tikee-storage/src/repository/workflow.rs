#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap, HashSet};

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait, sea_query::Expr,
};
use serde::{Deserialize, Serialize};
use tikee_core::InstanceStatus;

use crate::entities::{
    app, dispatch_queue, instance_event, job, namespace, workflow, workflow_edge,
    workflow_instance, workflow_node, workflow_node_instance, workflow_shard,
};

use super::util::{new_id, now_rfc3339, rfc3339_after_seconds};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WorkflowDefinition {
    pub nodes: Vec<WorkflowNodeSpec>,
    pub edges: Vec<WorkflowEdgeSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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
pub struct WorkflowValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AdvanceWorkflowInput {
    pub node_key: String,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AdvanceWorkflowResult {
    pub instance: WorkflowInstanceSummary,
    pub queued_nodes: Vec<String>,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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
pub struct WorkflowShardSummary {
    pub id: String,
    pub workflow_instance_id: String,
    pub workflow_node_instance_id: String,
    pub node_key: String,
    pub shard_index: i32,
    pub status: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub job_instance_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CompleteWorkflowShardInput {
    pub status: String,
    pub output: Option<serde_json::Value>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CompleteWorkflowShardResult {
    pub shard: WorkflowShardSummary,
    pub node_completed: bool,
    pub node_status: Option<String>,
    pub advance: Option<AdvanceWorkflowResult>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DispatchQueueSloSummary {
    pub total: u64,
    pub by_status: BTreeMap<String, u64>,
    pub pending: u64,
    pub running: u64,
    pub oldest_pending_age_seconds: u64,
    pub average_pending_age_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DispatchQueueSummary {
    pub id: String,
    pub job_instance_id: Option<String>,
    pub workflow_node_instance_id: Option<String>,
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
pub struct DispatchQueueClaim {
    pub item: DispatchQueueSummary,
    pub lease_owner: String,
    pub lease_until: String,
    pub fencing_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct QueueOverview {
    pub pending: usize,
    pub running: usize,
    pub done: usize,
    pub failed: usize,
    pub items: Vec<DispatchQueueSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecoverWorkflowNodeInput {
    pub node_key: String,
    pub action: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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
pub struct RecoverWorkflowNodeResult {
    pub instance: WorkflowInstanceSummary,
    pub queued_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InstanceEventSummary {
    pub id: String,
    pub instance_id: String,
    pub instance_type: String,
    pub event_type: String,
    pub message: String,
    pub payload: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowRepository {
    db: DatabaseConnection,
}

impl WorkflowRepository {
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_workflow(
        &self,
        input: CreateWorkflow,
    ) -> Result<WorkflowSummary, sea_orm::DbErr> {
        let validation = validate_workflow_definition(&input.definition);
        if !validation.valid {
            return Err(sea_orm::DbErr::Custom(validation.errors.join("; ")));
        }
        let now = now_rfc3339();
        let workflow_id = new_id("wf");
        let definition_json = serde_json::to_string(&input.definition)
            .map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?;
        let txn = self.db.begin().await?;
        let model = workflow::ActiveModel {
            id: Set(workflow_id.clone()),
            name: Set(input.name),
            definition: Set(definition_json),
            status: Set("active".to_owned()),
            created_by: Set(input.created_by),
            created_at: Set(now.clone()),
            updated_at: Set(now.clone()),
        }
        .insert(&txn)
        .await?;
        for node in &input.definition.nodes {
            workflow_node::ActiveModel {
                id: Set(new_id("wfn")),
                workflow_id: Set(workflow_id.clone()),
                node_key: Set(node.key.clone()),
                name: Set(node.name.clone().unwrap_or_else(|| node.key.clone())),
                kind: Set(node.kind.clone().unwrap_or_else(|| "job".to_owned())),
                job_id: Set(node.job_id.clone()),
                processor_name: Set(normalize_processor_name(node.processor_name.clone())),
                config: Set(node
                    .config
                    .as_ref()
                    .and_then(|value| serde_json::to_string(value).ok())),
                created_at: Set(now.clone()),
            }
            .insert(&txn)
            .await?;
        }
        for edge in &input.definition.edges {
            workflow_edge::ActiveModel {
                id: Set(new_id("wfe")),
                workflow_id: Set(workflow_id.clone()),
                from_node_key: Set(edge.from.clone()),
                to_node_key: Set(edge.to.clone()),
                condition: Set(edge
                    .condition
                    .clone()
                    .unwrap_or_else(|| "always".to_owned())),
                created_at: Set(now.clone()),
            }
            .insert(&txn)
            .await?;
        }
        txn.commit().await?;
        WorkflowSummary::from_model(model)
    }

    pub async fn update_workflow(
        &self,
        id: &str,
        input: UpdateWorkflow,
    ) -> Result<Option<WorkflowSummary>, sea_orm::DbErr> {
        let validation = validate_workflow_definition(&input.definition);
        if !validation.valid {
            return Err(sea_orm::DbErr::Custom(validation.errors.join("; ")));
        }
        let Some(existing) = workflow::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let now = now_rfc3339();
        let definition_json = serde_json::to_string(&input.definition)
            .map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?;
        let txn = self.db.begin().await?;
        workflow_node::Entity::delete_many()
            .filter(workflow_node::Column::WorkflowId.eq(id.to_owned()))
            .exec(&txn)
            .await?;
        workflow_edge::Entity::delete_many()
            .filter(workflow_edge::Column::WorkflowId.eq(id.to_owned()))
            .exec(&txn)
            .await?;
        let mut active: workflow::ActiveModel = existing.into();
        active.name = Set(input.name);
        active.definition = Set(definition_json);
        active.updated_at = Set(now.clone());
        let model = active.update(&txn).await?;
        for node in &input.definition.nodes {
            workflow_node::ActiveModel {
                id: Set(new_id("wfn")),
                workflow_id: Set(id.to_owned()),
                node_key: Set(node.key.clone()),
                name: Set(node.name.clone().unwrap_or_else(|| node.key.clone())),
                kind: Set(node.kind.clone().unwrap_or_else(|| "job".to_owned())),
                job_id: Set(node.job_id.clone()),
                processor_name: Set(normalize_processor_name(node.processor_name.clone())),
                config: Set(node
                    .config
                    .as_ref()
                    .and_then(|value| serde_json::to_string(value).ok())),
                created_at: Set(now.clone()),
            }
            .insert(&txn)
            .await?;
        }
        for edge in &input.definition.edges {
            workflow_edge::ActiveModel {
                id: Set(new_id("wfe")),
                workflow_id: Set(id.to_owned()),
                from_node_key: Set(edge.from.clone()),
                to_node_key: Set(edge.to.clone()),
                condition: Set(edge
                    .condition
                    .clone()
                    .unwrap_or_else(|| "always".to_owned())),
                created_at: Set(now.clone()),
            }
            .insert(&txn)
            .await?;
        }
        txn.commit().await?;
        WorkflowSummary::from_model(model).map(Some)
    }

    pub async fn list_workflows(&self) -> Result<Vec<WorkflowSummary>, sea_orm::DbErr> {
        let rows = workflow::Entity::find()
            .order_by_desc(workflow::Column::CreatedAt)
            .all(&self.db)
            .await?;
        rows.into_iter().map(WorkflowSummary::from_model).collect()
    }

    pub async fn get_workflow(&self, id: &str) -> Result<Option<WorkflowSummary>, sea_orm::DbErr> {
        workflow::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
            .map(WorkflowSummary::from_model)
            .transpose()
    }

    pub async fn run_workflow(
        &self,
        workflow_id: &str,
        trigger_type: &str,
    ) -> Result<Option<WorkflowInstanceSummary>, sea_orm::DbErr> {
        let Some(workflow) = self.get_workflow(workflow_id).await? else {
            return Ok(None);
        };
        let now = now_rfc3339();
        let instance_id = new_id("wfi");
        let txn = self.db.begin().await?;
        let instance = workflow_instance::ActiveModel {
            id: Set(instance_id.clone()),
            workflow_id: Set(workflow_id.to_owned()),
            status: Set("pending".to_owned()),
            trigger_type: Set(trigger_type.to_owned()),
            created_at: Set(now.clone()),
            updated_at: Set(now.clone()),
        }
        .insert(&txn)
        .await?;
        let start_nodes = start_node_keys(&workflow.definition);
        let mut node_summaries = Vec::new();
        for node in &workflow.definition.nodes {
            let is_start = start_nodes.contains(&node.key);
            let node_instance = workflow_node_instance::ActiveModel {
                id: Set(new_id("wfni")),
                workflow_instance_id: Set(instance_id.clone()),
                node_key: Set(node.key.clone()),
                status: Set(if is_start { "queued" } else { "waiting" }.to_owned()),
                job_instance_id: Set(None),
                child_workflow_instance_id: Set(None),
                created_at: Set(now.clone()),
                updated_at: Set(now.clone()),
            }
            .insert(&txn)
            .await?;
            if is_start {
                dispatch_queue::ActiveModel {
                    id: Set(new_id("dq")),
                    job_instance_id: Set(None),
                    workflow_node_instance_id: Set(Some(node_instance.id.clone())),
                    priority: Set(0),
                    run_after: Set(now.clone()),
                    status: Set("pending".to_owned()),
                    attempt: Set(0),
                    lease_owner: Set(None),
                    lease_until: Set(None),
                    fencing_token: Set(None),
                    worker_selector: Set(None),
                    created_at: Set(now.clone()),
                    updated_at: Set(now.clone()),
                }
                .insert(&txn)
                .await?;
            }
            node_summaries.push(WorkflowNodeInstanceSummary::from(node_instance));
        }
        instance_event::ActiveModel {
            id: Set(new_id("evt")),
            instance_id: Set(instance_id.clone()),
            instance_type: Set("workflow".to_owned()),
            event_type: Set("workflow.started".to_owned()),
            message: Set(format!("workflow {workflow_id} started")),
            payload: Set(None),
            created_at: Set(now),
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        Ok(Some(WorkflowInstanceSummary::from_model(
            instance,
            node_summaries,
        )))
    }

    #[allow(clippy::too_many_lines)]
    pub async fn advance_workflow(
        &self,
        instance_id: &str,
        input: AdvanceWorkflowInput,
    ) -> Result<Option<AdvanceWorkflowResult>, sea_orm::DbErr> {
        let Some(instance) = workflow_instance::Entity::find_by_id(instance_id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let Some(workflow) = self.get_workflow(&instance.workflow_id).await? else {
            return Ok(None);
        };
        let Some(node_model) = workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::WorkflowInstanceId.eq(instance_id.to_owned()))
            .filter(workflow_node_instance::Column::NodeKey.eq(input.node_key.clone()))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let now = now_rfc3339();
        let txn = self.db.begin().await?;
        let mut active: workflow_node_instance::ActiveModel = node_model.into();
        active.status = Set(input.status.clone());
        active.updated_at = Set(now.clone());
        active.update(&txn).await?;

        instance_event::ActiveModel {
            id: Set(new_id("evt")),
            instance_id: Set(instance_id.to_owned()),
            instance_type: Set("workflow".to_owned()),
            event_type: Set(format!("workflow.node.{}", input.status)),
            message: Set(input
                .message
                .unwrap_or_else(|| format!("node {} {}", input.node_key, input.status))),
            payload: Set(None),
            created_at: Set(now.clone()),
        }
        .insert(&txn)
        .await?;

        let mut queued_nodes = Vec::new();
        if input.status == "succeeded" || input.status == "failed" {
            let eligible =
                next_nodes_for_status(&workflow.definition, &input.node_key, &input.status);
            for node_key in eligible {
                if all_predecessors_satisfied(&workflow.definition, &node_key, instance_id, &txn)
                    .await?
                    && let Some(waiting) = workflow_node_instance::Entity::find()
                        .filter(
                            workflow_node_instance::Column::WorkflowInstanceId
                                .eq(instance_id.to_owned()),
                        )
                        .filter(workflow_node_instance::Column::NodeKey.eq(node_key.clone()))
                        .one(&txn)
                        .await?
                        .filter(|node| node.status == "waiting")
                {
                    let mut waiting_active: workflow_node_instance::ActiveModel = waiting.into();
                    waiting_active.status = Set("queued".to_owned());
                    waiting_active.updated_at = Set(now.clone());
                    let queued = waiting_active.update(&txn).await?;
                    dispatch_queue::ActiveModel {
                        id: Set(new_id("dq")),
                        job_instance_id: Set(None),
                        workflow_node_instance_id: Set(Some(queued.id)),
                        priority: Set(0),
                        run_after: Set(now.clone()),
                        status: Set("pending".to_owned()),
                        attempt: Set(0),
                        lease_owner: Set(None),
                        lease_until: Set(None),
                        fencing_token: Set(None),
                        worker_selector: Set(None),
                        created_at: Set(now.clone()),
                        updated_at: Set(now.clone()),
                    }
                    .insert(&txn)
                    .await?;
                    queued_nodes.push(node_key);
                }
            }
        }

        let node_rows = workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::WorkflowInstanceId.eq(instance_id.to_owned()))
            .all(&txn)
            .await?;
        let completed = node_rows
            .iter()
            .all(|node| matches!(node.status.as_str(), "succeeded" | "failed" | "skipped"));
        let next_instance_status = if completed {
            if node_rows.iter().any(|node| node.status == "failed") {
                "failed"
            } else {
                "succeeded"
            }
        } else {
            "running"
        };
        let mut instance_active: workflow_instance::ActiveModel = instance.into();
        instance_active.status = Set(next_instance_status.to_owned());
        instance_active.updated_at = Set(now.clone());
        instance_active.update(&txn).await?;
        if completed {
            instance_event::ActiveModel {
                id: Set(new_id("evt")),
                instance_id: Set(instance_id.to_owned()),
                instance_type: Set("workflow".to_owned()),
                event_type: Set(format!("workflow.{next_instance_status}")),
                message: Set(format!("workflow {instance_id} {next_instance_status}")),
                payload: Set(None),
                created_at: Set(now),
            }
            .insert(&txn)
            .await?;
        }
        txn.commit().await?;
        let refreshed = self
            .get_workflow_instance(instance_id)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(instance_id.to_owned()))?;
        let result = AdvanceWorkflowResult {
            instance: refreshed,
            queued_nodes,
            completed,
        };
        if result.completed {
            self.propagate_child_workflow_completion(&result.instance)
                .await?;
        }
        Ok(Some(result))
    }

    #[allow(clippy::too_many_lines)]
    pub async fn materialize_next_queued_node(
        &self,
    ) -> Result<Option<MaterializeWorkflowNodeResult>, sea_orm::DbErr> {
        self.materialize_next_queued_node_with_lease("tikee-dispatcher", 30)
            .await
    }

    #[allow(clippy::too_many_lines)]
    pub async fn materialize_next_queued_node_with_lease(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
    ) -> Result<Option<MaterializeWorkflowNodeResult>, sea_orm::DbErr> {
        self.materialize_next_queued_node_with_fencing(lease_owner, lease_seconds, lease_owner)
            .await
    }

    #[allow(clippy::too_many_lines)]
    pub async fn materialize_next_queued_node_with_fencing(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
        fencing_token: &str,
    ) -> Result<Option<MaterializeWorkflowNodeResult>, sea_orm::DbErr> {
        let Some(claim) = self
            .claim_next_workflow_node_queue_item_with_fencing(
                lease_owner,
                lease_seconds,
                fencing_token,
            )
            .await?
        else {
            return Ok(None);
        };
        let Some(queue_row) = dispatch_queue::Entity::find_by_id(claim.item.id.clone())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let Some(node_instance_id) = queue_row.workflow_node_instance_id.clone() else {
            return Ok(None);
        };
        let Some(node_instance) = workflow_node_instance::Entity::find_by_id(node_instance_id)
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let Some(instance) =
            workflow_instance::Entity::find_by_id(node_instance.workflow_instance_id.clone())
                .one(&self.db)
                .await?
        else {
            return Ok(None);
        };
        let Some(workflow) = self.get_workflow(&instance.workflow_id).await? else {
            return Ok(None);
        };
        let Some(node_spec) = workflow
            .definition
            .nodes
            .iter()
            .find(|node| node.key == node_instance.node_key)
            .cloned()
        else {
            return Ok(None);
        };
        let now = now_rfc3339();
        let txn = self.db.begin().await?;
        let mut queue_active: dispatch_queue::ActiveModel = queue_row.clone().into();
        queue_active.status = Set("running".to_owned());
        queue_active.updated_at = Set(now.clone());
        queue_active.update(&txn).await?;

        let mut node_active: workflow_node_instance::ActiveModel = node_instance.into();
        node_active.status = Set("running".to_owned());
        node_active.updated_at = Set(now.clone());
        let mut shards = Vec::new();
        match node_kind(&node_spec) {
            "job" => {
                let job_instance_id = new_id("inst");
                let job_id = node_spec.job_id.clone().unwrap_or_default();
                ensure_workflow_job_soft_link(&txn, &job_id, &now).await?;
                crate::entities::job_instance::ActiveModel {
                    id: Set(job_instance_id.clone()),
                    job_id: Set(job_id),
                    status: Set("pending".to_owned()),
                    trigger_type: Set("workflow".to_owned()),
                    execution_mode: Set("single".to_owned()),
                    created_at: Set(now.clone()),
                    updated_at: Set(now.clone()),
                }
                .insert(&txn)
                .await?;
                node_active.job_instance_id = Set(Some(job_instance_id.clone()));
                dispatch_queue::ActiveModel {
                    id: Set(new_id("dq")),
                    job_instance_id: Set(Some(job_instance_id)),
                    workflow_node_instance_id: Set(None),
                    priority: Set(0),
                    run_after: Set(now.clone()),
                    status: Set("pending".to_owned()),
                    attempt: Set(0),
                    lease_owner: Set(None),
                    lease_until: Set(None),
                    fencing_token: Set(None),
                    worker_selector: Set(None),
                    created_at: Set(now.clone()),
                    updated_at: Set(now.clone()),
                }
                .insert(&txn)
                .await?;
            }
            "map" | "map_reduce" => {
                let shard_job_id = node_spec.job_id.clone().unwrap_or_else(|| {
                    format!("workflow-shard-{}-{}", instance.workflow_id, node_spec.key)
                });
                ensure_workflow_job_soft_link(&txn, &shard_job_id, &now).await?;
                for (index, item) in node_spec
                    .map_items
                    .unwrap_or_default()
                    .into_iter()
                    .enumerate()
                {
                    let job_instance_id = new_id("inst");
                    let shard = workflow_shard::ActiveModel {
                        id: Set(new_id("wfs")),
                        workflow_instance_id: Set(instance.id.clone()),
                        workflow_node_instance_id: Set(node_active.id.clone().unwrap()),
                        node_key: Set(node_spec.key.clone()),
                        shard_index: Set(i32::try_from(index).unwrap_or(i32::MAX)),
                        status: Set("pending".to_owned()),
                        input: Set(
                            serde_json::to_string(&item).unwrap_or_else(|_| "null".to_owned())
                        ),
                        output: Set(None),
                        job_instance_id: Set(Some(job_instance_id.clone())),
                        created_at: Set(now.clone()),
                        updated_at: Set(now.clone()),
                    }
                    .insert(&txn)
                    .await?;
                    crate::entities::job_instance::ActiveModel {
                        id: Set(job_instance_id.clone()),
                        job_id: Set(shard_job_id.clone()),
                        status: Set("pending".to_owned()),
                        trigger_type: Set("workflow_shard".to_owned()),
                        execution_mode: Set("single".to_owned()),
                        created_at: Set(now.clone()),
                        updated_at: Set(now.clone()),
                    }
                    .insert(&txn)
                    .await?;
                    dispatch_queue::ActiveModel {
                        id: Set(new_id("dq")),
                        job_instance_id: Set(Some(job_instance_id)),
                        workflow_node_instance_id: Set(None),
                        priority: Set(0),
                        run_after: Set(now.clone()),
                        status: Set("pending".to_owned()),
                        attempt: Set(0),
                        lease_owner: Set(None),
                        lease_until: Set(None),
                        fencing_token: Set(None),
                        worker_selector: Set(None),
                        created_at: Set(now.clone()),
                        updated_at: Set(now.clone()),
                    }
                    .insert(&txn)
                    .await?;
                    shards.push(WorkflowShardSummary::from(shard));
                }
            }
            "sub_workflow" => {
                let child_id = new_id("wfi");
                let child_workflow_id = node_spec.child_workflow_id.clone().unwrap_or_default();
                workflow_instance::ActiveModel {
                    id: Set(child_id.clone()),
                    workflow_id: Set(child_workflow_id.clone()),
                    status: Set("pending".to_owned()),
                    trigger_type: Set("sub_workflow".to_owned()),
                    created_at: Set(now.clone()),
                    updated_at: Set(now.clone()),
                }
                .insert(&txn)
                .await?;
                if let Some(child_workflow) = self.get_workflow(&child_workflow_id).await? {
                    let child_start_nodes = start_node_keys(&child_workflow.definition);
                    for child_node in &child_workflow.definition.nodes {
                        let is_start = child_start_nodes.contains(&child_node.key);
                        let child_node_instance = workflow_node_instance::ActiveModel {
                            id: Set(new_id("wfni")),
                            workflow_instance_id: Set(child_id.clone()),
                            node_key: Set(child_node.key.clone()),
                            status: Set(if is_start { "queued" } else { "waiting" }.to_owned()),
                            job_instance_id: Set(None),
                            child_workflow_instance_id: Set(None),
                            created_at: Set(now.clone()),
                            updated_at: Set(now.clone()),
                        }
                        .insert(&txn)
                        .await?;
                        if is_start {
                            dispatch_queue::ActiveModel {
                                id: Set(new_id("dq")),
                                job_instance_id: Set(None),
                                workflow_node_instance_id: Set(Some(child_node_instance.id)),
                                priority: Set(0),
                                run_after: Set(now.clone()),
                                status: Set("pending".to_owned()),
                                attempt: Set(0),
                                lease_owner: Set(None),
                                lease_until: Set(None),
                                fencing_token: Set(None),
                                worker_selector: Set(None),
                                created_at: Set(now.clone()),
                                updated_at: Set(now.clone()),
                            }
                            .insert(&txn)
                            .await?;
                        }
                    }
                    instance_event::ActiveModel {
                        id: Set(new_id("evt")),
                        instance_id: Set(child_id.clone()),
                        instance_type: Set("workflow".to_owned()),
                        event_type: Set("workflow.started".to_owned()),
                        message: Set(format!("workflow {child_workflow_id} started")),
                        payload: Set(None),
                        created_at: Set(now.clone()),
                    }
                    .insert(&txn)
                    .await?;
                }
                node_active.child_workflow_instance_id = Set(Some(child_id.clone()));
                instance_event::ActiveModel {
                    id: Set(new_id("evt")),
                    instance_id: Set(instance.id.clone()),
                    instance_type: Set("workflow".to_owned()),
                    event_type: Set("workflow.sub_workflow.started".to_owned()),
                    message: Set(format!("child workflow {child_id} started")),
                    payload: Set(None),
                    created_at: Set(now.clone()),
                }
                .insert(&txn)
                .await?;
            }
            _ => {}
        }
        let updated_node = node_active.update(&txn).await?;
        let mut queue_done: dispatch_queue::ActiveModel = queue_row.into();
        queue_done.status = Set("done".to_owned());
        queue_done.lease_owner = Set(None);
        queue_done.lease_until = Set(None);
        queue_done.updated_at = Set(now.clone());
        let updated_queue = queue_done.update(&txn).await?;
        instance_event::ActiveModel {
            id: Set(new_id("evt")),
            instance_id: Set(instance.id.clone()),
            instance_type: Set("workflow".to_owned()),
            event_type: Set("workflow.node.materialized".to_owned()),
            message: Set(format!("node {} materialized", updated_node.node_key)),
            payload: Set(None),
            created_at: Set(now),
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        let refreshed = self
            .get_workflow_instance(&instance.id)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(instance.id.clone()))?;
        Ok(Some(MaterializeWorkflowNodeResult {
            instance: refreshed,
            node: WorkflowNodeInstanceSummary::from(updated_node),
            shards,
            queue_item: DispatchQueueSummary::from(updated_queue),
        }))
    }

    pub async fn processor_name_for_job_instance(
        &self,
        job_instance_id: &str,
    ) -> Result<Option<String>, sea_orm::DbErr> {
        if crate::entities::job_instance::Entity::find_by_id(job_instance_id.to_owned())
            .one(&self.db)
            .await?
            .is_none()
        {
            return Ok(None);
        }

        if let Some(node_instance) = workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&self.db)
            .await?
            && let Some(workflow_instance) =
                workflow_instance::Entity::find_by_id(node_instance.workflow_instance_id.clone())
                    .one(&self.db)
                    .await?
            && let Some(workflow) = self.get_workflow(&workflow_instance.workflow_id).await?
            && let Some(node) = workflow
                .definition
                .nodes
                .iter()
                .find(|node| node.key == node_instance.node_key)
            && let Some(processor_name) = normalize_processor_name(node.processor_name.clone())
        {
            return Ok(Some(processor_name));
        }

        if let Some(shard) = workflow_shard::Entity::find()
            .filter(workflow_shard::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&self.db)
            .await?
            && let Some(workflow) = self.get_workflow(&shard.workflow_instance_id).await?
            && let Some(node) = workflow
                .definition
                .nodes
                .iter()
                .find(|node| node.key == shard.node_key)
            && let Some(processor_name) = normalize_processor_name(node.processor_name.clone())
        {
            return Ok(Some(processor_name));
        }

        Ok(None)
    }

    pub async fn get_node_by_job_instance(
        &self,
        job_instance_id: &str,
    ) -> Result<Option<WorkflowNodeInstanceSummary>, sea_orm::DbErr> {
        workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&self.db)
            .await
            .map(|model| model.map(WorkflowNodeInstanceSummary::from))
    }

    pub async fn complete_job_node_from_result(
        &self,
        job_instance_id: &str,
        status: InstanceStatus,
        message: Option<String>,
    ) -> Result<Option<WorkflowJobResultOutcome>, sea_orm::DbErr> {
        let terminal_status = if status == InstanceStatus::Succeeded {
            "succeeded"
        } else {
            "failed"
        }
        .to_owned();
        if let Some(shard_result) = self
            .complete_shard_by_job_instance(
                job_instance_id,
                CompleteWorkflowShardInput {
                    status: terminal_status.clone(),
                    output: None,
                    message: message.clone(),
                },
            )
            .await?
        {
            self.mark_job_queue_done(job_instance_id, terminal_status.as_str())
                .await?;
            return Ok(shard_result
                .advance
                .map(|advance| WorkflowJobResultOutcome {
                    workflow_instance_id: advance.instance.id,
                    node_key: shard_result.shard.node_key,
                    status: terminal_status,
                    queued_nodes: advance.queued_nodes,
                    completed: advance.completed,
                }));
        }
        let Some(node) = workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let node_key = node.node_key.clone();
        let workflow_instance_id = node.workflow_instance_id.clone();
        let advance = self
            .advance_workflow(
                &workflow_instance_id,
                AdvanceWorkflowInput {
                    node_key: node_key.clone(),
                    status: terminal_status.clone(),
                    message: message.or_else(|| {
                        Some(format!(
                            "job instance {job_instance_id} completed as {terminal_status}"
                        ))
                    }),
                },
            )
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(workflow_instance_id.clone()))?;
        self.mark_job_queue_done(job_instance_id, terminal_status.as_str())
            .await?;
        Ok(Some(WorkflowJobResultOutcome {
            workflow_instance_id,
            node_key,
            status: terminal_status,
            queued_nodes: advance.queued_nodes,
            completed: advance.completed,
        }))
    }

    async fn complete_shard_by_job_instance(
        &self,
        job_instance_id: &str,
        input: CompleteWorkflowShardInput,
    ) -> Result<Option<CompleteWorkflowShardResult>, sea_orm::DbErr> {
        let Some(shard) = workflow_shard::Entity::find()
            .filter(workflow_shard::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        self.complete_workflow_shard(&shard.id, input).await
    }

    async fn mark_job_queue_done(
        &self,
        job_instance_id: &str,
        node_status: &str,
    ) -> Result<(), sea_orm::DbErr> {
        let Some(queue_row) = dispatch_queue::Entity::find()
            .filter(dispatch_queue::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&self.db)
            .await?
        else {
            return Ok(());
        };
        let mut active: dispatch_queue::ActiveModel = queue_row.into();
        active.status = Set(if node_status == "succeeded" {
            "done".to_owned()
        } else {
            "failed".to_owned()
        });
        active.lease_owner = Set(None);
        active.lease_until = Set(None);
        active.fencing_token = Set(None);
        active.updated_at = Set(now_rfc3339());
        active.update(&self.db).await?;
        Ok(())
    }

    pub async fn list_workflow_shards(
        &self,
        instance_id: &str,
    ) -> Result<Vec<WorkflowShardSummary>, sea_orm::DbErr> {
        let rows = workflow_shard::Entity::find()
            .filter(workflow_shard::Column::WorkflowInstanceId.eq(instance_id.to_owned()))
            .order_by_asc(workflow_shard::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(WorkflowShardSummary::from).collect())
    }

    pub async fn complete_workflow_shard(
        &self,
        shard_id: &str,
        input: CompleteWorkflowShardInput,
    ) -> Result<Option<CompleteWorkflowShardResult>, sea_orm::DbErr> {
        let status = normalize_terminal_status(&input.status)?;
        let Some(shard) = workflow_shard::Entity::find_by_id(shard_id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        if matches!(shard.status.as_str(), "succeeded" | "failed") {
            return Ok(Some(CompleteWorkflowShardResult {
                shard: WorkflowShardSummary::from(shard),
                node_completed: false,
                node_status: None,
                advance: None,
            }));
        }

        let now = now_rfc3339();
        let output = input
            .output
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?;
        let workflow_instance_id = shard.workflow_instance_id.clone();
        let workflow_node_instance_id = shard.workflow_node_instance_id.clone();
        let shard_job_instance_id = shard.job_instance_id.clone();
        let node_key = shard.node_key.clone();
        let shard_index = shard.shard_index;
        let txn = self.db.begin().await?;
        let mut active: workflow_shard::ActiveModel = shard.into();
        active.status = Set(status.clone());
        active.output = Set(output.clone());
        active.updated_at = Set(now.clone());
        let updated = active.update(&txn).await?;
        instance_event::ActiveModel {
            id: Set(new_id("evt")),
            instance_id: Set(workflow_instance_id.clone()),
            instance_type: Set("workflow".to_owned()),
            event_type: Set(format!("workflow.shard.{status}")),
            message: Set(input.message.unwrap_or_else(|| {
                format!("shard {node_key}#{shard_index} completed as {status}")
            })),
            payload: Set(output),
            created_at: Set(now.clone()),
        }
        .insert(&txn)
        .await?;

        let sibling_rows = workflow_shard::Entity::find()
            .filter(
                workflow_shard::Column::WorkflowNodeInstanceId
                    .eq(workflow_node_instance_id.clone()),
            )
            .all(&txn)
            .await?;
        let has_failed = sibling_rows.iter().any(|row| row.status == "failed");
        let all_succeeded = sibling_rows.iter().all(|row| row.status == "succeeded");
        txn.commit().await?;
        if let Some(job_instance_id) = &shard_job_instance_id {
            self.mark_job_queue_done(job_instance_id, &status).await?;
        }

        let node_status = if has_failed {
            Some("failed".to_owned())
        } else if all_succeeded {
            Some("succeeded".to_owned())
        } else {
            None
        };
        let advance = if let Some(node_status) = &node_status {
            self.advance_workflow(
                &workflow_instance_id,
                AdvanceWorkflowInput {
                    node_key,
                    status: node_status.clone(),
                    message: Some(format!(
                        "workflow shards completed with aggregate status {node_status}"
                    )),
                },
            )
            .await?
        } else {
            None
        };

        Ok(Some(CompleteWorkflowShardResult {
            shard: WorkflowShardSummary::from(updated),
            node_completed: node_status.is_some(),
            node_status,
            advance,
        }))
    }

    async fn propagate_child_workflow_completion(
        &self,
        child_instance: &WorkflowInstanceSummary,
    ) -> Result<(), sea_orm::DbErr> {
        let Some(parent_node) = workflow_node_instance::Entity::find()
            .filter(
                workflow_node_instance::Column::ChildWorkflowInstanceId
                    .eq(child_instance.id.clone()),
            )
            .one(&self.db)
            .await?
        else {
            return Ok(());
        };
        if matches!(
            parent_node.status.as_str(),
            "succeeded" | "failed" | "skipped"
        ) {
            return Ok(());
        }
        let parent_status = if child_instance.status == "succeeded" {
            "succeeded"
        } else {
            "failed"
        };
        let _ = Box::pin(self.advance_workflow(
            &parent_node.workflow_instance_id,
            AdvanceWorkflowInput {
                node_key: parent_node.node_key,
                status: parent_status.to_owned(),
                message: Some(format!(
                    "child workflow {} completed as {}",
                    child_instance.id, child_instance.status
                )),
            },
        ))
        .await?;
        Ok(())
    }

    pub async fn claim_next_dispatch_queue_item(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_next_dispatch_queue_item_with_fencing(lease_owner, lease_seconds, None)
            .await
    }

    pub async fn claim_next_dispatch_queue_item_with_fencing(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
        fencing_token: Option<&str>,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_next_dispatch_queue_item_matching(
            lease_owner,
            lease_seconds,
            DispatchQueueClaimKind::Any,
            fencing_token,
        )
        .await
    }

    pub async fn claim_next_workflow_node_queue_item(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_next_workflow_node_queue_item_with_fencing(
            lease_owner,
            lease_seconds,
            lease_owner,
        )
        .await
    }

    pub async fn claim_next_workflow_node_queue_item_with_fencing(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
        fencing_token: &str,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_next_dispatch_queue_item_matching(
            lease_owner,
            lease_seconds,
            DispatchQueueClaimKind::WorkflowNode,
            Some(fencing_token),
        )
        .await
    }

    pub async fn claim_next_job_queue_item(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_next_job_queue_item_with_fencing(lease_owner, lease_seconds, lease_owner)
            .await
    }

    pub async fn claim_next_job_queue_item_with_fencing(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
        fencing_token: &str,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_next_dispatch_queue_item_matching(
            lease_owner,
            lease_seconds,
            DispatchQueueClaimKind::JobInstance,
            Some(fencing_token),
        )
        .await
    }

    async fn claim_next_dispatch_queue_item_matching(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
        kind: DispatchQueueClaimKind,
        fencing_token: Option<&str>,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        let now = now_rfc3339();
        let mut query = dispatch_queue::Entity::find()
            .filter(dispatch_queue::Column::Status.eq("pending"))
            .filter(dispatch_queue::Column::RunAfter.lte(now.clone()))
            .filter(
                dispatch_queue::Column::LeaseUntil
                    .is_null()
                    .or(dispatch_queue::Column::LeaseUntil.lt(now.clone())),
            )
            .order_by_asc(dispatch_queue::Column::Priority)
            .order_by_asc(dispatch_queue::Column::RunAfter)
            .limit(1);
        query = match kind {
            DispatchQueueClaimKind::Any => query,
            DispatchQueueClaimKind::WorkflowNode => {
                query.filter(dispatch_queue::Column::WorkflowNodeInstanceId.is_not_null())
            }
            DispatchQueueClaimKind::JobInstance => {
                query.filter(dispatch_queue::Column::JobInstanceId.is_not_null())
            }
        };
        let Some((queue_id,)) = query
            .select_only()
            .column(dispatch_queue::Column::Id)
            .into_tuple::<(String,)>()
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        self.claim_dispatch_queue_item_with_fencing(
            &queue_id,
            lease_owner,
            lease_seconds,
            fencing_token,
        )
        .await
    }

    pub async fn claim_dispatch_queue_item(
        &self,
        queue_id: &str,
        lease_owner: &str,
        lease_seconds: i64,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_dispatch_queue_item_with_fencing(queue_id, lease_owner, lease_seconds, None)
            .await
    }

    pub async fn claim_dispatch_queue_item_with_fencing(
        &self,
        queue_id: &str,
        lease_owner: &str,
        lease_seconds: i64,
        fencing_token: Option<&str>,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        let now = now_rfc3339();
        let lease_until = rfc3339_after_seconds(lease_seconds.max(1));
        let fencing_token = fencing_token.map_or_else(
            || format!("lease:{lease_owner}:{queue_id}:{lease_until}"),
            ToOwned::to_owned,
        );
        let txn = self.db.begin().await?;
        let result = dispatch_queue::Entity::update_many()
            .col_expr(
                dispatch_queue::Column::LeaseOwner,
                Expr::value(Some(lease_owner.to_owned())),
            )
            .col_expr(
                dispatch_queue::Column::LeaseUntil,
                Expr::value(Some(lease_until.clone())),
            )
            .col_expr(
                dispatch_queue::Column::FencingToken,
                Expr::value(Some(fencing_token.clone())),
            )
            .col_expr(
                dispatch_queue::Column::Attempt,
                Expr::col(dispatch_queue::Column::Attempt).add(1),
            )
            .col_expr(dispatch_queue::Column::UpdatedAt, Expr::value(now.clone()))
            .filter(dispatch_queue::Column::Id.eq(queue_id.to_owned()))
            .filter(dispatch_queue::Column::Status.eq("pending"))
            .filter(
                dispatch_queue::Column::LeaseUntil
                    .is_null()
                    .or(dispatch_queue::Column::LeaseUntil.lt(now)),
            )
            .exec(&txn)
            .await?;
        if result.rows_affected == 0 {
            txn.commit().await?;
            return Ok(None);
        }
        let updated = dispatch_queue::Entity::find_by_id(queue_id.to_owned())
            .one(&txn)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(queue_id.to_owned()))?;
        txn.commit().await?;
        Ok(Some(DispatchQueueClaim {
            item: DispatchQueueSummary::from(updated),
            lease_owner: lease_owner.to_owned(),
            lease_until,
            fencing_token,
        }))
    }

    pub async fn mark_dispatch_queue_running(
        &self,
        queue_id: &str,
        lease_owner: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let result = dispatch_queue::Entity::update_many()
            .col_expr(dispatch_queue::Column::Status, Expr::value("running"))
            .col_expr(
                dispatch_queue::Column::LeaseOwner,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::LeaseUntil,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::FencingToken,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::UpdatedAt,
                Expr::value(now_rfc3339()),
            )
            .filter(dispatch_queue::Column::Id.eq(queue_id.to_owned()))
            .filter(dispatch_queue::Column::LeaseOwner.eq(lease_owner.to_owned()))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    pub async fn clear_expired_dispatch_queue_leases(&self) -> Result<u64, sea_orm::DbErr> {
        let now = now_rfc3339();
        let result = dispatch_queue::Entity::update_many()
            .col_expr(
                dispatch_queue::Column::LeaseOwner,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::LeaseUntil,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::FencingToken,
                Expr::value(Option::<String>::None),
            )
            .col_expr(dispatch_queue::Column::UpdatedAt, Expr::value(now.clone()))
            .filter(dispatch_queue::Column::Status.eq("pending"))
            .filter(dispatch_queue::Column::LeaseUntil.is_not_null())
            .filter(dispatch_queue::Column::LeaseUntil.lt(now))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected)
    }

    pub async fn release_dispatch_queue_item(
        &self,
        queue_id: &str,
        lease_owner: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let Some(row) = dispatch_queue::Entity::find_by_id(queue_id.to_owned())
            .one(&self.db)
            .await?
            .filter(|row| row.lease_owner.as_deref() == Some(lease_owner))
        else {
            return Ok(false);
        };
        let mut active: dispatch_queue::ActiveModel = row.into();
        active.lease_owner = Set(None);
        active.lease_until = Set(None);
        active.fencing_token = Set(None);
        active.updated_at = Set(now_rfc3339());
        active.update(&self.db).await?;
        Ok(true)
    }

    pub async fn dispatch_queue_slo_summary(
        &self,
    ) -> Result<DispatchQueueSloSummary, sea_orm::DbErr> {
        let rows = dispatch_queue::Entity::find().all(&self.db).await?;
        let now = time::OffsetDateTime::now_utc();
        let mut summary = DispatchQueueSloSummary::default();
        let mut pending_age_total = 0_u64;

        for row in rows {
            summary.total = summary.total.saturating_add(1);
            *summary.by_status.entry(row.status.clone()).or_insert(0) += 1;
            match row.status.as_str() {
                "pending" => {
                    summary.pending = summary.pending.saturating_add(1);
                    let age = dispatch_queue_age_seconds(&row.created_at, now);
                    pending_age_total = pending_age_total.saturating_add(age);
                    summary.oldest_pending_age_seconds =
                        summary.oldest_pending_age_seconds.max(age);
                }
                "running" => {
                    summary.running = summary.running.saturating_add(1);
                }
                _ => {}
            }
        }

        summary.average_pending_age_seconds =
            pending_age_total.checked_div(summary.pending).unwrap_or(0);

        Ok(summary)
    }

    pub async fn queue_overview(&self, limit: u64) -> Result<QueueOverview, sea_orm::DbErr> {
        let rows = dispatch_queue::Entity::find()
            .order_by_desc(dispatch_queue::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await?;
        let mut pending = 0;
        let mut running = 0;
        let mut done = 0;
        let mut failed = 0;
        for row in &rows {
            match row.status.as_str() {
                "pending" => pending += 1,
                "running" => running += 1,
                "done" => done += 1,
                "failed" => failed += 1,
                _ => {}
            }
        }
        Ok(QueueOverview {
            pending,
            running,
            done,
            failed,
            items: rows.into_iter().map(DispatchQueueSummary::from).collect(),
        })
    }

    pub async fn recover_workflow_node(
        &self,
        instance_id: &str,
        input: RecoverWorkflowNodeInput,
    ) -> Result<Option<RecoverWorkflowNodeResult>, sea_orm::DbErr> {
        let status = match input.action.as_str() {
            "retry" => "queued",
            "skip" => "skipped",
            "fail" => "failed",
            "succeed" => "succeeded",
            other => {
                return Err(sea_orm::DbErr::Custom(format!(
                    "unsupported recovery action: {other}"
                )));
            }
        };
        let Some(node) = workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::WorkflowInstanceId.eq(instance_id.to_owned()))
            .filter(workflow_node_instance::Column::NodeKey.eq(input.node_key.clone()))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let now = now_rfc3339();
        let txn = self.db.begin().await?;
        let mut node_active: workflow_node_instance::ActiveModel = node.into();
        node_active.status = Set(status.to_owned());
        node_active.updated_at = Set(now.clone());
        let updated = node_active.update(&txn).await?;
        if input.action == "retry" {
            dispatch_queue::ActiveModel {
                id: Set(new_id("dq")),
                job_instance_id: Set(None),
                workflow_node_instance_id: Set(Some(updated.id.clone())),
                priority: Set(0),
                run_after: Set(now.clone()),
                status: Set("pending".to_owned()),
                attempt: Set(0),
                lease_owner: Set(None),
                lease_until: Set(None),
                fencing_token: Set(None),
                worker_selector: Set(None),
                created_at: Set(now.clone()),
                updated_at: Set(now.clone()),
            }
            .insert(&txn)
            .await?;
        }
        instance_event::ActiveModel {
            id: Set(new_id("evt")),
            instance_id: Set(instance_id.to_owned()),
            instance_type: Set("workflow".to_owned()),
            event_type: Set(format!("workflow.node.recovery.{}", input.action)),
            message: Set(input
                .message
                .unwrap_or_else(|| format!("node {} {}", input.node_key, input.action))),
            payload: Set(None),
            created_at: Set(now),
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        let advance = if matches!(input.action.as_str(), "skip" | "fail" | "succeed") {
            self.advance_workflow(
                instance_id,
                AdvanceWorkflowInput {
                    node_key: input.node_key,
                    status: status.to_owned(),
                    message: None,
                },
            )
            .await?
        } else {
            None
        };
        let instance = self
            .get_workflow_instance(instance_id)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(instance_id.to_owned()))?;
        Ok(Some(RecoverWorkflowNodeResult {
            instance,
            queued_nodes: advance.map_or_else(Vec::new, |result| result.queued_nodes),
        }))
    }

    pub async fn get_workflow_instance(
        &self,
        id: &str,
    ) -> Result<Option<WorkflowInstanceSummary>, sea_orm::DbErr> {
        let Some(instance) = workflow_instance::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let nodes = workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::WorkflowInstanceId.eq(id.to_owned()))
            .order_by_asc(workflow_node_instance::Column::CreatedAt)
            .all(&self.db)
            .await?
            .into_iter()
            .map(WorkflowNodeInstanceSummary::from)
            .collect();
        Ok(Some(WorkflowInstanceSummary::from_model(instance, nodes)))
    }

    pub async fn list_instance_events(
        &self,
        instance_id: &str,
    ) -> Result<Vec<InstanceEventSummary>, sea_orm::DbErr> {
        let rows = instance_event::Entity::find()
            .filter(instance_event::Column::InstanceId.eq(instance_id.to_owned()))
            .order_by_asc(instance_event::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(InstanceEventSummary::from).collect())
    }
}

impl WorkflowSummary {
    fn from_model(model: workflow::Model) -> Result<Self, sea_orm::DbErr> {
        let definition = serde_json::from_str(&model.definition)
            .map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?;
        Ok(Self {
            id: model.id,
            name: model.name,
            definition,
            status: model.status,
            created_by: model.created_by,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}

impl WorkflowNodeInstanceSummary {
    fn from(model: workflow_node_instance::Model) -> Self {
        Self {
            id: model.id,
            workflow_instance_id: model.workflow_instance_id,
            node_key: model.node_key,
            status: model.status,
            job_instance_id: model.job_instance_id,
            child_workflow_instance_id: model.child_workflow_instance_id,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

impl WorkflowInstanceSummary {
    fn from_model(
        model: workflow_instance::Model,
        nodes: Vec<WorkflowNodeInstanceSummary>,
    ) -> Self {
        Self {
            id: model.id,
            workflow_id: model.workflow_id,
            status: model.status,
            trigger_type: model.trigger_type,
            nodes,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

async fn ensure_workflow_job_soft_link<C>(
    db: &C,
    job_id: &str,
    now: &str,
) -> Result<(), sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    if job::Entity::find_by_id(job_id.to_owned())
        .one(db)
        .await?
        .is_some()
    {
        return Ok(());
    }
    let namespace_id = format!("ns-{job_id}");
    let app_id = format!("app-{job_id}");
    namespace::ActiveModel {
        id: Set(namespace_id.clone()),
        name: Set(format!("workflow-{job_id}")),
        created_at: Set(now.to_owned()),
        updated_at: Set(now.to_owned()),
    }
    .insert(db)
    .await?;
    app::ActiveModel {
        id: Set(app_id.clone()),
        namespace_id: Set(namespace_id.clone()),
        name: Set("workflow".to_owned()),
        created_at: Set(now.to_owned()),
        updated_at: Set(now.to_owned()),
    }
    .insert(db)
    .await?;
    job::ActiveModel {
        id: Set(job_id.to_owned()),
        namespace_id: Set(namespace_id),
        app_id: Set(app_id),
        name: Set(format!("workflow node {job_id}")),
        schedule_type: Set("api".to_owned()),
        schedule_expr: Set(None),
        processor_name: Set(Some(job_id.to_owned())),
        enabled: Set(true),
        created_at: Set(now.to_owned()),
        updated_at: Set(now.to_owned()),
    }
    .insert(db)
    .await?;
    Ok(())
}

impl From<workflow_shard::Model> for WorkflowShardSummary {
    fn from(model: workflow_shard::Model) -> Self {
        Self {
            id: model.id,
            workflow_instance_id: model.workflow_instance_id,
            workflow_node_instance_id: model.workflow_node_instance_id,
            node_key: model.node_key,
            shard_index: model.shard_index,
            status: model.status,
            input: serde_json::from_str(&model.input).unwrap_or(serde_json::Value::Null),
            output: model
                .output
                .and_then(|value| serde_json::from_str(&value).ok()),
            job_instance_id: model.job_instance_id,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

impl From<dispatch_queue::Model> for DispatchQueueSummary {
    fn from(model: dispatch_queue::Model) -> Self {
        Self {
            id: model.id,
            job_instance_id: model.job_instance_id,
            workflow_node_instance_id: model.workflow_node_instance_id,
            priority: model.priority,
            run_after: model.run_after,
            status: model.status,
            attempt: model.attempt,
            lease_owner: model.lease_owner,
            lease_until: model.lease_until,
            fencing_token: model.fencing_token,
            worker_selector: model.worker_selector,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

fn dispatch_queue_age_seconds(created_at: &str, now: time::OffsetDateTime) -> u64 {
    time::OffsetDateTime::parse(created_at, &time::format_description::well_known::Rfc3339)
        .ok()
        .and_then(|created| (now - created).whole_seconds().try_into().ok())
        .unwrap_or(0)
}

fn normalize_terminal_status(status: &str) -> Result<String, sea_orm::DbErr> {
    match status {
        "succeeded" | "failed" => Ok(status.to_owned()),
        other => Err(sea_orm::DbErr::Custom(format!(
            "unsupported workflow shard status: {other}"
        ))),
    }
}

#[derive(Debug, Clone, Copy)]
enum DispatchQueueClaimKind {
    Any,
    WorkflowNode,
    JobInstance,
}

fn normalize_processor_name(value: Option<String>) -> Option<String> {
    value.and_then(|name| {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

fn node_kind(node: &WorkflowNodeSpec) -> &str {
    node.kind.as_deref().unwrap_or("job")
}

impl From<instance_event::Model> for InstanceEventSummary {
    fn from(model: instance_event::Model) -> Self {
        Self {
            id: model.id,
            instance_id: model.instance_id,
            instance_type: model.instance_type,
            event_type: model.event_type,
            message: model.message,
            payload: model.payload,
            created_at: model.created_at,
        }
    }
}

pub fn validate_workflow_definition(definition: &WorkflowDefinition) -> WorkflowValidationResult {
    let mut errors = Vec::new();
    if definition.nodes.is_empty() {
        errors.push("workflow must contain at least one node".to_owned());
    }
    let mut keys = HashSet::new();
    for node in &definition.nodes {
        if node.key.trim().is_empty() {
            errors.push("node key cannot be empty".to_owned());
        }
        if !keys.insert(node.key.clone()) {
            errors.push(format!("duplicate node key: {}", node.key));
        }
    }
    let allowed_kinds = [
        "start",
        "end",
        "job",
        "script",
        "http",
        "condition",
        "parallel",
        "join",
        "delay",
        "approval",
        "notification",
        "map",
        "map_reduce",
        "sub_workflow",
    ];
    for node in &definition.nodes {
        let kind = node.kind.as_deref().unwrap_or("job");
        if !allowed_kinds.contains(&kind) {
            errors.push(format!("unsupported node kind: {kind}"));
        }
        if kind == "job" && node.job_id.as_deref().unwrap_or("").is_empty() {
            errors.push(format!("job node {} requires job_id", node.key));
        }
        if kind == "condition" && node_config_string(node, "expression").is_none() {
            errors.push(format!(
                "condition node {} requires config.expression",
                node.key
            ));
        }
        if kind == "http" && node_config_string(node, "url").is_none() {
            errors.push(format!("http node {} requires config.url", node.key));
        }
        if kind == "script" && node_config_string(node, "source").is_none() {
            errors.push(format!("script node {} requires config.source", node.key));
        }
        if kind == "approval" && node_config_string(node, "approvers").is_none() {
            errors.push(format!(
                "approval node {} requires config.approvers",
                node.key
            ));
        }
        if kind == "sub_workflow" && node.child_workflow_id.as_deref().unwrap_or("").is_empty() {
            errors.push(format!(
                "sub_workflow node {} requires child_workflow_id",
                node.key
            ));
        }
        if (kind == "map" || kind == "map_reduce")
            && node.map_items.as_ref().is_none_or(Vec::is_empty)
        {
            errors.push(format!("{kind} node {} requires map_items", node.key));
        }
    }
    let allowed_conditions = ["always", "on_success", "on_failure"];
    for edge in &definition.edges {
        if !keys.contains(&edge.from) {
            errors.push(format!("edge references missing from node: {}", edge.from));
        }
        if !keys.contains(&edge.to) {
            errors.push(format!("edge references missing to node: {}", edge.to));
        }
        let condition = edge.condition.as_deref().unwrap_or("always");
        if !allowed_conditions.contains(&condition) {
            errors.push(format!("unsupported edge condition: {condition}"));
        }
    }
    if !definition.nodes.is_empty() && start_node_keys(definition).is_empty() {
        errors.push("workflow must contain at least one start node".to_owned());
    }
    if has_cycle(definition) {
        errors.push("workflow graph must be acyclic".to_owned());
    }
    WorkflowValidationResult {
        valid: errors.is_empty(),
        errors,
    }
}

fn node_config_string<'a>(node: &'a WorkflowNodeSpec, key: &str) -> Option<&'a str> {
    node.config
        .as_ref()
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn next_nodes_for_status(
    definition: &WorkflowDefinition,
    node_key: &str,
    status: &str,
) -> Vec<String> {
    definition
        .edges
        .iter()
        .filter(|edge| edge.from == node_key)
        .filter(|edge| {
            edge_condition_satisfied(status, edge.condition.as_deref().unwrap_or("always"))
        })
        .map(|edge| edge.to.clone())
        .collect()
}

async fn all_predecessors_satisfied<C>(
    definition: &WorkflowDefinition,
    node_key: &str,
    instance_id: &str,
    db: &C,
) -> Result<bool, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let incoming: Vec<&WorkflowEdgeSpec> = definition
        .edges
        .iter()
        .filter(|edge| edge.to == node_key)
        .collect();
    for edge in incoming {
        let predecessor = workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::WorkflowInstanceId.eq(instance_id.to_owned()))
            .filter(workflow_node_instance::Column::NodeKey.eq(edge.from.clone()))
            .one(db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(edge.from.clone()))?;
        if !edge_condition_satisfied(
            &predecessor.status,
            edge.condition.as_deref().unwrap_or("always"),
        ) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn edge_condition_satisfied(status: &str, condition: &str) -> bool {
    match condition {
        "always" => matches!(status, "succeeded" | "failed" | "skipped"),
        "on_failure" => status == "failed",
        _ => status == "succeeded",
    }
}

fn start_node_keys(definition: &WorkflowDefinition) -> HashSet<String> {
    let targets: HashSet<_> = definition
        .edges
        .iter()
        .map(|edge| edge.to.clone())
        .collect();
    definition
        .nodes
        .iter()
        .filter(|node| !targets.contains(&node.key))
        .map(|node| node.key.clone())
        .collect()
}

fn has_cycle(definition: &WorkflowDefinition) -> bool {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in &definition.nodes {
        graph.entry(&node.key).or_default();
    }
    for edge in &definition.edges {
        graph.entry(&edge.from).or_default().push(&edge.to);
    }
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    definition
        .nodes
        .iter()
        .any(|node| dfs_cycle(&node.key, &graph, &mut visiting, &mut visited))
}

fn dfs_cycle<'a>(
    node: &'a str,
    graph: &HashMap<&'a str, Vec<&'a str>>,
    visiting: &mut HashSet<&'a str>,
    visited: &mut HashSet<&'a str>,
) -> bool {
    if visited.contains(node) {
        return false;
    }
    if !visiting.insert(node) {
        return true;
    }
    if let Some(next) = graph.get(node) {
        for child in next {
            if dfs_cycle(child, graph, visiting, visited) {
                return true;
            }
        }
    }
    visiting.remove(node);
    visited.insert(node);
    false
}
