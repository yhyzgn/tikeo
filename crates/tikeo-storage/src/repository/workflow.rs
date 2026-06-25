use crate::entities::{
    dispatch_queue, instance_event, workflow, workflow_edge, workflow_instance, workflow_node,
    workflow_node_instance, workflow_shard,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};

use super::util::{new_id, now_rfc3339, rfc3339_after_seconds};
mod types;

use super::scheduler_shard_policy;
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

fn workflow_dispatch_shard(namespace: &str, app: &str, durable_id: &str) -> (i32, i64, i32) {
    let policy = scheduler_shard_policy();
    (
        policy.shard_id_for(namespace, app, durable_id),
        policy.shard_map_version,
        policy.shard_count,
    )
}

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

struct MaterializeNodeContext<'a> {
    txn: &'a DatabaseTransaction,
    workflow: &'a WorkflowSummary,
    instance: &'a workflow_instance::Model,
    node_spec: &'a WorkflowNodeSpec,
    node_instance_id: &'a str,
    now: &'a str,
}

async fn insert_workflow_job_instance(
    txn: &DatabaseTransaction,
    job_instance_id: &str,
    job_id: &str,
    trigger_type: &str,
    now: &str,
) -> Result<(), sea_orm::DbErr> {
    crate::entities::job_instance::ActiveModel {
        id: Set(job_instance_id.to_owned()),
        job_id: Set(job_id.to_owned()),
        status: Set("pending".to_owned()),
        trigger_type: Set(trigger_type.to_owned()),
        execution_mode: Set("single".to_owned()),
        result_worker_id: Set(None),
        result_success: Set(None),
        result_message: Set(None),
        result_completed_at: Set(None),
        created_at: Set(now.to_owned()),
        updated_at: Set(now.to_owned()),
    }
    .insert(txn)
    .await?;
    Ok(())
}

async fn insert_dispatch_queue_item(
    txn: &DatabaseTransaction,
    job_instance_id: Option<String>,
    workflow_node_instance_id: Option<String>,
    shard: (i32, i64, i32),
    run_after: &str,
    now: &str,
) -> Result<(), sea_orm::DbErr> {
    dispatch_queue::ActiveModel {
        id: Set(new_id("dq")),
        job_instance_id: Set(job_instance_id),
        workflow_node_instance_id: Set(workflow_node_instance_id),
        shard_id: Set(Some(shard.0)),
        shard_map_version: Set(Some(shard.1)),
        shard_count: Set(Some(shard.2)),
        owner_epoch: Set(None),
        owner_fencing_token: Set(None),
        priority: Set(0),
        run_after: Set(run_after.to_owned()),
        status: Set("pending".to_owned()),
        attempt: Set(0),
        lease_owner: Set(None),
        lease_until: Set(None),
        fencing_token: Set(None),
        worker_selector: Set(None),
        namespace: Set(None),
        app: Set(None),
        worker_pool: Set(None),
        created_at: Set(now.to_owned()),
        updated_at: Set(now.to_owned()),
    }
    .insert(txn)
    .await?;
    Ok(())
}

async fn materialize_job_dispatch(
    ctx: &MaterializeNodeContext<'_>,
    job_id: &str,
    trigger_type: &str,
    node_active: &mut workflow_node_instance::ActiveModel,
) -> Result<String, sea_orm::DbErr> {
    let job_instance_id = new_id("inst");
    ensure_workflow_job_soft_link(ctx.txn, job_id, ctx.now).await?;
    insert_workflow_job_instance(ctx.txn, &job_instance_id, job_id, trigger_type, ctx.now).await?;
    node_active.job_instance_id = Set(Some(job_instance_id.clone()));
    let shard = workflow_dispatch_shard("workflow", &ctx.workflow.name, &job_instance_id);
    insert_dispatch_queue_item(
        ctx.txn,
        Some(job_instance_id.clone()),
        None,
        shard,
        ctx.now,
        ctx.now,
    )
    .await?;
    Ok(job_instance_id)
}

async fn materialize_map_node(
    ctx: &MaterializeNodeContext<'_>,
) -> Result<Vec<WorkflowShardSummary>, sea_orm::DbErr> {
    let shard_job_id = ctx.node_spec.job_id.clone().unwrap_or_else(|| {
        format!(
            "workflow-shard-{}-{}",
            ctx.instance.workflow_id, ctx.node_spec.key
        )
    });
    ensure_workflow_job_soft_link(ctx.txn, &shard_job_id, ctx.now).await?;
    let mut shards = Vec::new();
    for (index, item) in ctx
        .node_spec
        .map_items
        .clone()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
    {
        let job_instance_id = new_id("inst");
        let shard = workflow_shard::ActiveModel {
            id: Set(new_id("wfs")),
            workflow_instance_id: Set(ctx.instance.id.clone()),
            workflow_node_instance_id: Set(ctx.node_instance_id.to_owned()),
            node_key: Set(ctx.node_spec.key.clone()),
            shard_index: Set(i32::try_from(index).unwrap_or(i32::MAX)),
            status: Set("pending".to_owned()),
            input: Set(serde_json::to_string(&item).unwrap_or_else(|_| "null".to_owned())),
            output: Set(None),
            checkpoint: Set(None),
            retry_count: Set(0),
            job_instance_id: Set(Some(job_instance_id.clone())),
            created_at: Set(ctx.now.to_owned()),
            updated_at: Set(ctx.now.to_owned()),
        }
        .insert(ctx.txn)
        .await?;
        insert_workflow_job_instance(
            ctx.txn,
            &job_instance_id,
            &shard_job_id,
            "workflow_shard",
            ctx.now,
        )
        .await?;
        let dispatch_shard =
            workflow_dispatch_shard("workflow", &ctx.workflow.name, &job_instance_id);
        insert_dispatch_queue_item(
            ctx.txn,
            Some(job_instance_id),
            None,
            dispatch_shard,
            ctx.now,
            ctx.now,
        )
        .await?;
        shards.push(WorkflowShardSummary::from(shard));
    }
    Ok(shards)
}

async fn materialize_child_start_nodes(
    ctx: &MaterializeNodeContext<'_>,
    child_id: &str,
    child_workflow: &WorkflowSummary,
) -> Result<(), sea_orm::DbErr> {
    let child_start_nodes = start_node_keys(&child_workflow.definition);
    for child_node in &child_workflow.definition.nodes {
        let is_start = child_start_nodes.contains(&child_node.key);
        let child_node_instance = workflow_node_instance::ActiveModel {
            id: Set(new_id("wfni")),
            workflow_instance_id: Set(child_id.to_owned()),
            node_key: Set(child_node.key.clone()),
            status: Set(if is_start { "queued" } else { "waiting" }.to_owned()),
            job_instance_id: Set(None),
            child_workflow_instance_id: Set(None),
            created_at: Set(ctx.now.to_owned()),
            updated_at: Set(ctx.now.to_owned()),
        }
        .insert(ctx.txn)
        .await?;
        if is_start {
            let shard = workflow_dispatch_shard(
                "workflow",
                &child_workflow.name,
                &format!("{child_id}:{}", child_node.key),
            );
            insert_dispatch_queue_item(
                ctx.txn,
                None,
                Some(child_node_instance.id),
                shard,
                &workflow_node_run_after(&child_workflow.definition, &child_node.key, ctx.now),
                ctx.now,
            )
            .await?;
        }
    }
    Ok(())
}

async fn insert_workflow_event(
    txn: &DatabaseTransaction,
    instance_id: &str,
    event_type: &str,
    message: String,
    now: &str,
) -> Result<(), sea_orm::DbErr> {
    instance_event::ActiveModel {
        id: Set(new_id("evt")),
        instance_id: Set(instance_id.to_owned()),
        instance_type: Set("workflow".to_owned()),
        event_type: Set(event_type.to_owned()),
        message: Set(message),
        payload: Set(None),
        created_at: Set(now.to_owned()),
    }
    .insert(txn)
    .await?;
    Ok(())
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
    /// New.
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    #[must_use]
    /// Db.
    pub fn db(&self) -> DatabaseConnection {
        self.db.clone()
    }

    /// Create workflow.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
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

    /// Update workflow.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
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

    /// List workflows.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn list_workflows(&self) -> Result<Vec<WorkflowSummary>, sea_orm::DbErr> {
        let rows = workflow::Entity::find()
            .order_by_desc(workflow::Column::CreatedAt)
            .all(&self.db)
            .await?;
        rows.into_iter().map(WorkflowSummary::from_model).collect()
    }

    /// Get workflow.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn get_workflow(&self, id: &str) -> Result<Option<WorkflowSummary>, sea_orm::DbErr> {
        workflow::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
            .map(WorkflowSummary::from_model)
            .transpose()
    }

    /// Run workflow.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
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
                let (shard_id, shard_map_version, shard_count) = workflow_dispatch_shard(
                    "workflow",
                    &workflow.name,
                    &format!("{}:{}", instance_id, node.key),
                );
                dispatch_queue::ActiveModel {
                    id: Set(new_id("dq")),
                    job_instance_id: Set(None),
                    workflow_node_instance_id: Set(Some(node_instance.id.clone())),
                    shard_id: Set(Some(shard_id)),
                    shard_map_version: Set(Some(shard_map_version)),
                    shard_count: Set(Some(shard_count)),
                    owner_epoch: Set(None),
                    owner_fencing_token: Set(None),
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

    /// Advance workflow.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
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
                    let (shard_id, shard_map_version, shard_count) = workflow_dispatch_shard(
                        "workflow",
                        &workflow.name,
                        &format!("{instance_id}:{node_key}"),
                    );
                    dispatch_queue::ActiveModel {
                        id: Set(new_id("dq")),
                        job_instance_id: Set(None),
                        workflow_node_instance_id: Set(Some(queued.id)),
                        shard_id: Set(Some(shard_id)),
                        shard_map_version: Set(Some(shard_map_version)),
                        shard_count: Set(Some(shard_count)),
                        owner_epoch: Set(None),
                        owner_fencing_token: Set(None),
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

    /// Materialize next queued node.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn materialize_next_queued_node(
        &self,
    ) -> Result<Option<MaterializeWorkflowNodeResult>, sea_orm::DbErr> {
        self.materialize_next_queued_node_with_lease("tikeo-dispatcher", 30)
            .await
    }

    /// Materialize next queued node with lease.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn materialize_next_queued_node_with_lease(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
    ) -> Result<Option<MaterializeWorkflowNodeResult>, sea_orm::DbErr> {
        self.materialize_next_queued_node_with_fencing(lease_owner, lease_seconds, lease_owner)
            .await
    }

    async fn materialize_node(
        &self,
        ctx: MaterializeNodeContext<'_>,
        node_active: &mut workflow_node_instance::ActiveModel,
    ) -> Result<Vec<WorkflowShardSummary>, sea_orm::DbErr> {
        match node_kind(ctx.node_spec) {
            "job" => {
                let job_id = ctx.node_spec.job_id.clone().unwrap_or_default();
                materialize_job_dispatch(&ctx, &job_id, "workflow", node_active).await?;
                Ok(Vec::new())
            }
            "map" | "map_reduce" => materialize_map_node(&ctx).await,
            "sub_workflow" => {
                self.materialize_sub_workflow(&ctx, node_active).await?;
                Ok(Vec::new())
            }
            "script" => {
                let job_id = ctx.node_spec.job_id.clone().unwrap_or_else(|| {
                    format!(
                        "workflow-script-{}-{}",
                        ctx.instance.workflow_id, ctx.node_spec.key
                    )
                });
                materialize_job_dispatch(&ctx, &job_id, "workflow", node_active).await?;
                Ok(Vec::new())
            }
            "http" | "grpc" | "sql" | "file_cleanup" => {
                let job_id = format!(
                    "workflow-{}-{}-{}",
                    node_kind(ctx.node_spec),
                    ctx.instance.workflow_id,
                    ctx.node_spec.key
                );
                materialize_job_dispatch(&ctx, &job_id, "workflow", node_active).await?;
                Ok(Vec::new())
            }
            "condition" => {
                node_active.status = Set(if evaluate_condition_node(ctx.node_spec) {
                    "succeeded".to_owned()
                } else {
                    "failed".to_owned()
                });
                Ok(Vec::new())
            }
            "approval" => {
                node_active.status = Set(
                    if workflow_config_bool(ctx.node_spec, "approved").unwrap_or(false) {
                        "succeeded".to_owned()
                    } else {
                        "running".to_owned()
                    },
                );
                Ok(Vec::new())
            }
            "delay" | "parallel" | "join" | "notification" | "compensation" | "start" | "end" => {
                node_active.status = Set("succeeded".to_owned());
                Ok(Vec::new())
            }
            _ => Ok(Vec::new()),
        }
    }

    async fn materialize_sub_workflow(
        &self,
        ctx: &MaterializeNodeContext<'_>,
        node_active: &mut workflow_node_instance::ActiveModel,
    ) -> Result<(), sea_orm::DbErr> {
        let child_id = new_id("wfi");
        let child_workflow_id = ctx.node_spec.child_workflow_id.clone().unwrap_or_default();
        workflow_instance::ActiveModel {
            id: Set(child_id.clone()),
            workflow_id: Set(child_workflow_id.clone()),
            status: Set("pending".to_owned()),
            trigger_type: Set("sub_workflow".to_owned()),
            created_at: Set(ctx.now.to_owned()),
            updated_at: Set(ctx.now.to_owned()),
        }
        .insert(ctx.txn)
        .await?;
        if let Some(child_workflow) = self.get_workflow(&child_workflow_id).await? {
            materialize_child_start_nodes(ctx, &child_id, &child_workflow).await?;
            insert_workflow_event(
                ctx.txn,
                &child_id,
                "workflow.started",
                format!("workflow {child_workflow_id} started"),
                ctx.now,
            )
            .await?;
        }
        node_active.child_workflow_instance_id = Set(Some(child_id.clone()));
        insert_workflow_event(
            ctx.txn,
            &ctx.instance.id,
            "workflow.sub_workflow.started",
            format!("child workflow {child_id} started"),
            ctx.now,
        )
        .await?;
        Ok(())
    }

    /// Materialize next queued node with fencing.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
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
        self.materialize_claimed_workflow_node_queue_item(&claim.item.id)
            .await
    }

    /// Materialize claimed workflow node queue item.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn materialize_claimed_workflow_node_queue_item(
        &self,
        queue_id: &str,
    ) -> Result<Option<MaterializeWorkflowNodeResult>, sea_orm::DbErr> {
        let Some(queue_row) = dispatch_queue::Entity::find_by_id(queue_id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let Some(node_instance_id) = queue_row.workflow_node_instance_id.clone() else {
            return Ok(None);
        };
        let Some(node_instance) = workflow_node_instance::Entity::find_by_id(&node_instance_id)
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
        let shards = self
            .materialize_node(
                MaterializeNodeContext {
                    txn: &txn,
                    workflow: &workflow,
                    instance: &instance,
                    node_spec: &node_spec,
                    node_instance_id: &node_instance_id,
                    now: &now,
                },
                &mut node_active,
            )
            .await?;
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

    /// Get workflow instance.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
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
mod recovery;
mod runtime;
mod validation;

pub use validation::validate_workflow_definition;
