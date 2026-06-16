//! Minimal pending-instance dispatcher for Worker Tunnel sessions.

use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

use tikeo_core::{
    ExecutionMode, InstanceStatus, ScriptExecutionPolicy, ScriptLanguage, ScriptPolicyError,
    ScriptStatus, WasmProcessorSpec,
};
use tikeo_proto::worker::v1::{
    DispatchTask, ScriptProcessorBinding, TaskProcessorBinding, WasmProcessorBinding,
    task_processor_binding,
};
use tikeo_storage::{
    AppendJobInstanceLog, AuditLogRepository, CreateWorkerDispatchOutbox,
    JobInstanceAttemptRepository, JobInstanceRepository, JobRepository, ScriptRepository,
    ScriptSummary, ScriptVersionSummary, WorkerDispatchOutboxRepository, WorkflowRepository,
};
use tokio::time;
use tonic_prost::prost::Message as _;
use tracing::{debug, warn};
use uuid::Uuid;

use super::{WorkerRegistry, capability::WorkerRequirement, governance};
use crate::{
    cluster::SharedClusterCoordinator,
    notification::{JobNotificationEvent, NotificationCenter, emit_job_instance_event_best_effort},
};

const DISPATCH_INTERVAL: Duration = Duration::from_millis(500);
const DISPATCH_BATCH_SIZE: u64 = 16;
const DISPATCH_LEASE_SECONDS: i64 = 30;
const DISPATCH_RETRY_BACKOFF_SECONDS: i64 = 2;
const DISPATCH_STALE_RUNNING_SECONDS: i64 = 60;
const DISPATCHER_LEASE_OWNER: &str = "tikeo-dispatcher";

fn new_assignment_token() -> String {
    format!("asg-{}", Uuid::now_v7())
}

fn encoded_dispatch_payload(task: &DispatchTask) -> String {
    BASE64_STANDARD.encode(task.encode_to_vec())
}

fn dispatcher_fencing_token(node_id: &str, leader_fencing_token: Option<&str>) -> String {
    leader_fencing_token.map_or_else(
        || format!("standalone:{node_id}:{DISPATCHER_LEASE_OWNER}"),
        |token| format!("raft:{node_id}:{token}"),
    )
}

/// Run the minimal single-node dispatch loop forever.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    jobs: JobRepository,
    instances: JobInstanceRepository,
    attempts: JobInstanceAttemptRepository,
    outbox: WorkerDispatchOutboxRepository,
    workflows: WorkflowRepository,
    scripts: ScriptRepository,
    logs: tikeo_storage::JobInstanceLogRepository,
    audit: AuditLogRepository,
    registry: WorkerRegistry,
    cluster: SharedClusterCoordinator,
    notifications: NotificationCenter,
) {
    let mut ticker = time::interval(DISPATCH_INTERVAL);
    loop {
        ticker.tick().await;
        if let Err(error) = dispatch_once_if_owner(
            &jobs,
            &instances,
            &attempts,
            &outbox,
            &workflows,
            &scripts,
            &logs,
            &audit,
            &registry,
            &cluster,
            &notifications,
        )
        .await
        {
            warn!(%error, "worker dispatch iteration failed");
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_once_if_owner(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    outbox: &WorkerDispatchOutboxRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    logs: &tikeo_storage::JobInstanceLogRepository,
    audit: &AuditLogRepository,
    registry: &WorkerRegistry,
    cluster: &SharedClusterCoordinator,
    notifications: &NotificationCenter,
) -> Result<(), tikeo_storage::DbErr> {
    let status = cluster.status().await;
    if !status.can_schedule {
        debug!(role = status.role.as_str(), node_id = %status.node_id, "skip worker dispatch without cluster ownership");
        return Ok(());
    }
    let fencing_token =
        dispatcher_fencing_token(&status.node_id, status.leader_fencing_token.as_deref());
    dispatch_once(
        jobs,
        instances,
        attempts,
        outbox,
        workflows,
        scripts,
        logs,
        audit,
        registry,
        &fencing_token,
        notifications,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_once(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    outbox: &WorkerDispatchOutboxRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    logs: &tikeo_storage::JobInstanceLogRepository,
    audit: &AuditLogRepository,
    registry: &WorkerRegistry,
    fencing_token: &str,
    notifications: &NotificationCenter,
) -> Result<(), tikeo_storage::DbErr> {
    let recovered = workflows
        .requeue_stale_running_job_dispatches(DISPATCH_STALE_RUNNING_SECONDS)
        .await?;
    if recovered > 0 {
        warn!(recovered, "requeued stale running job dispatches");
    }
    let _expired = workflows.clear_expired_dispatch_queue_leases().await?;
    if let Some(materialized) = workflows
        .materialize_next_queued_node_with_fencing(
            DISPATCHER_LEASE_OWNER,
            DISPATCH_LEASE_SECONDS,
            fencing_token,
        )
        .await?
    {
        crate::notification::emit_workflow_notification_node_requested_best_effort(
            notifications,
            workflows,
            &materialized,
        )
        .await;
    }
    dispatch_broadcast_attempts(
        jobs,
        instances,
        attempts,
        outbox,
        workflows,
        scripts,
        logs,
        audit,
        registry,
        notifications,
    )
    .await?;
    dispatch_single_instances(
        jobs,
        instances,
        attempts,
        outbox,
        workflows,
        scripts,
        logs,
        audit,
        registry,
        fencing_token,
        notifications,
    )
    .await
}

struct DurableDispatchIntent<'a> {
    instance_id: &'a str,
    attempt_id: &'a str,
    worker_id: &'a str,
    owner_fencing_token: &'a str,
    task: DispatchTask,
}

async fn persist_outbox_then_hint_dispatch(
    attempts: &JobInstanceAttemptRepository,
    outbox: &WorkerDispatchOutboxRepository,
    registry: &WorkerRegistry,
    mut intent: DurableDispatchIntent<'_>,
) -> Result<bool, tikeo_storage::DbErr> {
    let Some(target) = registry.dispatch_target(intent.worker_id).await else {
        return Ok(false);
    };
    let assignment_token = new_assignment_token();
    let _recorded = attempts
        .record_assignment_token(intent.instance_id, intent.worker_id, &assignment_token)
        .await?;
    intent.task.assignment_token.clone_from(&assignment_token);
    let _created = outbox
        .create(CreateWorkerDispatchOutbox {
            instance_id: intent.instance_id.to_owned(),
            attempt_id: intent.attempt_id.to_owned(),
            worker_id: target.worker_id.clone(),
            logical_instance_id: target.logical_instance_id,
            gateway_node_id: target.gateway_node_id,
            gateway_generation: target.generation,
            assignment_token,
            dispatch_payload: encoded_dispatch_payload(&intent.task),
            shard_id: 0,
            owner_node_id: DISPATCHER_LEASE_OWNER.to_owned(),
            owner_epoch: 0,
            owner_fencing_token: intent.owner_fencing_token.to_owned(),
            next_delivery_at: None,
        })
        .await?;
    Ok(registry
        .dispatch_tokened_to_worker(intent.worker_id, intent.task)
        .await)
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::large_stack_frames
)]
async fn dispatch_single_instances(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    outbox: &WorkerDispatchOutboxRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    logs: &tikeo_storage::JobInstanceLogRepository,
    audit: &AuditLogRepository,
    registry: &WorkerRegistry,
    fencing_token: &str,
    notifications: &NotificationCenter,
) -> Result<(), tikeo_storage::DbErr> {
    for _ in 0..DISPATCH_BATCH_SIZE {
        let Some(claim) = workflows
            .claim_next_job_queue_item_with_fencing(
                DISPATCHER_LEASE_OWNER,
                DISPATCH_LEASE_SECONDS,
                fencing_token,
            )
            .await?
        else {
            break;
        };
        let Some(instance_id) = claim.item.job_instance_id.clone() else {
            continue;
        };
        let Some(instance) = instances.get(&instance_id).await? else {
            let _ = workflows
                .mark_dispatch_queue_failed(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            warn!(queue_id = %claim.item.id, %instance_id, "closed dispatch queue item for missing job instance");
            continue;
        };
        let retrying_instance = matches!(
            instance.status,
            InstanceStatus::Running | InstanceStatus::Retrying
        ) && claim.item.attempt > 1;
        if instance.status != InstanceStatus::Pending && !retrying_instance {
            let _ = workflows
                .mark_dispatch_queue_done_by_instance(&instance.id)
                .await?;
            debug!(instance_id = %instance.id, status = %instance.status, "closed dispatch queue item for non-pending instance");
            continue;
        }
        if !retrying_instance && !instances.claim_pending_for_dispatch(&instance.id).await? {
            let _ = workflows
                .release_dispatch_queue_item(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            continue;
        }
        let Some(job) = jobs.get(&instance.job_id).await? else {
            let _ = workflows
                .mark_dispatch_queue_failed(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            if let Some(updated) = instances
                .update_status(&instance.id, InstanceStatus::Failed)
                .await?
            {
                emit_job_instance_event_best_effort(
                    notifications,
                    &updated,
                    JobNotificationEvent::Failed,
                    Some("missing job during dispatch"),
                )
                .await;
            }
            warn!(queue_id = %claim.item.id, instance_id = %instance.id, job_id = %instance.job_id, "closed dispatch queue item for missing job");
            continue;
        };

        let executor = resolve_job_executor(workflows, &instance.id, &job).await?;
        if let JobExecutor::Http { config } = &executor {
            append_retry_dispatch_progress_log(
                logs,
                &instance.id,
                claim.item.attempt,
                &job,
                "executor builtin.http",
            )
            .await?;
            let outcome = execute_http_processor(config).await;
            complete_builtin_processor_outcome(
                instances,
                workflows,
                logs,
                &job,
                &instance.id,
                claim.item.attempt,
                "builtin.http",
                outcome.success,
                outcome.message,
                notifications,
            )
            .await?;
            continue;
        }
        if let JobExecutor::Grpc { config } = &executor {
            append_retry_dispatch_progress_log(
                logs,
                &instance.id,
                claim.item.attempt,
                &job,
                "executor builtin.grpc",
            )
            .await?;
            let outcome = execute_grpc_processor(config).await;
            complete_builtin_processor_outcome(
                instances,
                workflows,
                logs,
                &job,
                &instance.id,
                claim.item.attempt,
                "builtin.grpc",
                outcome.success,
                outcome.message,
                notifications,
            )
            .await?;
            continue;
        }
        if let JobExecutor::Sql { config } = &executor {
            append_retry_dispatch_progress_log(
                logs,
                &instance.id,
                claim.item.attempt,
                &job,
                "executor builtin.sql",
            )
            .await?;
            let outcome = execute_sql_processor(config).await;
            complete_builtin_processor_outcome(
                instances,
                workflows,
                logs,
                &job,
                &instance.id,
                claim.item.attempt,
                "builtin.sql",
                outcome.success,
                outcome.message,
                notifications,
            )
            .await?;
            continue;
        }

        if let JobExecutor::FileCleanup { config } = &executor {
            append_retry_dispatch_progress_log(
                logs,
                &instance.id,
                claim.item.attempt,
                &job,
                "executor builtin.file_cleanup",
            )
            .await?;
            let outcome = execute_file_cleanup_processor(config).await;
            complete_builtin_processor_outcome(
                instances,
                workflows,
                logs,
                &job,
                &instance.id,
                claim.item.attempt,
                "builtin.file_cleanup",
                outcome.success,
                outcome.message,
                notifications,
            )
            .await?;
            continue;
        }
        let task = match build_dispatch_task(
            scripts,
            instance.id.clone(),
            instance.job_id.clone(),
            executor.clone(),
        )
        .await?
        {
            DispatchTaskBuild::Built(task) => task,
            DispatchTaskBuild::Rejected(failure) => {
                append_script_governance_log(logs, audit, &instance.id, &failure).await?;
                let _ = workflows
                    .mark_dispatch_queue_failed(&claim.item.id, DISPATCHER_LEASE_OWNER)
                    .await?;
                if let Some(updated) = instances
                    .update_status(&instance.id, InstanceStatus::Failed)
                    .await?
                {
                    emit_job_instance_event_best_effort(
                        notifications,
                        &updated,
                        JobNotificationEvent::ScriptGovernanceFailure,
                        Some(&failure.message()),
                    )
                    .await;
                    emit_job_instance_event_best_effort(
                        notifications,
                        &updated,
                        JobNotificationEvent::Failed,
                        Some(&failure.message()),
                    )
                    .await;
                }
                continue;
            }
        };

        let requirement = required_task_requirement_for_executor(&task, &executor);
        let eligible_workers = registry
            .find_ordered_persisted_dispatch_workers(&job.namespace, &job.app, requirement.as_ref())
            .await;
        if let Some(worker_id) = eligible_workers.first() {
            let created_attempts = attempts
                .create_pending_for_workers(&instance.id, std::slice::from_ref(worker_id))
                .await?;
            let Some(attempt) = created_attempts.first() else {
                let _ = workflows
                    .release_dispatch_queue_item_after(
                        &claim.item.id,
                        DISPATCHER_LEASE_OWNER,
                        DISPATCH_RETRY_BACKOFF_SECONDS,
                    )
                    .await?;
                instances
                    .update_status(&instance.id, InstanceStatus::Pending)
                    .await?;
                continue;
            };
            let dispatch_hint_sent = persist_outbox_then_hint_dispatch(
                attempts,
                outbox,
                registry,
                DurableDispatchIntent {
                    instance_id: &instance.id,
                    attempt_id: &attempt.id,
                    worker_id,
                    owner_fencing_token: fencing_token,
                    task,
                },
            )
            .await?;
            if !dispatch_hint_sent {
                debug!(%worker_id, instance_id = %instance.id, "dispatch hint failed after durable outbox was queued");
            }
            attempts
                .update_status_if_current(
                    &instance.id,
                    worker_id,
                    InstanceStatus::Pending,
                    InstanceStatus::Running,
                )
                .await?;
            append_retry_dispatch_progress_log(
                logs,
                &instance.id,
                claim.item.attempt,
                &job,
                &format!("worker {worker_id}"),
            )
            .await?;
            let instance_marked_running = if retrying_instance {
                instances
                    .update_status_if_current(
                        &instance.id,
                        InstanceStatus::Retrying,
                        InstanceStatus::Running,
                    )
                    .await?
            } else {
                instances
                    .update_status_if_current(
                        &instance.id,
                        InstanceStatus::Dispatching,
                        InstanceStatus::Running,
                    )
                    .await?
            };
            if instance_marked_running && let Some(updated) = instances.get(&instance.id).await? {
                emit_job_instance_event_best_effort(
                    notifications,
                    &updated,
                    JobNotificationEvent::Running,
                    Some(&format!("dispatched to worker {worker_id}")),
                )
                .await;
            }
            let _ = workflows
                .mark_dispatch_queue_running(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            debug!(%worker_id, instance_id = %instance.id, "dispatched instance to worker");
        } else {
            if let Some(requirement) = requirement.as_ref() {
                append_script_governance_log(
                    logs,
                    audit,
                    &instance.id,
                    &ScriptGovernanceFailure::NoEligibleWorkerCapability(
                        requirement.display_label(),
                    ),
                )
                .await?;
                let _ = workflows
                    .mark_dispatch_queue_failed(&claim.item.id, DISPATCHER_LEASE_OWNER)
                    .await?;
                if let Some(updated) = instances
                    .update_status(&instance.id, InstanceStatus::Failed)
                    .await?
                {
                    let reason = ScriptGovernanceFailure::NoEligibleWorkerCapability(
                        requirement.display_label(),
                    )
                    .message();
                    emit_job_instance_event_best_effort(
                        notifications,
                        &updated,
                        JobNotificationEvent::NoEligibleWorker,
                        Some(&reason),
                    )
                    .await;
                    emit_job_instance_event_best_effort(
                        notifications,
                        &updated,
                        JobNotificationEvent::ScriptGovernanceFailure,
                        Some(&reason),
                    )
                    .await;
                }
                continue;
            }
            let _ = workflows
                .release_dispatch_queue_item_after(
                    &claim.item.id,
                    DISPATCHER_LEASE_OWNER,
                    DISPATCH_RETRY_BACKOFF_SECONDS,
                )
                .await?;
            instances
                .update_status(&instance.id, InstanceStatus::Pending)
                .await?;
        }
    }

    Ok(())
}

async fn append_retry_dispatch_progress_log(
    logs: &tikeo_storage::JobInstanceLogRepository,
    instance_id: &str,
    attempt: i32,
    job: &tikeo_storage::JobSummary,
    target: &str,
) -> Result<(), tikeo_storage::DbErr> {
    if attempt <= 1 {
        return Ok(());
    }
    append_dispatcher_execution_log(
        logs,
        instance_id,
        "tikeo-retry",
        "info",
        &format!(
            "retry attempt {}/{} dispatching to {target}",
            attempt, job.retry_policy.max_attempts
        ),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn complete_builtin_processor_outcome(
    instances: &JobInstanceRepository,
    workflows: &WorkflowRepository,
    logs: &tikeo_storage::JobInstanceLogRepository,
    job: &tikeo_storage::JobSummary,
    instance_id: &str,
    attempt: i32,
    worker_id: &str,
    success: bool,
    message: String,
    notifications: &NotificationCenter,
) -> Result<(), tikeo_storage::DbErr> {
    instances
        .record_result(instance_id, worker_id, success, &message)
        .await?;
    append_dispatcher_execution_log(
        logs,
        instance_id,
        worker_id,
        if success { "info" } else { "error" },
        &format!("task result success={success} message={message}"),
    )
    .await?;
    if !success && job.retry_policy.allows_retry_after_attempt(attempt) {
        let delay_seconds = job.retry_policy.delay_after_attempt_seconds(attempt);
        if let Some(requeued) = workflows
            .requeue_dispatch_queue_for_retry(instance_id, delay_seconds)
            .await?
        {
            append_dispatcher_execution_log(
                logs,
                instance_id,
                "tikeo-retry",
                "info",
                &format!(
                    "retry scheduled: completed attempt {}/{} failed on {worker_id}; next attempt after {}s at {}; result={message}",
                    attempt, job.retry_policy.max_attempts, delay_seconds, requeued.run_after
                ),
            )
            .await?;
            if let Some(updated) = instances.get(instance_id).await? {
                emit_job_instance_event_best_effort(
                    notifications,
                    &updated,
                    JobNotificationEvent::RetryScheduled,
                    Some(&message),
                )
                .await;
            }
            return Ok(());
        }
    } else if !success {
        append_dispatcher_execution_log(
            logs,
            instance_id,
            "tikeo-retry",
            "error",
            &format!(
                "retry exhausted after attempt {}/{}; final failure from {worker_id}: {message}",
                attempt, job.retry_policy.max_attempts
            ),
        )
        .await?;
    }
    let status = if success {
        InstanceStatus::Succeeded
    } else {
        InstanceStatus::Failed
    };
    let _ = workflows
        .mark_dispatch_queue_done_by_instance(instance_id)
        .await?;
    if let Some(updated) = instances.update_status(instance_id, status).await? {
        let event = if !success && status == InstanceStatus::Failed {
            Some(terminal_failure_notification_event(job, attempt))
        } else {
            JobNotificationEvent::from_terminal_status(status)
        };
        if let Some(event) = event {
            emit_job_instance_event_best_effort(notifications, &updated, event, Some(&message))
                .await;
        }
    }
    let _ = workflows
        .complete_job_node_from_result(instance_id, status, Some(message))
        .await?;
    Ok(())
}

fn terminal_failure_notification_event(
    job: &tikeo_storage::JobSummary,
    attempt: i32,
) -> JobNotificationEvent {
    let policy = job.retry_policy.clone().normalized();
    if policy.enabled && policy.max_attempts > 1 && attempt >= policy.max_attempts {
        JobNotificationEvent::RetryExhausted
    } else {
        JobNotificationEvent::Failed
    }
}

async fn append_dispatcher_execution_log(
    logs: &tikeo_storage::JobInstanceLogRepository,
    instance_id: &str,
    worker_id: &str,
    level: &str,
    message: &str,
) -> Result<(), tikeo_storage::DbErr> {
    let sequence = logs
        .count_by_instance(instance_id)
        .await
        .map_or(0, |count| i64::try_from(count).unwrap_or(i64::MAX - 1) + 1);
    let _ = logs
        .append(AppendJobInstanceLog {
            instance_id: instance_id.to_owned(),
            worker_id: worker_id.to_owned(),
            level: level.to_owned(),
            message: message.to_owned(),
            sequence,
        })
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_broadcast_attempts(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    outbox: &WorkerDispatchOutboxRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    logs: &tikeo_storage::JobInstanceLogRepository,
    audit: &AuditLogRepository,
    registry: &WorkerRegistry,
    notifications: &NotificationCenter,
) -> Result<(), tikeo_storage::DbErr> {
    let pending = attempts.list_pending(DISPATCH_BATCH_SIZE).await?;

    for attempt in pending {
        let Some(instance) = instances.get(&attempt.instance_id).await? else {
            continue;
        };
        if instance.execution_mode != ExecutionMode::Broadcast {
            continue;
        }
        let executor = if let Some(job) = jobs.get(&instance.job_id).await? {
            resolve_job_executor(workflows, &instance.id, &job).await?
        } else {
            JobExecutor::SdkProcessor {
                processor_name: instance.job_id.clone(),
                processor_type: None,
            }
        };
        let task = match build_dispatch_task(
            scripts,
            attempt.instance_id.clone(),
            instance.job_id.clone(),
            executor.clone(),
        )
        .await?
        {
            DispatchTaskBuild::Built(task) => task,
            DispatchTaskBuild::Rejected(failure) => {
                append_script_governance_log(logs, audit, &attempt.instance_id, &failure).await?;
                attempts
                    .update_status(
                        &attempt.instance_id,
                        &attempt.worker_id,
                        InstanceStatus::Failed,
                    )
                    .await?;
                continue;
            }
        };

        let requirement = required_task_requirement_for_executor(&task, &executor);
        if !registry
            .persisted_worker_supports_requirement(&attempt.worker_id, requirement.as_ref())
            .await
        {
            if let Some(requirement) = requirement.as_ref() {
                append_script_governance_log(
                    logs,
                    audit,
                    &attempt.instance_id,
                    &ScriptGovernanceFailure::NoEligibleWorkerCapability(
                        requirement.display_label(),
                    ),
                )
                .await?;
                attempts
                    .update_status(
                        &attempt.instance_id,
                        &attempt.worker_id,
                        InstanceStatus::Failed,
                    )
                    .await?;
            }
            continue;
        }

        let dispatch_hint_sent = persist_outbox_then_hint_dispatch(
            attempts,
            outbox,
            registry,
            DurableDispatchIntent {
                instance_id: &attempt.instance_id,
                attempt_id: &attempt.id,
                worker_id: &attempt.worker_id,
                owner_fencing_token: DISPATCHER_LEASE_OWNER,
                task,
            },
        )
        .await?;
        if dispatch_hint_sent {
            attempts
                .update_status_if_current(
                    &attempt.instance_id,
                    &attempt.worker_id,
                    InstanceStatus::Pending,
                    InstanceStatus::Running,
                )
                .await?;
            let instance_marked_running = instances
                .update_status_if_current(
                    &attempt.instance_id,
                    InstanceStatus::Pending,
                    InstanceStatus::Running,
                )
                .await?;
            if instance_marked_running
                && let Some(updated) = instances.get(&attempt.instance_id).await?
            {
                emit_job_instance_event_best_effort(
                    notifications,
                    &updated,
                    JobNotificationEvent::Running,
                    Some(&format!(
                        "dispatched broadcast attempt to worker {}",
                        attempt.worker_id
                    )),
                )
                .await;
            }
            debug!(worker_id = %attempt.worker_id, instance_id = %attempt.instance_id, "dispatched broadcast attempt to worker");
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DispatchTaskBuild {
    Built(DispatchTask),
    Rejected(ScriptGovernanceFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScriptGovernanceFailure {
    MissingScript,
    NotApproved,
    MissingReleasePointer,
    MissingReleasedVersion,
    UnsupportedLanguage,
    PolicyRejected(String),
    NoEligibleWorkerCapability(String),
}

impl ScriptGovernanceFailure {
    const fn code(&self) -> &'static str {
        match self {
            Self::MissingScript => "script_missing",
            Self::NotApproved => "script_not_approved",
            Self::MissingReleasePointer => "script_missing_release_pointer",
            Self::MissingReleasedVersion => "script_missing_released_version",
            Self::UnsupportedLanguage => "script_unsupported_language",
            Self::PolicyRejected(_) => "script_policy_rejected",
            Self::NoEligibleWorkerCapability(_) => "script_no_eligible_worker_capability",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::MissingScript => "script governance rejected dispatch: script definition is missing".to_owned(),
            Self::NotApproved => "script governance rejected dispatch: script is not approved".to_owned(),
            Self::MissingReleasePointer => {
                "script governance rejected dispatch: approved script has no released version pointer"
                    .to_owned()
            }
            Self::MissingReleasedVersion => {
                "script governance rejected dispatch: released script version is missing".to_owned()
            }
            Self::UnsupportedLanguage => {
                "script governance rejected dispatch: script language is unsupported".to_owned()
            }
            Self::PolicyRejected(reason) => {
                format!("script governance rejected dispatch: policy rejected ({reason})")
            }
            Self::NoEligibleWorkerCapability(capability) => format!(
                "script governance failed dispatch: no connected worker advertises required capability {capability}"
            ),
        }
    }
}

async fn append_script_governance_log(
    logs: &tikeo_storage::JobInstanceLogRepository,
    audit: &AuditLogRepository,
    instance_id: &str,
    failure: &ScriptGovernanceFailure,
) -> Result<(), tikeo_storage::DbErr> {
    let failure_class = failure.code();
    let message = failure.message();
    let payload = governance::script_governance_payload(failure_class, &message);
    let _ = logs
        .append(AppendJobInstanceLog {
            instance_id: instance_id.to_owned(),
            worker_id: "tikeo-dispatcher".to_owned(),
            level: "warn".to_owned(),
            message: payload.to_string(),
            sequence: 0,
        })
        .await?;
    governance::materialize_script_governance_audit(
        audit,
        "tikeo-dispatcher",
        instance_id,
        failure_class,
        &message,
    )
    .await?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum JobExecutor {
    SdkProcessor {
        processor_name: String,
        processor_type: Option<String>,
    },
    Script {
        script_id: String,
    },
    Http {
        config: serde_json::Value,
    },
    Grpc {
        config: serde_json::Value,
    },
    Sql {
        config: serde_json::Value,
    },
    FileCleanup {
        config: serde_json::Value,
    },
}

async fn build_dispatch_task(
    scripts: &ScriptRepository,
    instance_id: String,
    job_id: String,
    executor: JobExecutor,
) -> Result<DispatchTaskBuild, tikeo_storage::DbErr> {
    let (processor_name, processor_binding) = match executor {
        JobExecutor::Script { script_id } => {
            let Some(script) = scripts.get(&script_id).await? else {
                warn!(%script_id, "script processor binding references missing script; dispatch remains pending");
                return Ok(DispatchTaskBuild::Rejected(
                    ScriptGovernanceFailure::MissingScript,
                ));
            };
            if !script_is_dispatchable(&script) {
                warn!(script_id = %script.id, language = %script.language, status = %script.status, "script is not dispatchable; dispatch remains pending");
                return Ok(DispatchTaskBuild::Rejected(
                    ScriptGovernanceFailure::NotApproved,
                ));
            }
            let Some(version_number) = script.released_version_number else {
                warn!(script_id = %script.id, "approved script has no released version pointer; dispatch remains pending");
                return Ok(DispatchTaskBuild::Rejected(
                    ScriptGovernanceFailure::MissingReleasePointer,
                ));
            };
            let Some(version) = scripts
                .versions()
                .get_version_by_number(&script.id, version_number)
                .await?
            else {
                warn!(script_id = %script.id, version_number, "released script version is missing; dispatch remains pending");
                return Ok(DispatchTaskBuild::Rejected(
                    ScriptGovernanceFailure::MissingReleasedVersion,
                ));
            };
            let Some(language) = parse_script_language(&version.language) else {
                warn!(script_id = %script.id, language = %version.language, "released script version has unsupported language; dispatch remains pending");
                return Ok(DispatchTaskBuild::Rejected(
                    ScriptGovernanceFailure::UnsupportedLanguage,
                ));
            };
            if let Err(error) =
                validate_script_version_dispatchable(&version, script.release_grants.as_ref())
            {
                warn!(script_id = %script.id, version_number, language = %version.language, %error, "released script version policy is not dispatchable; dispatch remains pending");
                return Ok(DispatchTaskBuild::Rejected(
                    ScriptGovernanceFailure::PolicyRejected(error.to_string()),
                ));
            }

            (
                script.id.clone(),
                if language == ScriptLanguage::Wasm {
                    Some(Box::new(wasm_processor_binding(&script, &version)))
                } else {
                    Some(Box::new(script_processor_binding(&script, &version)))
                },
            )
        }
        JobExecutor::SdkProcessor { processor_name, .. } => (processor_name, None),
        JobExecutor::Http { .. } => ("builtin.http".to_owned(), None),
        JobExecutor::Grpc { .. } => ("builtin.grpc".to_owned(), None),
        JobExecutor::Sql { .. } => ("builtin.sql".to_owned(), None),
        JobExecutor::FileCleanup { .. } => ("builtin.file_cleanup".to_owned(), None),
    };

    Ok(DispatchTaskBuild::Built(DispatchTask {
        instance_id,
        job_id,
        payload: Vec::new(),
        processor_name,
        processor_binding,
        assignment_token: String::new(),
    }))
}

fn required_task_requirement_for_executor(
    task: &DispatchTask,
    executor: &JobExecutor,
) -> Option<WorkerRequirement> {
    match executor {
        JobExecutor::SdkProcessor {
            processor_name,
            processor_type: Some(processor_type),
        } if !processor_type.trim().is_empty() && processor_type != "sdk" => {
            Some(WorkerRequirement::PluginProcessor {
                processor_type: processor_type.trim().to_owned(),
                processor_name: processor_name.trim().to_owned(),
            })
        }
        JobExecutor::SdkProcessor { processor_name, .. } => Some(WorkerRequirement::SdkProcessor {
            name: processor_name.trim().to_owned(),
        }),
        JobExecutor::Script { .. } => required_task_requirement(task),
        JobExecutor::Http { .. }
        | JobExecutor::Grpc { .. }
        | JobExecutor::Sql { .. }
        | JobExecutor::FileCleanup { .. } => None,
    }
}

fn required_task_requirement(task: &DispatchTask) -> Option<WorkerRequirement> {
    let binding = task.processor_binding.as_ref()?;
    match binding.kind.as_ref()? {
        task_processor_binding::Kind::Wasm(_) => Some(WorkerRequirement::ScriptRunner {
            language: "wasm".to_owned(),
        }),
        task_processor_binding::Kind::Script(script) => Some(WorkerRequirement::ScriptRunner {
            language: script.language.trim().to_owned(),
        }),
    }
}

fn script_is_dispatchable(script: &ScriptSummary) -> bool {
    script.status == ScriptStatus::Approved.as_str()
        && parse_script_language(&script.language).is_some()
}

#[cfg(test)]
fn script_version_is_dispatchable(version: &ScriptVersionSummary) -> bool {
    validate_script_version_dispatchable(version, None).is_ok()
}

fn validate_script_version_dispatchable(
    version: &ScriptVersionSummary,
    release_grants: Option<&tikeo_storage::ScriptReleaseGrantEvidenceSummary>,
) -> Result<(), ScriptDispatchValidationError> {
    match parse_script_language(&version.language) {
        Some(ScriptLanguage::Wasm) => script_version_to_wasm_spec(version)
            .validate()
            .map_err(|error| ScriptDispatchValidationError(error.to_string())),
        Some(
            ScriptLanguage::Shell
            | ScriptLanguage::Python
            | ScriptLanguage::Js
            | ScriptLanguage::Ts
            | ScriptLanguage::PowerShell
            | ScriptLanguage::Php
            | ScriptLanguage::Groovy
            | ScriptLanguage::Rhai,
        ) => validate_script_policy_for_dispatch(
            &script_policy(version.policy.clone()),
            release_grants,
        ),
        None => Err(ScriptDispatchValidationError(
            "script language is unsupported".to_owned(),
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScriptDispatchValidationError(String);

impl From<ScriptPolicyError> for ScriptDispatchValidationError {
    fn from(value: ScriptPolicyError) -> Self {
        Self(value.to_string())
    }
}

impl std::fmt::Display for ScriptDispatchValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn parse_script_language(language: &str) -> Option<ScriptLanguage> {
    language.parse::<ScriptLanguage>().ok()
}

fn validate_script_policy_for_dispatch(
    policy: &ScriptExecutionPolicy,
    release_grants: Option<&tikeo_storage::ScriptReleaseGrantEvidenceSummary>,
) -> Result<(), ScriptDispatchValidationError> {
    if policy.resources.timeout_ms == 0 {
        return Err(ScriptDispatchValidationError(
            "script timeout must be greater than zero".to_owned(),
        ));
    }
    if policy.resources.max_memory_bytes == 0 {
        return Err(ScriptDispatchValidationError(
            "script memory limit must be greater than zero".to_owned(),
        ));
    }
    if policy.resources.max_output_bytes == 0 {
        return Err(ScriptDispatchValidationError(
            "script output limit must be greater than zero".to_owned(),
        ));
    }
    if release_grants.is_none() {
        policy
            .validate_default_deny()
            .map_err(ScriptDispatchValidationError::from)?;
    }
    Ok(())
}

fn script_processor_binding(
    script: &ScriptSummary,
    version: &ScriptVersionSummary,
) -> TaskProcessorBinding {
    let policy = script_policy(version.policy.clone());
    let release_grants = script.release_grants.as_ref();
    let language = parse_script_language(&version.language).map_or_else(
        || version.language.clone(),
        |language| language.as_str().to_owned(),
    );
    TaskProcessorBinding {
        kind: Some(task_processor_binding::Kind::Script(
            ScriptProcessorBinding {
                script_id: script.id.clone(),
                version: script.version.clone(),
                language,
                content: version.content.as_bytes().to_vec(),
                version_id: version.id.clone(),
                version_number: u64::try_from(version.version_number).unwrap_or_default(),
                content_sha256: version.content_sha256.clone(),
                timeout_ms: policy.resources.timeout_ms,
                max_memory_bytes: policy.resources.max_memory_bytes,
                max_output_bytes: policy.resources.max_output_bytes,
                allow_network: policy.network.enabled
                    || release_grants.is_some_and(|grants| !grants.url.is_empty()),
                allowed_env_vars: policy.env_vars,
                read_only_paths: release_grants
                    .map(|grants| grants.file_read.clone())
                    .unwrap_or(policy.filesystem.read_only_paths),
                writable_paths: release_grants
                    .map(|grants| grants.file_write.clone())
                    .unwrap_or(policy.filesystem.writable_paths),
                secret_refs: release_grants
                    .map(|grants| grants.secret.clone())
                    .unwrap_or(policy.secrets.refs),
                allowed_network_hosts: release_grants
                    .map(|grants| grants.url.clone())
                    .unwrap_or(policy.network.allowed_hosts),
                sandbox_backend: policy.sandbox.backend.as_str().to_owned(),
            },
        )),
    }
}

fn script_policy(value: serde_json::Value) -> ScriptExecutionPolicy {
    serde_json::from_value(value).unwrap_or_default()
}

fn wasm_processor_binding(
    script: &ScriptSummary,
    version: &ScriptVersionSummary,
) -> TaskProcessorBinding {
    let spec = script_version_to_wasm_spec(version);
    TaskProcessorBinding {
        kind: Some(task_processor_binding::Kind::Wasm(WasmProcessorBinding {
            script_id: script.id.clone(),
            version: script.version.clone(),
            module: version.content.as_bytes().to_vec(),
            runtime: spec.runtime.as_str().to_owned(),
            entrypoint: spec.entrypoint,
            timeout_ms: spec.resources.timeout_ms,
            max_memory_bytes: spec.resources.max_memory_bytes,
            fuel: spec.resources.fuel,
            allow_network: spec.capabilities.network,
            allowed_env_vars: spec.capabilities.env_vars,
            version_id: version.id.clone(),
            version_number: u64::try_from(version.version_number).unwrap_or_default(),
            module_sha256: version.content_sha256.clone(),
            module_signature: String::new(),
        })),
    }
}

fn script_version_to_wasm_spec(version: &ScriptVersionSummary) -> WasmProcessorSpec {
    let mut spec = WasmProcessorSpec::default();
    spec.resources.timeout_ms = version
        .timeout_seconds
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
        .map_or(spec.resources.timeout_ms, |seconds| {
            seconds.saturating_mul(1000)
        });
    spec.resources.max_memory_bytes = version
        .max_memory_bytes
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(spec.resources.max_memory_bytes);
    spec.capabilities.network = version.allow_network;
    spec.capabilities.env_vars = version.allowed_env_vars.clone().unwrap_or_default();
    spec
}

async fn resolve_job_executor(
    workflows: &WorkflowRepository,
    instance_id: &str,
    job: &tikeo_storage::JobSummary,
) -> Result<JobExecutor, tikeo_storage::DbErr> {
    if let Some(binding) = workflows.job_binding_for_instance(instance_id).await? {
        if binding.node_kind == "http" {
            return Ok(JobExecutor::Http {
                config: binding.config.unwrap_or_else(|| serde_json::json!({})),
            });
        }
        if binding.node_kind == "grpc" {
            return Ok(JobExecutor::Grpc {
                config: binding.config.unwrap_or_else(|| serde_json::json!({})),
            });
        }
        if binding.node_kind == "sql" {
            return Ok(JobExecutor::Sql {
                config: binding.config.unwrap_or_else(|| serde_json::json!({})),
            });
        }
        if binding.node_kind == "file_cleanup" {
            return Ok(JobExecutor::FileCleanup {
                config: binding.config.unwrap_or_else(|| serde_json::json!({})),
            });
        }
        if let Some(processor_name) = binding.processor_name {
            return Ok(JobExecutor::SdkProcessor {
                processor_name,
                processor_type: None,
            });
        }
    }
    if let Some(script_id) = job
        .script_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(JobExecutor::Script {
            script_id: script_id.to_owned(),
        });
    }
    Ok(JobExecutor::SdkProcessor {
        processor_name: job
            .processor_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&job.name)
            .to_owned(),
        processor_type: job.processor_type.clone(),
    })
}

mod processors;
use processors::{
    execute_file_cleanup_processor, execute_grpc_processor, execute_http_processor,
    execute_sql_processor,
};
#[cfg(test)]
mod tests;
