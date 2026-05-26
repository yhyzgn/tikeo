#![allow(missing_docs)]

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait, sea_query::Expr,
};
use tikee_core::InstanceStatus;

use crate::entities::{
    dispatch_queue, instance_event, workflow, workflow_edge, workflow_instance, workflow_node,
    workflow_node_instance, workflow_shard,
};

use super::util::{new_id, now_rfc3339, rfc3339_after_seconds};
mod types;

use conversions::{
    DispatchQueueClaimKind, dispatch_queue_age_seconds, elapsed_seconds,
    ensure_workflow_job_soft_link, node_kind, normalize_processor_name, normalize_terminal_status,
    success_ratio,
};
pub use types::*;
use validation::{all_predecessors_satisfied, next_nodes_for_status, start_node_keys};

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

    pub async fn mark_dispatch_queue_done_by_instance(
        &self,
        instance_id: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let result = dispatch_queue::Entity::update_many()
            .col_expr(dispatch_queue::Column::Status, Expr::value("done"))
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
            .filter(dispatch_queue::Column::JobInstanceId.eq(instance_id.to_owned()))
            .filter(dispatch_queue::Column::Status.is_in(["pending", "running"]))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    pub async fn mark_dispatch_queue_failed(
        &self,
        queue_id: &str,
        lease_owner: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let result = dispatch_queue::Entity::update_many()
            .col_expr(dispatch_queue::Column::Status, Expr::value("failed"))
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

    pub async fn release_dispatch_queue_item(
        &self,
        queue_id: &str,
        lease_owner: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        self.release_dispatch_queue_item_after(queue_id, lease_owner, 0)
            .await
    }

    pub async fn release_dispatch_queue_item_after(
        &self,
        queue_id: &str,
        lease_owner: &str,
        delay_seconds: i64,
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
        active.run_after = Set(if delay_seconds > 0 {
            rfc3339_after_seconds(delay_seconds)
        } else {
            now_rfc3339()
        });
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
        let mut dispatch_latency_total = 0_u64;

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
                "done" | "failed" => {
                    summary.completed_dispatches = summary.completed_dispatches.saturating_add(1);
                    let latency = elapsed_seconds(&row.created_at, &row.updated_at);
                    dispatch_latency_total = dispatch_latency_total.saturating_add(latency);
                    summary.longest_dispatch_latency_seconds =
                        summary.longest_dispatch_latency_seconds.max(latency);
                }
                _ => {}
            }
        }

        summary.average_pending_age_seconds =
            pending_age_total.checked_div(summary.pending).unwrap_or(0);
        summary.average_dispatch_latency_seconds = dispatch_latency_total
            .checked_div(summary.completed_dispatches)
            .unwrap_or(0);

        Ok(summary)
    }

    pub async fn workflow_slo_summary(&self) -> Result<WorkflowSloSummary, sea_orm::DbErr> {
        let instances = workflow_instance::Entity::find().all(&self.db).await?;
        let shards = workflow_shard::Entity::find().all(&self.db).await?;
        let mut summary = WorkflowSloSummary::default();
        let mut instance_successes = 0_u64;
        let mut instance_failures = 0_u64;
        let mut instance_duration_total = 0_u64;
        let mut shard_successes = 0_u64;
        let mut shard_failures = 0_u64;
        let mut shard_duration_total = 0_u64;

        for row in instances {
            summary.instances_total = summary.instances_total.saturating_add(1);
            *summary
                .instances_by_status
                .entry(row.status.clone())
                .or_insert(0) += 1;
            match row.status.as_str() {
                "succeeded" => {
                    instance_successes = instance_successes.saturating_add(1);
                    summary.completed_instances = summary.completed_instances.saturating_add(1);
                    let duration = elapsed_seconds(&row.created_at, &row.updated_at);
                    instance_duration_total = instance_duration_total.saturating_add(duration);
                    summary.longest_instance_duration_seconds =
                        summary.longest_instance_duration_seconds.max(duration);
                }
                "failed" => {
                    instance_failures = instance_failures.saturating_add(1);
                    summary.completed_instances = summary.completed_instances.saturating_add(1);
                    let duration = elapsed_seconds(&row.created_at, &row.updated_at);
                    instance_duration_total = instance_duration_total.saturating_add(duration);
                    summary.longest_instance_duration_seconds =
                        summary.longest_instance_duration_seconds.max(duration);
                }
                _ => {}
            }
        }
        summary.average_instance_duration_seconds = instance_duration_total
            .checked_div(summary.completed_instances)
            .unwrap_or(0);
        summary.instance_success_ratio = success_ratio(instance_successes, instance_failures);

        for row in shards {
            summary.shards_total = summary.shards_total.saturating_add(1);
            *summary
                .shards_by_status
                .entry(row.status.clone())
                .or_insert(0) += 1;
            match row.status.as_str() {
                "succeeded" => {
                    shard_successes = shard_successes.saturating_add(1);
                    summary.completed_shards = summary.completed_shards.saturating_add(1);
                    let duration = elapsed_seconds(&row.created_at, &row.updated_at);
                    shard_duration_total = shard_duration_total.saturating_add(duration);
                    summary.longest_shard_duration_seconds =
                        summary.longest_shard_duration_seconds.max(duration);
                }
                "failed" => {
                    shard_failures = shard_failures.saturating_add(1);
                    summary.completed_shards = summary.completed_shards.saturating_add(1);
                    let duration = elapsed_seconds(&row.created_at, &row.updated_at);
                    shard_duration_total = shard_duration_total.saturating_add(duration);
                    summary.longest_shard_duration_seconds =
                        summary.longest_shard_duration_seconds.max(duration);
                }
                _ => {}
            }
        }
        summary.average_shard_duration_seconds = shard_duration_total
            .checked_div(summary.completed_shards)
            .unwrap_or(0);
        summary.shard_success_ratio = success_ratio(shard_successes, shard_failures);

        Ok(summary)
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
}

mod conversions;
mod events;
mod queue;
mod validation;

pub use validation::validate_workflow_definition;
