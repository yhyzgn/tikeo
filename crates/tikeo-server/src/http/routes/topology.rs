use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use tikeo_storage::{JobSummary as StorageJobSummary, WorkflowNodeSpec, WorkflowSummary};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, ErrorResponse, JobImpactApiResponse, JobImpactJobRef, JobImpactResponse,
        JobImpactRiskSummary, JobImpactWorkflowRef, JobTopologyApiResponse, JobTopologyEdge,
        JobTopologyNode, JobTopologyResponse, JobTopologyUnresolvedRef, WorkflowReplayApiResponse,
        WorkflowReplayResponse,
    },
    error::ApiError,
};

/// Discover job dependency topology from active jobs and workflow definitions.
///
/// # Errors
///
/// Returns authorization or storage errors when topology inputs cannot be loaded.
#[utoipa::path(
    get,
    path = "/api/v1/jobs/topology",
    tag = "jobs",
    responses(
        (status = 200, description = "Job topology graph", body = JobTopologyApiResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
/// Job topology.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn job_topology(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<JobTopologyApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "jobs", "read").await?;
    let jobs = state
        .jobs
        .list_jobs()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let visible_jobs: Vec<_> = jobs
        .into_iter()
        .filter(|job| {
            crate::http::access_scope::allows_resource(
                &principal.scope_bindings,
                &job.namespace,
                &job.app,
                None,
            )
        })
        .collect();
    let workflows = state
        .workflows
        .list_workflows()
        .await
        .map_err(|error| ApiError::storage(&error))?;

    Ok(Json(ApiResponse::success(build_job_topology(
        &visible_jobs,
        &workflows,
    ))))
}

/// Analyze cross-workflow impact for one job.
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors when impact inputs cannot be loaded.
#[utoipa::path(
    get,
    path = "/api/v1/jobs/{job}/impact",
    tag = "jobs",
    params(("job" = String, Path, description = "Job identifier")),
    responses(
        (status = 200, description = "Job impact analysis", body = JobImpactApiResponse),
        (status = 404, description = "Job not found", body = ErrorResponse)
    )
)]
/// Job impact.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn job_impact(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job): Path<String>,
) -> Result<Json<JobImpactApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "jobs", "read").await?;
    let jobs = state
        .jobs
        .list_jobs()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let visible_jobs: Vec<_> = jobs
        .into_iter()
        .filter(|item| {
            crate::http::access_scope::allows_resource(
                &principal.scope_bindings,
                &item.namespace,
                &item.app,
                None,
            )
        })
        .collect();
    let target = visible_jobs
        .iter()
        .find(|item| item.id == job)
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;
    let workflows = state
        .workflows
        .list_workflows()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let topology = build_job_topology(&visible_jobs, &workflows);
    Ok(Json(ApiResponse::success(build_job_impact(
        target,
        &visible_jobs,
        &workflows,
        &topology,
    ))))
}

/// Return a workflow replay bundle with definition graph and persisted timeline events.
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors when replay inputs cannot be loaded.
#[utoipa::path(
    get,
    path = "/api/v1/workflow-instances/{id}/replay",
    tag = "workflows",
    params(("id" = String, Path, description = "Workflow instance identifier")),
    responses(
        (status = 200, description = "Workflow replay bundle", body = WorkflowReplayApiResponse),
        (status = 404, description = "Workflow instance not found", body = ErrorResponse)
    )
)]
/// Workflow replay.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn workflow_replay(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<WorkflowReplayApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "workflows", "read").await?;
    let instance = state
        .workflows
        .get_workflow_instance(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("workflow instance not found: {id}")))?;
    let workflow = state
        .workflows
        .get_workflow(&instance.workflow_id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| {
            ApiError::not_found(format!("workflow not found: {}", instance.workflow_id))
        })?;
    let events = state
        .workflows
        .list_instance_events(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let graph = build_workflow_replay_graph(&workflow);
    Ok(Json(ApiResponse::success(WorkflowReplayResponse {
        instance,
        workflow,
        events,
        graph,
    })))
}

fn build_job_topology(
    jobs: &[StorageJobSummary],
    workflows: &[WorkflowSummary],
) -> JobTopologyResponse {
    let job_by_id: HashMap<&str, &StorageJobSummary> =
        jobs.iter().map(|job| (job.id.as_str(), job)).collect();
    let mut nodes = jobs
        .iter()
        .enumerate()
        .map(|(index, job)| job_node(job, 0, index))
        .collect::<Vec<_>>();
    nodes.extend(
        workflows
            .iter()
            .enumerate()
            .map(|(index, workflow)| workflow_node(workflow, index)),
    );

    let mut edges = Vec::new();
    let mut unresolved = Vec::new();
    for workflow in workflows {
        add_workflow_topology(workflow, &job_by_id, &mut edges, &mut unresolved);
    }

    JobTopologyResponse {
        nodes,
        edges,
        unresolved,
    }
}

fn build_job_impact(
    target: &StorageJobSummary,
    jobs: &[StorageJobSummary],
    workflows: &[WorkflowSummary],
    topology: &JobTopologyResponse,
) -> JobImpactResponse {
    let job_by_id: HashMap<&str, &StorageJobSummary> =
        jobs.iter().map(|job| (job.id.as_str(), job)).collect();
    let mut workflow_nodes = BTreeMap::<String, (String, BTreeSet<String>)>::new();
    let mut upstream = BTreeSet::<String>::new();
    let mut downstream = BTreeSet::<String>::new();
    for workflow in workflows {
        let nodes_by_key: HashMap<&str, &WorkflowNodeSpec> = workflow
            .definition
            .nodes
            .iter()
            .map(|node| (node.key.as_str(), node))
            .collect();
        for node in &workflow.definition.nodes {
            if node.job_id.as_deref() == Some(target.id.as_str()) {
                workflow_nodes
                    .entry(workflow.id.clone())
                    .or_insert_with(|| (workflow.name.clone(), BTreeSet::new()))
                    .1
                    .insert(node.key.clone());
            }
        }
        for edge in &workflow.definition.edges {
            let from_job = nodes_by_key
                .get(edge.from.as_str())
                .and_then(|node| node.job_id.as_deref());
            let to_job = nodes_by_key
                .get(edge.to.as_str())
                .and_then(|node| node.job_id.as_deref());
            if to_job == Some(target.id.as_str())
                && let Some(from_job) = from_job
            {
                upstream.insert(from_job.to_owned());
            }
            if from_job == Some(target.id.as_str())
                && let Some(to_job) = to_job
            {
                downstream.insert(to_job.to_owned());
            }
        }
    }
    let referencing_workflows = workflow_nodes
        .into_iter()
        .map(|(id, (name, node_keys))| JobImpactWorkflowRef {
            id,
            name,
            node_keys: node_keys.into_iter().collect(),
        })
        .collect::<Vec<_>>();
    let upstream_jobs = impact_job_refs(upstream, &job_by_id);
    let downstream_jobs = impact_job_refs(downstream, &job_by_id);
    let risk_summary = impact_risk_summary(
        referencing_workflows.len(),
        upstream_jobs.len(),
        downstream_jobs.len(),
        topology.unresolved.len(),
    );
    JobImpactResponse {
        target_job: impact_job_ref(target),
        referencing_workflows,
        upstream_jobs,
        downstream_jobs,
        risk_summary,
    }
}

fn impact_job_refs(
    ids: BTreeSet<String>,
    job_by_id: &HashMap<&str, &StorageJobSummary>,
) -> Vec<JobImpactJobRef> {
    ids.into_iter()
        .filter_map(|id| job_by_id.get(id.as_str()).copied())
        .map(impact_job_ref)
        .collect()
}

fn impact_job_ref(job: &StorageJobSummary) -> JobImpactJobRef {
    JobImpactJobRef {
        id: job.id.clone(),
        name: job.name.clone(),
        namespace: job.namespace.clone(),
        app: job.app.clone(),
    }
}

fn impact_risk_summary(
    workflow_count: usize,
    upstream_count: usize,
    downstream_count: usize,
    unresolved_count: usize,
) -> JobImpactRiskSummary {
    let risk_level = if unresolved_count > 0 || workflow_count >= 3 {
        "high"
    } else if workflow_count > 0 || downstream_count > 0 {
        "medium"
    } else {
        "low"
    };
    let mut reasons = Vec::new();
    reasons.push(format!("referenced by {workflow_count} workflow(s)"));
    if upstream_count > 0 {
        reasons.push(format!(
            "{upstream_count} upstream job(s) may feed this job"
        ));
    }
    if downstream_count > 0 {
        reasons.push(format!(
            "{downstream_count} downstream job(s) may be affected"
        ));
    }
    if unresolved_count > 0 {
        reasons.push(format!(
            "{unresolved_count} unresolved topology reference(s) exist"
        ));
    }
    JobImpactRiskSummary {
        workflow_count: u64::try_from(workflow_count).unwrap_or(u64::MAX),
        upstream_count: u64::try_from(upstream_count).unwrap_or(u64::MAX),
        downstream_count: u64::try_from(downstream_count).unwrap_or(u64::MAX),
        unresolved_count: u64::try_from(unresolved_count).unwrap_or(u64::MAX),
        risk_level: risk_level.to_owned(),
        reasons,
    }
}

fn build_workflow_replay_graph(workflow: &WorkflowSummary) -> JobTopologyResponse {
    let nodes = workflow
        .definition
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| JobTopologyNode {
            id: node.key.clone(),
            node_type: "workflow_node".to_owned(),
            label: node.name.clone().unwrap_or_else(|| node.key.clone()),
            namespace: None,
            app: None,
            metadata: serde_json::json!({
                "layer": index,
                "position": { "x": 80 + (index * 180), "y": 120 },
                "jobId": node.job_id,
                "kind": node.kind,
            }),
        })
        .collect();
    let edges = workflow
        .definition
        .edges
        .iter()
        .map(|edge| JobTopologyEdge {
            id: format!("{}:{}:{}", workflow.id, edge.from, edge.to),
            from: edge.from.clone(),
            to: edge.to.clone(),
            edge_type: "workflow_node_dependency".to_owned(),
            label: edge.condition.clone(),
            workflow_id: Some(workflow.id.clone()),
            workflow_name: Some(workflow.name.clone()),
            condition: edge.condition.clone(),
            metadata: serde_json::json!({}),
        })
        .collect();
    JobTopologyResponse {
        nodes,
        edges,
        unresolved: Vec::new(),
    }
}

fn job_node(job: &StorageJobSummary, layer: usize, index: usize) -> JobTopologyNode {
    JobTopologyNode {
        id: job.id.clone(),
        node_type: "job".to_owned(),
        label: job.name.clone(),
        namespace: Some(job.namespace.clone()),
        app: Some(job.app.clone()),
        metadata: serde_json::json!({
            "layer": layer,
            "position": { "x": 80 + (index * 180), "y": 120 + (layer * 180) },
            "scheduleType": job.schedule_type,
            "enabled": job.enabled,
            "versionNumber": job.version_number,
            "processorName": job.processor_name,
            "scriptId": job.script_id,
            "canaryJobId": job.canary_job_id,
            "canaryPercent": job.canary_percent,
        }),
    }
}

fn workflow_node(workflow: &WorkflowSummary, index: usize) -> JobTopologyNode {
    JobTopologyNode {
        id: workflow.id.clone(),
        node_type: "workflow".to_owned(),
        label: workflow.name.clone(),
        namespace: None,
        app: None,
        metadata: serde_json::json!({
            "layer": 1,
            "position": { "x": 80 + (index * 180), "y": 300 },
            "status": workflow.status,
            "nodeCount": workflow.definition.nodes.len(),
            "edgeCount": workflow.definition.edges.len(),
        }),
    }
}

fn add_workflow_topology(
    workflow: &WorkflowSummary,
    job_by_id: &HashMap<&str, &StorageJobSummary>,
    edges: &mut Vec<JobTopologyEdge>,
    unresolved: &mut Vec<JobTopologyUnresolvedRef>,
) {
    let nodes_by_key: HashMap<&str, &WorkflowNodeSpec> = workflow
        .definition
        .nodes
        .iter()
        .map(|node| (node.key.as_str(), node))
        .collect();
    for node in &workflow.definition.nodes {
        add_workflow_job_ref(workflow, node, job_by_id, edges, unresolved);
    }
    for edge in &workflow.definition.edges {
        let Some(from_job) = nodes_by_key
            .get(edge.from.as_str())
            .and_then(|node| node.job_id.as_ref())
        else {
            continue;
        };
        let Some(to_job) = nodes_by_key
            .get(edge.to.as_str())
            .and_then(|node| node.job_id.as_ref())
        else {
            continue;
        };
        if job_by_id.contains_key(from_job.as_str()) && job_by_id.contains_key(to_job.as_str()) {
            let condition = edge
                .condition
                .clone()
                .unwrap_or_else(|| "always".to_owned());
            edges.push(JobTopologyEdge {
                id: format!("{}:{}:{}", workflow.id, edge.from, edge.to),
                from: from_job.clone(),
                to: to_job.clone(),
                edge_type: "workflow_job_dependency".to_owned(),
                label: Some(condition.clone()),
                workflow_id: Some(workflow.id.clone()),
                workflow_name: Some(workflow.name.clone()),
                condition: Some(condition),
                metadata: serde_json::json!({
                    "fromNodeKey": edge.from,
                    "toNodeKey": edge.to,
                }),
            });
        }
    }
}

fn add_workflow_job_ref(
    workflow: &WorkflowSummary,
    node: &WorkflowNodeSpec,
    job_by_id: &HashMap<&str, &StorageJobSummary>,
    edges: &mut Vec<JobTopologyEdge>,
    unresolved: &mut Vec<JobTopologyUnresolvedRef>,
) {
    let Some(job_id) = node
        .job_id
        .as_ref()
        .filter(|job_id| !job_id.trim().is_empty())
    else {
        return;
    };
    if job_by_id.contains_key(job_id.as_str()) {
        edges.push(JobTopologyEdge {
            id: format!("{}:{}:{}", workflow.id, node.key, job_id),
            from: workflow.id.clone(),
            to: job_id.clone(),
            edge_type: "workflow_job_ref".to_owned(),
            label: Some(node.key.clone()),
            workflow_id: Some(workflow.id.clone()),
            workflow_name: Some(workflow.name.clone()),
            condition: None,
            metadata: serde_json::json!({
                "nodeKey": node.key,
                "nodeName": node.name,
                "nodeKind": node.kind,
            }),
        });
    } else {
        unresolved.push(JobTopologyUnresolvedRef {
            workflow_id: workflow.id.clone(),
            workflow_name: workflow.name.clone(),
            node_key: node.key.clone(),
            missing_job_id: job_id.clone(),
            reason: "workflow node references missing job".to_owned(),
        });
    }
}
