#![allow(missing_docs)]

use std::collections::{HashMap, HashSet};

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::entities::{
    dispatch_queue, instance_event, workflow, workflow_edge, workflow_instance, workflow_node,
    workflow_node_instance,
};

use super::util::{new_id, now_rfc3339};

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
    pub created_at: String,
    pub updated_at: String,
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
                    .unwrap_or_else(|| "on_success".to_owned())),
                created_at: Set(now.clone()),
            }
            .insert(&txn)
            .await?;
        }
        txn.commit().await?;
        WorkflowSummary::from_model(model)
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
        Ok(Some(AdvanceWorkflowResult {
            instance: refreshed,
            queued_nodes,
            completed,
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
    let allowed_kinds = ["job", "map", "map_reduce", "sub_workflow"];
    for node in &definition.nodes {
        let kind = node.kind.as_deref().unwrap_or("job");
        if !allowed_kinds.contains(&kind) {
            errors.push(format!("unsupported node kind: {kind}"));
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
        let condition = edge.condition.as_deref().unwrap_or("on_success");
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
            edge_condition_satisfied(status, edge.condition.as_deref().unwrap_or("on_success"))
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
            edge.condition.as_deref().unwrap_or("on_success"),
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
