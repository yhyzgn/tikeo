use serde::Serialize;

use crate::http::{
    AppState,
    dto::{
        JobInstanceAttemptSummary, JobInstanceLogSummary, JobInstanceResult, JobInstanceSummary,
        JobSummary,
    },
    error::ApiError,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Snapshot emitted on the job instance log stream when instance state changes.
pub(super) struct JobInstanceLogStreamSnapshot {
    /// Latest instance summary.
    pub(super) instance: JobInstanceSummary,
    /// Latest broadcast attempt summaries for the instance.
    pub(super) attempts: Vec<JobInstanceAttemptSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Attempt summaries grouped by instance for the instance-list stream.
pub(super) struct JobInstanceListStreamAttemptGroup {
    /// Instance identifier.
    pub(super) instance_id: String,
    /// Latest attempt summaries for the instance.
    pub(super) items: Vec<JobInstanceAttemptSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Snapshot emitted on the global job instance list stream.
pub(super) struct JobInstanceListStreamSnapshot {
    /// Jobs visible to the current principal.
    pub(super) jobs: Vec<JobSummary>,
    /// Latest visible instances sorted newest first.
    pub(super) instances: Vec<JobInstanceSummary>,
    /// Attempt summaries grouped by instance. Omitted on the list stream to avoid per-instance N+1 work; details stream carries attempts.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) attempts: Vec<JobInstanceListStreamAttemptGroup>,
}

impl From<tikeo_storage::JobSummary> for JobSummary {
    fn from(value: tikeo_storage::JobSummary) -> Self {
        Self {
            id: value.id,
            namespace: value.namespace,
            app: value.app,
            name: value.name,
            schedule_type: value.schedule_type,
            schedule_expr: value.schedule_expr,
            misfire_policy: value.misfire_policy,
            schedule_start_at: value.schedule_start_at,
            schedule_end_at: value.schedule_end_at,
            schedule_calendar: value
                .schedule_calendar_json
                .as_deref()
                .and_then(|value| serde_json::from_str(value).ok()),
            processor_name: value.processor_name,
            processor_type: value.processor_type,
            worker_pool: value.worker_pool,
            script_id: value.script_id,
            version_number: value.version_number,
            enabled: value.enabled,
            canary_job_id: value.canary_job_id,
            canary_percent: value.canary_percent,
            canary_policy: value.canary_policy,
            retry_policy: value.retry_policy,
        }
    }
}

pub(super) async fn instance_list_stream_snapshot(
    state: &AppState,
    principal: &crate::http::dto::MeResponse,
) -> Result<JobInstanceListStreamSnapshot, ApiError> {
    let jobs = state
        .jobs
        .list_jobs()
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .filter(|job| {
            crate::http::access_scope::allows_resource(
                &principal.scope_bindings,
                &job.namespace,
                &job.app,
                job.worker_pool.as_deref(),
            )
        })
        .map(JobSummary::from)
        .collect::<Vec<_>>();

    let mut instances = Vec::new();
    for job in &jobs {
        for instance in state
            .instances
            .list_by_job(&job.id)
            .await
            .map_err(|error| ApiError::storage(&error))?
        {
            instances.push(instance_summary_with_latest_log(state, instance).await?);
        }
    }
    instances.sort_by(|left, right| right.created_at.cmp(&left.created_at));

    Ok(JobInstanceListStreamSnapshot {
        jobs,
        instances,
        attempts: Vec::new(),
    })
}

pub(super) async fn instance_summary_with_latest_log(
    state: &AppState,
    value: tikeo_storage::JobInstanceSummary,
) -> Result<JobInstanceSummary, ApiError> {
    let log_count = state
        .logs
        .count_by_instance(&value.id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let latest_log = state
        .logs
        .latest_by_instance(&value.id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .map(JobInstanceLogSummary::from);
    let worker_id = state
        .logs
        .latest_worker_by_instance(&value.id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(JobInstanceSummary {
        id: value.id,
        job_id: value.job_id,
        status: value.status.to_string(),
        trigger_type: value.trigger_type.to_string(),
        execution_mode: value.execution_mode.to_string(),
        created_at: value.created_at,
        updated_at: value.updated_at,
        log_count,
        latest_log,
        worker_id,
        result: value.result.map(|result| JobInstanceResult {
            worker_id: result.worker_id,
            success: result.success,
            message: result.message,
            completed_at: result.completed_at,
        }),
        canary_routing: None,
    })
}

pub(super) async fn instance_log_stream_snapshot(
    state: &AppState,
    instance: &str,
) -> Result<JobInstanceLogStreamSnapshot, ApiError> {
    let summary = state
        .instances
        .get(instance)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("instance not found: {instance}")))?;
    let attempts = state
        .attempts
        .list_by_instance(instance)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(JobInstanceAttemptSummary::from)
        .collect();

    Ok(JobInstanceLogStreamSnapshot {
        instance: instance_summary_with_latest_log(state, summary).await?,
        attempts,
    })
}

impl From<tikeo_storage::JobInstanceAttemptSummary> for JobInstanceAttemptSummary {
    fn from(value: tikeo_storage::JobInstanceAttemptSummary) -> Self {
        Self {
            id: value.id,
            instance_id: value.instance_id,
            worker_id: value.worker_id,
            status: value.status.to_string(),
            result: value.result.map(|result| JobInstanceResult {
                worker_id: result.worker_id,
                success: result.success,
                message: result.message,
                completed_at: result.completed_at,
            }),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<tikeo_storage::JobInstanceLogSummary> for JobInstanceLogSummary {
    fn from(value: tikeo_storage::JobInstanceLogSummary) -> Self {
        let governance = parse_log_governance(&value.message);
        Self {
            id: value.id,
            instance_id: value.instance_id,
            worker_id: value.worker_id,
            level: value.level,
            message: governance
                .as_ref()
                .and_then(|parsed| parsed.message.clone())
                .unwrap_or(value.message),
            governance_event: governance.as_ref().map(|parsed| parsed.event.clone()),
            governance_failure_class: governance
                .as_ref()
                .and_then(|parsed| parsed.failure_class.clone()),
            governance_message: governance.and_then(|parsed| parsed.message),
            sequence: value.sequence,
            created_at: value.created_at,
        }
    }
}

struct ParsedLogGovernance {
    event: String,
    failure_class: Option<String>,
    message: Option<String>,
}

fn parse_log_governance(message: &str) -> Option<ParsedLogGovernance> {
    let value = serde_json::from_str::<serde_json::Value>(message).ok()?;
    let event = value.get("event")?.as_str()?.to_owned();
    if event != "script_execution_governance" {
        return None;
    }
    Some(ParsedLogGovernance {
        event,
        failure_class: value
            .get("failure_class")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        message: value
            .get("message")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    })
}
