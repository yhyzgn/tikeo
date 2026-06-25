use crate::repository::job::{JobCanaryPolicy, JobRetryPolicy};

use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

use crate::entities::{
    app, dispatch_queue, instance_event, job, namespace, workflow, workflow_instance,
    workflow_node_instance, workflow_shard,
};

use super::types::{
    DispatchQueueSummary, InstanceEventSummary, WorkflowInstanceSummary,
    WorkflowNodeInstanceSummary, WorkflowNodeSpec, WorkflowShardSummary, WorkflowSummary,
};

impl WorkflowSummary {
    /// From model.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub(super) fn from_model(model: workflow::Model) -> Result<Self, sea_orm::DbErr> {
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
    /// From.
    pub(super) fn from(model: workflow_node_instance::Model) -> Self {
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
    /// From model.
    pub(super) fn from_model(
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

/// Ensure workflow job soft link.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub(super) async fn ensure_workflow_job_soft_link<C>(
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
        misfire_policy: Set("fire_once".to_owned()),
        schedule_start_at: Set(None),
        schedule_end_at: Set(None),
        schedule_calendar_json: Set(None),
        processor_name: Set(Some(job_id.to_owned())),
        processor_type: Set(None),
        script_id: Set(None),
        enabled: Set(true),
        canary_job_id: Set(None),
        canary_percent: Set(0),
        canary_policy_json: Set(JobCanaryPolicy::default_json()),
        retry_policy_json: Set(JobRetryPolicy::default_json()),
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
            checkpoint: model
                .checkpoint
                .and_then(|value| serde_json::from_str(&value).ok()),
            retry_count: model.retry_count,
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
            shard_id: model.shard_id,
            shard_map_version: model.shard_map_version,
            shard_count: model.shard_count,
            owner_epoch: model.owner_epoch,
            owner_fencing_token: model.owner_fencing_token,
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

/// Dispatch queue age seconds.
pub(super) fn dispatch_queue_age_seconds(created_at: &str, now: time::OffsetDateTime) -> u64 {
    time::OffsetDateTime::parse(created_at, &time::format_description::well_known::Rfc3339)
        .ok()
        .and_then(|created| (now - created).whole_seconds().try_into().ok())
        .unwrap_or(0)
}

/// Elapsed seconds.
pub(super) fn elapsed_seconds(start: &str, end: &str) -> u64 {
    let Ok(start) =
        time::OffsetDateTime::parse(start, &time::format_description::well_known::Rfc3339)
    else {
        return 0;
    };
    let Ok(end) = time::OffsetDateTime::parse(end, &time::format_description::well_known::Rfc3339)
    else {
        return 0;
    };
    (end - start).whole_seconds().try_into().unwrap_or(0)
}

/// Success ratio.
pub(super) fn success_ratio(successes: u64, failures: u64) -> f64 {
    let terminal = successes.saturating_add(failures);
    if terminal == 0 {
        return 1.0;
    }
    let bounded_successes = u32::try_from(successes).unwrap_or(u32::MAX);
    let bounded_terminal = u32::try_from(terminal).unwrap_or(u32::MAX);
    f64::from(bounded_successes) / f64::from(bounded_terminal)
}

/// Normalize terminal status.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub(super) fn normalize_terminal_status(status: &str) -> Result<String, sea_orm::DbErr> {
    match status {
        "succeeded" | "failed" => Ok(status.to_owned()),
        other => Err(sea_orm::DbErr::Custom(format!(
            "unsupported workflow shard status: {other}"
        ))),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DispatchQueueClaimKind {
    Any,
    WorkflowNode,
    JobInstance,
}

/// Normalize processor name.
pub(super) fn normalize_processor_name(value: Option<String>) -> Option<String> {
    value.and_then(|name| {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

/// Node kind.
pub(super) fn node_kind(node: &WorkflowNodeSpec) -> &str {
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
