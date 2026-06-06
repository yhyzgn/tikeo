#![allow(missing_docs)]

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait, sea_query::Expr,
};
use tikee_core::InstanceStatus;

use crate::entities::{
    app as app_entity, dispatch_queue, instance_event, job_instance, namespace as namespace_entity,
    workflow, workflow_edge, workflow_instance, workflow_node, workflow_node_instance,
    workflow_shard,
};

use super::util::{new_id, now_rfc3339, rfc3339_after_seconds};
mod types;

use conversions::{
    DispatchQueueClaimKind, dispatch_queue_age_seconds, elapsed_seconds,
    ensure_workflow_job_soft_link, node_kind, normalize_processor_name, normalize_terminal_status,
    success_ratio,
};
pub use types::*;
use validation::{
    all_predecessors_satisfied, next_nodes_for_status, start_node_keys, workflow_config_bool,
    workflow_config_i64, workflow_config_string,
};

fn evaluate_condition_node(node: &WorkflowNodeSpec) -> bool {
    if let Some(value) = workflow_config_bool(node, "result") {
        return value;
    }
    if let Some(value) = workflow_config_bool(node, "value") {
        return value;
    }
    workflow_config_string(node, "expression")
        .map(str::trim)
        .is_some_and(|expression| evaluate_safe_condition_expression(node, expression))
}

fn evaluate_safe_condition_expression(node: &WorkflowNodeSpec, expression: &str) -> bool {
    let expression = expression.trim();
    if expression.is_empty() {
        return false;
    }
    expression.split("||").any(|branch| {
        branch
            .split("&&")
            .all(|atom| evaluate_condition_atom(node, atom.trim()))
    })
}

fn evaluate_condition_atom(node: &WorkflowNodeSpec, atom: &str) -> bool {
    if atom.is_empty() {
        return false;
    }
    match atom.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "success" | "succeeded" => return true,
        "false" | "0" | "no" | "failure" | "failed" => return false,
        _ => {}
    }
    for operator in [">=", "<=", "==", "!=", ">", "<"] {
        if let Some((left, right)) = atom.split_once(operator) {
            let Some(left) = condition_value(node, left.trim()) else {
                return false;
            };
            let right = parse_condition_literal(right.trim());
            return compare_condition_values(&left, &right, operator);
        }
    }
    condition_value(node, atom)
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn condition_value(node: &WorkflowNodeSpec, path: &str) -> Option<serde_json::Value> {
    let mut path = path.trim();
    path = path.strip_prefix("context.").unwrap_or(path);
    path = path.strip_prefix("config.").unwrap_or(path);
    path = path.strip_prefix("vars.").unwrap_or(path);
    let config = node.config.as_ref()?;
    let root = config.get("vars").unwrap_or(config);
    let mut current = root;
    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }
        current = current.get(segment)?;
    }
    Some(current.clone())
}

fn parse_condition_literal(value: &str) -> serde_json::Value {
    let value = value.trim();
    if let Some(stripped) = value
        .strip_prefix('"')
        .and_then(|item| item.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|item| item.strip_suffix('\''))
        })
    {
        return serde_json::Value::String(stripped.to_owned());
    }
    match value.to_ascii_lowercase().as_str() {
        "true" => serde_json::Value::Bool(true),
        "false" => serde_json::Value::Bool(false),
        "null" => serde_json::Value::Null,
        _ => value
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map_or_else(
                || serde_json::Value::String(value.to_owned()),
                serde_json::Value::Number,
            ),
    }
}

fn compare_condition_values(
    left: &serde_json::Value,
    right: &serde_json::Value,
    operator: &str,
) -> bool {
    if let (Some(left), Some(right)) = (left.as_f64(), right.as_f64()) {
        return match operator {
            "==" => (left - right).abs() < f64::EPSILON,
            "!=" => (left - right).abs() >= f64::EPSILON,
            ">" => left > right,
            ">=" => left >= right,
            "<" => left < right,
            "<=" => left <= right,
            _ => false,
        };
    }
    if matches!(operator, "==" | "!=") {
        return if operator == "==" {
            left == right
        } else {
            left != right
        };
    }
    let Some(left) = left.as_str() else {
        return false;
    };
    let Some(right) = right.as_str() else {
        return false;
    };
    match operator {
        ">" => left > right,
        ">=" => left >= right,
        "<" => left < right,
        "<=" => left <= right,
        _ => false,
    }
}

fn workflow_node_run_after(definition: &WorkflowDefinition, node_key: &str, now: &str) -> String {
    definition
        .nodes
        .iter()
        .find(|node| node.key == node_key && node_kind(node) == "delay")
        .and_then(|node| workflow_config_i64(node, "seconds"))
        .filter(|seconds| *seconds > 0)
        .map_or_else(|| now.to_owned(), rfc3339_after_seconds)
}

fn json_string(value: Option<&serde_json::Value>) -> Result<Option<String>, sea_orm::DbErr> {
    value
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| sea_orm::DbErr::Custom(error.to_string()))
}

async fn update_shard_terminal(
    txn: &DatabaseTransaction,
    shard: workflow_shard::Model,
    status: &str,
    output: Option<String>,
    checkpoint: Option<String>,
    now: &str,
) -> Result<workflow_shard::Model, sea_orm::DbErr> {
    let mut active: workflow_shard::ActiveModel = shard.into();
    active.status = Set(status.to_owned());
    active.output = Set(output);
    if checkpoint.is_some() {
        active.checkpoint = Set(checkpoint);
    }
    active.updated_at = Set(now.to_owned());
    active.update(txn).await
}

async fn insert_shard_completion_event(
    txn: &DatabaseTransaction,
    input: ShardCompletionEventInput,
) -> Result<(), sea_orm::DbErr> {
    instance_event::ActiveModel {
        id: Set(new_id("evt")),
        instance_id: Set(input.workflow_instance_id),
        instance_type: Set("workflow".to_owned()),
        event_type: Set(format!("workflow.shard.{}", input.status)),
        message: Set(input.message.unwrap_or_else(|| {
            format!(
                "shard {}#{} completed as {}",
                input.node_key, input.shard_index, input.status
            )
        })),
        payload: Set(input.output),
        created_at: Set(input.now),
    }
    .insert(txn)
    .await?;
    Ok(())
}

async fn maybe_persist_map_reduce_result(
    txn: &DatabaseTransaction,
    workflow_instance_id: &str,
    workflow_node_instance_id: &str,
    sibling_rows: &[workflow_shard::Model],
    now: &str,
) -> Result<(), sea_orm::DbErr> {
    if !sibling_rows.iter().all(|row| row.status == "succeeded") {
        return Ok(());
    }
    let Some(node_row) = workflow_node_instance::Entity::find_by_id(workflow_node_instance_id)
        .one(txn)
        .await?
    else {
        return Ok(());
    };
    let Some(workflow_row) = workflow_instance::Entity::find_by_id(workflow_instance_id)
        .one(txn)
        .await?
    else {
        return Ok(());
    };
    let definition_row = workflow_node::Entity::find()
        .filter(workflow_node::Column::WorkflowId.eq(workflow_row.workflow_id))
        .filter(workflow_node::Column::NodeKey.eq(node_row.node_key.clone()))
        .one(txn)
        .await?;
    if definition_row
        .as_ref()
        .is_some_and(|node| node.kind == "map_reduce")
    {
        persist_map_reduce_result_chunks(
            txn,
            workflow_instance_id,
            &node_row.node_key,
            sibling_rows,
            now,
        )
        .await?;
    }
    Ok(())
}

fn aggregate_shard_node_status(has_failed: bool, all_succeeded: bool) -> Option<String> {
    if has_failed {
        Some("failed".to_owned())
    } else if all_succeeded {
        Some("succeeded".to_owned())
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowRepository {
    db: DatabaseConnection,
}

async fn persist_map_reduce_result_chunks(
    txn: &DatabaseTransaction,
    workflow_instance_id: &str,
    node_key: &str,
    shards: &[workflow_shard::Model],
    now: &str,
) -> Result<(), sea_orm::DbErr> {
    let mut outputs = Vec::new();
    for shard in shards {
        outputs.push(serde_json::json!({
            "shardIndex": shard.shard_index,
            "output": shard.output.as_ref().and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok()),
            "checkpoint": shard.checkpoint.as_ref().and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok()),
        }));
    }
    outputs.sort_by_key(|value| {
        value
            .get("shardIndex")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or_default()
    });
    let chunk_size = 2_usize;
    let mut chunk_ids = Vec::new();
    for (chunk_index, chunk) in outputs.chunks(chunk_size).enumerate() {
        let event_id = new_id("evt");
        chunk_ids.push(event_id.clone());
        instance_event::ActiveModel {
            id: Set(event_id),
            instance_id: Set(workflow_instance_id.to_owned()),
            instance_type: Set("workflow".to_owned()),
            event_type: Set("workflow.map_reduce.chunk".to_owned()),
            message: Set(format!("map_reduce {node_key} result chunk {chunk_index}")),
            payload: Set(Some(
                serde_json::to_string(&serde_json::json!({
                    "nodeKey": node_key,
                    "chunkIndex": chunk_index,
                    "items": chunk,
                }))
                .map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?,
            )),
            created_at: Set(now.to_owned()),
        }
        .insert(txn)
        .await?;
    }
    instance_event::ActiveModel {
        id: Set(new_id("evt")),
        instance_id: Set(workflow_instance_id.to_owned()),
        instance_type: Set("workflow".to_owned()),
        event_type: Set("workflow.map_reduce.manifest".to_owned()),
        message: Set(format!(
            "map_reduce {node_key} reduced {} shards into {} chunks",
            outputs.len(),
            chunk_ids.len()
        )),
        payload: Set(Some(
            serde_json::to_string(&serde_json::json!({
                "nodeKey": node_key,
                "totalShards": outputs.len(),
                "chunkSize": chunk_size,
                "chunkEventIds": chunk_ids,
                "spilled": true,
            }))
            .map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?,
        )),
        created_at: Set(now.to_owned()),
    }
    .insert(txn)
    .await?;
    Ok(())
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
                    run_after: Set(workflow_node_run_after(
                        &workflow.definition,
                        &node.key,
                        &now,
                    )),
                    status: Set("pending".to_owned()),
                    attempt: Set(0),
                    lease_owner: Set(None),
                    lease_until: Set(None),
                    fencing_token: Set(None),
                    worker_selector: Set(None),
                    namespace: Set(None),
                    app: Set(None),
                    worker_pool: Set(None),
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
                        run_after: Set(workflow_node_run_after(
                            &workflow.definition,
                            &node_key,
                            &now,
                        )),
                        status: Set("pending".to_owned()),
                        attempt: Set(0),
                        lease_owner: Set(None),
                        lease_until: Set(None),
                        fencing_token: Set(None),
                        worker_selector: Set(None),
                        namespace: Set(None),
                        app: Set(None),
                        worker_pool: Set(None),
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
        let _expired_approval_nodes = self.expire_timed_out_approval_nodes().await?;
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
                    result_worker_id: Set(None),
                    result_success: Set(None),
                    result_message: Set(None),
                    result_completed_at: Set(None),
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
                    namespace: Set(None),
                    app: Set(None),
                    worker_pool: Set(None),
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
                    .clone()
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
                        checkpoint: Set(None),
                        retry_count: Set(0),
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
                        result_worker_id: Set(None),
                        result_success: Set(None),
                        result_message: Set(None),
                        result_completed_at: Set(None),
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
                        namespace: Set(None),
                        app: Set(None),
                        worker_pool: Set(None),
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
                                run_after: Set(workflow_node_run_after(
                                    &child_workflow.definition,
                                    &child_node.key,
                                    &now,
                                )),
                                status: Set("pending".to_owned()),
                                attempt: Set(0),
                                lease_owner: Set(None),
                                lease_until: Set(None),
                                fencing_token: Set(None),
                                worker_selector: Set(None),
                                namespace: Set(None),
                                app: Set(None),
                                worker_pool: Set(None),
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
            "script" => {
                let job_instance_id = new_id("inst");
                let job_id = node_spec.job_id.clone().unwrap_or_else(|| {
                    format!("workflow-script-{}-{}", instance.workflow_id, node_spec.key)
                });
                ensure_workflow_job_soft_link(&txn, &job_id, &now).await?;
                crate::entities::job_instance::ActiveModel {
                    id: Set(job_instance_id.clone()),
                    job_id: Set(job_id),
                    status: Set("pending".to_owned()),
                    trigger_type: Set("workflow".to_owned()),
                    execution_mode: Set("single".to_owned()),
                    result_worker_id: Set(None),
                    result_success: Set(None),
                    result_message: Set(None),
                    result_completed_at: Set(None),
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
                    namespace: Set(None),
                    app: Set(None),
                    worker_pool: Set(None),
                    created_at: Set(now.clone()),
                    updated_at: Set(now.clone()),
                }
                .insert(&txn)
                .await?;
            }
            "http" | "grpc" | "sql" | "file_cleanup" => {
                let job_instance_id = new_id("inst");
                let job_id = format!(
                    "workflow-{}-{}-{}",
                    node_kind(&node_spec),
                    instance.workflow_id,
                    node_spec.key
                );
                ensure_workflow_job_soft_link(&txn, &job_id, &now).await?;
                crate::entities::job_instance::ActiveModel {
                    id: Set(job_instance_id.clone()),
                    job_id: Set(job_id),
                    status: Set("pending".to_owned()),
                    trigger_type: Set("workflow".to_owned()),
                    execution_mode: Set("single".to_owned()),
                    result_worker_id: Set(None),
                    result_success: Set(None),
                    result_message: Set(None),
                    result_completed_at: Set(None),
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
                    namespace: Set(None),
                    app: Set(None),
                    worker_pool: Set(None),
                    created_at: Set(now.clone()),
                    updated_at: Set(now.clone()),
                }
                .insert(&txn)
                .await?;
            }
            "condition" => {
                node_active.status = Set(if evaluate_condition_node(&node_spec) {
                    "succeeded".to_owned()
                } else {
                    "failed".to_owned()
                });
            }
            "approval" => {
                node_active.status = Set(
                    if workflow_config_bool(&node_spec, "approved").unwrap_or(false) {
                        "succeeded".to_owned()
                    } else {
                        "running".to_owned()
                    },
                );
            }
            "delay" | "parallel" | "join" | "notification" | "compensation" | "start" | "end" => {
                node_active.status = Set("succeeded".to_owned());
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
            created_at: Set(now.clone()),
        }
        .insert(&txn)
        .await?;
        let updated_node_status = updated_node.status.clone();
        let updated_node_key = updated_node.node_key.clone();
        let updated_node_summary = WorkflowNodeInstanceSummary::from(updated_node);
        let updated_queue_summary = DispatchQueueSummary::from(updated_queue);
        let should_auto_advance = matches!(
            node_kind(&node_spec),
            "condition"
                | "parallel"
                | "join"
                | "notification"
                | "compensation"
                | "start"
                | "end"
                | "delay"
        ) && matches!(
            updated_node_status.as_str(),
            "succeeded" | "failed" | "skipped"
        );
        txn.commit().await?;
        if should_auto_advance {
            let _ = Box::pin(self.advance_workflow(
                &instance.id,
                AdvanceWorkflowInput {
                    node_key: updated_node_key,
                    status: updated_node_status,
                    message: Some("workflow control node completed".to_owned()),
                },
            ))
            .await?;
        }
        let refreshed = self
            .get_workflow_instance(&instance.id)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(instance.id.clone()))?;
        Ok(Some(MaterializeWorkflowNodeResult {
            instance: refreshed,
            node: updated_node_summary,
            shards,
            queue_item: updated_queue_summary,
        }))
    }

    pub async fn expire_timed_out_approval_nodes(&self) -> Result<u64, sea_orm::DbErr> {
        let now = now_rfc3339();
        let running = workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::Status.eq("running"))
            .all(&self.db)
            .await?;
        let mut expired = 0_u64;
        for node in running {
            let Some(instance) =
                workflow_instance::Entity::find_by_id(node.workflow_instance_id.clone())
                    .one(&self.db)
                    .await?
            else {
                continue;
            };
            let Some(workflow) = self.get_workflow(&instance.workflow_id).await? else {
                continue;
            };
            let Some(node_spec) = workflow
                .definition
                .nodes
                .iter()
                .find(|candidate| {
                    candidate.key == node.node_key && node_kind(candidate) == "approval"
                })
                .cloned()
            else {
                continue;
            };
            let timeout_seconds = workflow_config_i64(&node_spec, "timeoutSeconds")
                .or_else(|| workflow_config_i64(&node_spec, "timeout_seconds"))
                .filter(|value| *value >= 0)
                .unwrap_or(0);
            if timeout_seconds == 0
                || elapsed_seconds(&node.updated_at, &now) < timeout_seconds.cast_unsigned()
            {
                continue;
            }
            let status = workflow_config_string(&node_spec, "onTimeout")
                .or_else(|| workflow_config_string(&node_spec, "on_timeout"))
                .filter(|value| matches!(*value, "succeeded" | "failed" | "skipped"))
                .unwrap_or("failed")
                .to_owned();
            if self
                .advance_workflow(
                    &instance.id,
                    AdvanceWorkflowInput {
                        node_key: node.node_key.clone(),
                        status,
                        message: Some("approval SLA timed out".to_owned()),
                    },
                )
                .await?
                .is_some()
            {
                expired = expired.saturating_add(1);
            }
        }
        Ok(expired)
    }

    pub async fn job_binding_for_instance(
        &self,
        job_instance_id: &str,
    ) -> Result<Option<WorkflowJobBindingSummary>, sea_orm::DbErr> {
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
        {
            return Ok(Some(WorkflowJobBindingSummary {
                node_kind: node_kind(node).to_owned(),
                processor_name: normalize_processor_name(node.processor_name.clone()),
                config: node.config.clone(),
            }));
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
        {
            return Ok(Some(WorkflowJobBindingSummary {
                node_kind: node_kind(node).to_owned(),
                processor_name: normalize_processor_name(node.processor_name.clone()),
                config: node.config.clone(),
            }));
        }

        Ok(None)
    }

    pub async fn processor_name_for_job_instance(
        &self,
        job_instance_id: &str,
    ) -> Result<Option<String>, sea_orm::DbErr> {
        Ok(self
            .job_binding_for_instance(job_instance_id)
            .await?
            .and_then(|binding| binding.processor_name))
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
                    checkpoint: None,
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

        let context = self.persist_completed_shard(shard, input, &status).await?;
        if let Some(job_instance_id) = &context.job_instance_id {
            self.mark_job_queue_done(job_instance_id, &status).await?;
        }

        let node_status = aggregate_shard_node_status(context.has_failed, context.all_succeeded);
        let advance = if let Some(node_status) = &node_status {
            self.advance_workflow(
                &context.workflow_instance_id,
                AdvanceWorkflowInput {
                    node_key: context.node_key,
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
            shard: context.updated,
            node_completed: node_status.is_some(),
            node_status,
            advance,
        }))
    }

    async fn persist_completed_shard(
        &self,
        shard: workflow_shard::Model,
        input: CompleteWorkflowShardInput,
        status: &str,
    ) -> Result<CompletedShardContext, sea_orm::DbErr> {
        let now = now_rfc3339();
        let output = json_string(input.output.as_ref())?;
        let checkpoint = json_string(input.checkpoint.as_ref())?;
        let workflow_instance_id = shard.workflow_instance_id.clone();
        let workflow_node_instance_id = shard.workflow_node_instance_id.clone();
        let job_instance_id = shard.job_instance_id.clone();
        let node_key = shard.node_key.clone();
        let shard_index = shard.shard_index;
        let txn = self.db.begin().await?;
        let updated =
            update_shard_terminal(&txn, shard, status, output.clone(), checkpoint, &now).await?;
        insert_shard_completion_event(
            &txn,
            ShardCompletionEventInput {
                workflow_instance_id: workflow_instance_id.clone(),
                node_key: node_key.clone(),
                shard_index,
                status: status.to_owned(),
                message: input.message,
                output,
                now: now.clone(),
            },
        )
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
        maybe_persist_map_reduce_result(
            &txn,
            &workflow_instance_id,
            &workflow_node_instance_id,
            &sibling_rows,
            &now,
        )
        .await?;
        txn.commit().await?;
        Ok(CompletedShardContext {
            workflow_instance_id,
            job_instance_id,
            node_key,
            updated: WorkflowShardSummary::from(updated),
            has_failed,
            all_succeeded,
        })
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
            .order_by_asc(dispatch_queue::Column::RunAfter);
        query = match kind {
            DispatchQueueClaimKind::Any => query,
            DispatchQueueClaimKind::WorkflowNode => {
                query.filter(dispatch_queue::Column::WorkflowNodeInstanceId.is_not_null())
            }
            DispatchQueueClaimKind::JobInstance => {
                query.filter(dispatch_queue::Column::JobInstanceId.is_not_null())
            }
        };
        let candidates = query
            .select_only()
            .column(dispatch_queue::Column::Id)
            .into_tuple::<(String,)>()
            .all(&self.db)
            .await?;
        let mut queue_id = None;
        for (candidate_id,) in candidates {
            if kind == DispatchQueueClaimKind::JobInstance
                && self
                    .dispatch_queue_item_blocked_by_quota(&candidate_id)
                    .await?
            {
                continue;
            }
            queue_id = Some(candidate_id);
            break;
        }
        let Some(queue_id) = queue_id else {
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

    async fn dispatch_queue_item_blocked_by_quota(
        &self,
        queue_id: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let Some(row) = dispatch_queue::Entity::find_by_id(queue_id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let (Some(namespace), Some(app), Some(pool)) = (row.namespace, row.app, row.worker_pool)
        else {
            return Ok(false);
        };
        let Some(namespace_model) = namespace_entity::Entity::find()
            .filter(namespace_entity::Column::Name.eq(namespace.clone()))
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let Some(app_model) = app_entity::Entity::find()
            .filter(app_entity::Column::NamespaceId.eq(namespace_model.id.clone()))
            .filter(app_entity::Column::Name.eq(app.clone()))
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let Some(scope) = crate::entities::worker_pool::Entity::find()
            .filter(crate::entities::worker_pool::Column::NamespaceId.eq(namespace_model.id))
            .filter(crate::entities::worker_pool::Column::AppId.eq(app_model.id))
            .filter(crate::entities::worker_pool::Column::Name.eq(pool.clone()))
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let active_depth = dispatch_queue::Entity::find()
            .filter(dispatch_queue::Column::Namespace.eq(namespace.clone()))
            .filter(dispatch_queue::Column::App.eq(app.clone()))
            .filter(dispatch_queue::Column::WorkerPool.eq(pool.clone()))
            .filter(dispatch_queue::Column::Status.is_in(["pending", "running"]))
            .all(&self.db)
            .await?
            .len();
        if scope.max_queue_depth > 0
            && active_depth > usize::try_from(scope.max_queue_depth).unwrap_or(usize::MAX)
        {
            return Ok(true);
        }
        let running_depth = dispatch_queue::Entity::find()
            .filter(dispatch_queue::Column::Namespace.eq(namespace))
            .filter(dispatch_queue::Column::App.eq(app))
            .filter(dispatch_queue::Column::WorkerPool.eq(pool))
            .filter(dispatch_queue::Column::Status.eq("running"))
            .all(&self.db)
            .await?
            .len();
        Ok(scope.max_concurrency > 0
            && running_depth >= usize::try_from(scope.max_concurrency).unwrap_or(usize::MAX))
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

    pub async fn requeue_stale_running_job_dispatches(
        &self,
        stale_after_seconds: i64,
    ) -> Result<u64, sea_orm::DbErr> {
        let cutoff = rfc3339_after_seconds(-stale_after_seconds.max(1));
        let now = now_rfc3339();
        let txn = self.db.begin().await?;
        let instance_ids = dispatch_queue::Entity::find()
            .select_only()
            .column(dispatch_queue::Column::JobInstanceId)
            .filter(dispatch_queue::Column::Status.eq("running"))
            .filter(dispatch_queue::Column::JobInstanceId.is_not_null())
            .filter(dispatch_queue::Column::UpdatedAt.lt(cutoff))
            .into_tuple::<(Option<String>,)>()
            .all(&txn)
            .await?
            .into_iter()
            .filter_map(|(id,)| id)
            .collect::<Vec<_>>();
        if instance_ids.is_empty() {
            txn.commit().await?;
            return Ok(0);
        }
        let queue_result = dispatch_queue::Entity::update_many()
            .col_expr(dispatch_queue::Column::Status, Expr::value("pending"))
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
            .filter(dispatch_queue::Column::JobInstanceId.is_in(instance_ids.clone()))
            .filter(dispatch_queue::Column::Status.eq("running"))
            .exec(&txn)
            .await?;
        job_instance::Entity::update_many()
            .col_expr(
                job_instance::Column::Status,
                Expr::value(InstanceStatus::Pending.to_string()),
            )
            .col_expr(job_instance::Column::UpdatedAt, Expr::value(now))
            .filter(job_instance::Column::Id.is_in(instance_ids))
            .filter(job_instance::Column::Status.eq(InstanceStatus::Running.to_string()))
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(queue_result.rows_affected)
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

    pub async fn dispatch_queue_for_instance(
        &self,
        instance_id: &str,
    ) -> Result<Option<DispatchQueueSummary>, sea_orm::DbErr> {
        let row = dispatch_queue::Entity::find()
            .filter(dispatch_queue::Column::JobInstanceId.eq(instance_id.to_owned()))
            .one(&self.db)
            .await?;
        Ok(row.map(DispatchQueueSummary::from))
    }

    pub async fn requeue_dispatch_queue_for_retry(
        &self,
        instance_id: &str,
        delay_seconds: i64,
    ) -> Result<Option<DispatchQueueSummary>, sea_orm::DbErr> {
        let Some(row) = dispatch_queue::Entity::find()
            .filter(dispatch_queue::Column::JobInstanceId.eq(instance_id.to_owned()))
            .filter(dispatch_queue::Column::Status.is_in(["pending", "running"]))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let run_after = if delay_seconds > 0 {
            rfc3339_after_seconds(delay_seconds)
        } else {
            now_rfc3339()
        };
        let now = now_rfc3339();
        let mut active: dispatch_queue::ActiveModel = row.into();
        active.status = Set("pending".to_owned());
        active.run_after = Set(run_after);
        active.lease_owner = Set(None);
        active.lease_until = Set(None);
        active.fencing_token = Set(None);
        active.updated_at = Set(now.clone());
        let updated = active.update(&self.db).await?;
        job_instance::Entity::update_many()
            .col_expr(
                job_instance::Column::Status,
                Expr::value(InstanceStatus::Pending.to_string()),
            )
            .col_expr(job_instance::Column::UpdatedAt, Expr::value(now))
            .filter(job_instance::Column::Id.eq(instance_id.to_owned()))
            .filter(job_instance::Column::Status.is_in([
                InstanceStatus::Running.to_string(),
                InstanceStatus::Dispatching.to_string(),
                InstanceStatus::Failed.to_string(),
            ]))
            .exec(&self.db)
            .await?;
        Ok(Some(DispatchQueueSummary::from(updated)))
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

    pub async fn cancel_job_instance(&self, job_instance_id: &str) -> Result<bool, sea_orm::DbErr> {
        let now = now_rfc3339();
        let txn = self.db.begin().await?;
        let instance_result = crate::entities::job_instance::Entity::update_many()
            .col_expr(
                crate::entities::job_instance::Column::Status,
                Expr::value("cancelled"),
            )
            .col_expr(
                crate::entities::job_instance::Column::UpdatedAt,
                Expr::value(now.clone()),
            )
            .filter(crate::entities::job_instance::Column::Id.eq(job_instance_id.to_owned()))
            .filter(crate::entities::job_instance::Column::Status.is_in([
                "pending",
                "dispatching",
                "running",
            ]))
            .exec(&txn)
            .await?;
        dispatch_queue::Entity::update_many()
            .col_expr(dispatch_queue::Column::Status, Expr::value("cancelled"))
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
            .filter(dispatch_queue::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .exec(&txn)
            .await?;
        if let Some(shard) = workflow_shard::Entity::find()
            .filter(workflow_shard::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&txn)
            .await?
        {
            let mut active: workflow_shard::ActiveModel = shard.into();
            active.status = Set("cancelled".to_owned());
            active.updated_at = Set(now.clone());
            active.update(&txn).await?;
        }
        instance_event::ActiveModel {
            id: Set(new_id("evt")),
            instance_id: Set(job_instance_id.to_owned()),
            instance_type: Set("job".to_owned()),
            event_type: Set("job.instance.cancelled".to_owned()),
            message: Set("job instance cancelled".to_owned()),
            payload: Set(None),
            created_at: Set(now),
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        Ok(instance_result.rows_affected > 0)
    }

    pub async fn rebalance_workflow_shards(
        &self,
        instance_id: &str,
        input: RebalanceWorkflowShardsInput,
    ) -> Result<Option<RebalanceWorkflowShardsResult>, sea_orm::DbErr> {
        let instance_exists = workflow_instance::Entity::find_by_id(instance_id.to_owned())
            .one(&self.db)
            .await?
            .is_some();
        if !instance_exists {
            return Ok(None);
        }
        let statuses = input.statuses.unwrap_or_else(|| vec!["failed".to_owned()]);
        let now = now_rfc3339();
        let mut query = workflow_shard::Entity::find()
            .filter(workflow_shard::Column::WorkflowInstanceId.eq(instance_id.to_owned()))
            .filter(workflow_shard::Column::Status.is_in(statuses));
        if let Some(node_key) = input
            .node_key
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            query = query.filter(workflow_shard::Column::NodeKey.eq(node_key.trim().to_owned()));
        }
        let shards = query.all(&self.db).await?;
        let txn = self.db.begin().await?;
        let mut requeued = Vec::new();
        for shard in shards {
            let next_retry_count = shard.retry_count.saturating_add(1);
            let job_instance_id = new_id("inst");
            crate::entities::job_instance::ActiveModel {
                id: Set(job_instance_id.clone()),
                job_id: Set(format!(
                    "workflow-shard-{}-{}",
                    shard.workflow_instance_id, shard.node_key
                )),
                status: Set("pending".to_owned()),
                trigger_type: Set("workflow_shard".to_owned()),
                execution_mode: Set("single".to_owned()),
                result_worker_id: Set(None),
                result_success: Set(None),
                result_message: Set(None),
                result_completed_at: Set(None),
                created_at: Set(now.clone()),
                updated_at: Set(now.clone()),
            }
            .insert(&txn)
            .await?;
            dispatch_queue::ActiveModel {
                id: Set(new_id("dq")),
                job_instance_id: Set(Some(job_instance_id.clone())),
                workflow_node_instance_id: Set(None),
                priority: Set(0),
                run_after: Set(now.clone()),
                status: Set("pending".to_owned()),
                attempt: Set(0),
                lease_owner: Set(None),
                lease_until: Set(None),
                fencing_token: Set(None),
                worker_selector: Set(None),
                namespace: Set(None),
                app: Set(None),
                worker_pool: Set(None),
                created_at: Set(now.clone()),
                updated_at: Set(now.clone()),
            }
            .insert(&txn)
            .await?;
            let mut active: workflow_shard::ActiveModel = shard.into();
            active.status = Set("pending".to_owned());
            active.output = Set(None);
            active.retry_count = Set(next_retry_count);
            active.job_instance_id = Set(Some(job_instance_id));
            active.updated_at = Set(now.clone());
            let updated = active.update(&txn).await?;
            requeued.push(WorkflowShardSummary::from(updated));
        }
        instance_event::ActiveModel {
            id: Set(new_id("evt")),
            instance_id: Set(instance_id.to_owned()),
            instance_type: Set("workflow".to_owned()),
            event_type: Set("workflow.shards.rebalanced".to_owned()),
            message: Set(input
                .message
                .unwrap_or_else(|| format!("rebalanced {} workflow shards", requeued.len()))),
            payload: Set(Some(
                serde_json::to_string(&requeued)
                    .map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?,
            )),
            created_at: Set(now),
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        Ok(Some(RebalanceWorkflowShardsResult {
            requeued_shards: requeued,
        }))
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
                namespace: Set(None),
                app: Set(None),
                worker_pool: Set(None),
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
