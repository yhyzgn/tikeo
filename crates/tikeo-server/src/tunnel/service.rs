//! gRPC Worker Tunnel service.

use crate::notification::{
    JobNotificationEvent, NotificationCenter, emit_job_instance_event_best_effort,
};
use tikeo_core::{ExecutionMode, InstanceStatus};
use tikeo_proto::worker::v1::{
    Heartbeat, Ping, ServerMessage, SubscribeTaskLogsRequest, TaskCheckpoint, TaskLog, TaskResult,
    UnregisterWorker, WorkerMessage, WorkerRegistered, server_message, worker_message,
    worker_tunnel_service_server::WorkerTunnelService,
};
use tikeo_storage::{
    AppendJobInstanceLog, AuditLogRepository, JobInstanceAttemptRepository,
    JobInstanceLogRepository, JobInstanceLogSummary, JobInstanceRepository, JobRepository,
    WorkerDispatchOutboxRepository, WorkflowRepository,
};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

use super::{WorkerRegistry, governance};

const DEFAULT_LEASE_SECONDS: u64 = 30;
const LOG_STREAM_BUFFER: usize = 256;

/// Broadcast bus for live task logs keyed by subscribers on instance id.
#[derive(Debug, Clone)]
pub struct TaskLogBroadcaster {
    tx: broadcast::Sender<TaskLog>,
}

impl Default for TaskLogBroadcaster {
    fn default() -> Self {
        let (tx, _rx) = broadcast::channel(LOG_STREAM_BUFFER);
        Self { tx }
    }
}

impl TaskLogBroadcaster {
    fn subscribe(&self) -> broadcast::Receiver<TaskLog> {
        self.tx.subscribe()
    }

    fn publish(&self, log: TaskLog) {
        let _ = self.tx.send(log);
    }
}

/// Worker Tunnel gRPC service implementation.
#[derive(Debug, Clone)]
pub struct WorkerTunnel {
    registry: WorkerRegistry,
    instances: JobInstanceRepository,
    jobs: JobRepository,
    logs: JobInstanceLogRepository,
    attempts: JobInstanceAttemptRepository,
    outbox: WorkerDispatchOutboxRepository,
    workflows: WorkflowRepository,
    audit: AuditLogRepository,
    notifications: NotificationCenter,
    log_broadcaster: TaskLogBroadcaster,
}

impl WorkerTunnel {
    /// Create a Worker Tunnel service backed by an in-memory registry.
    #[must_use]
    /// New.
    pub fn new(runtime: super::WorkerTunnelRuntime) -> Self {
        Self {
            registry: runtime.registry,
            instances: runtime.instances,
            jobs: runtime.jobs,
            logs: runtime.logs,
            attempts: runtime.attempts,
            outbox: runtime.outbox,
            workflows: runtime.workflows,
            audit: runtime.audit,
            notifications: runtime.notifications,
            log_broadcaster: runtime.log_broadcaster,
        }
    }
}

#[tonic::async_trait]
impl WorkerTunnelService for WorkerTunnel {
    type OpenTunnelStream = ReceiverStream<Result<ServerMessage, Status>>;
    type SubscribeTaskLogsStream = ReceiverStream<Result<TaskLog, Status>>;

    async fn open_tunnel(
        &self,
        request: Request<Streaming<WorkerMessage>>,
    ) -> Result<Response<Self::OpenTunnelStream>, Status> {
        let mut inbound = request.into_inner();
        let registry = self.registry.clone();
        let instances = self.instances.clone();
        let jobs = self.jobs.clone();
        let logs = self.logs.clone();
        let attempts = self.attempts.clone();
        let outbox = self.outbox.clone();
        let workflows = self.workflows.clone();
        let audit = self.audit.clone();
        let notifications = self.notifications.clone();
        let log_broadcaster = self.log_broadcaster.clone();
        let (tx, rx) = mpsc::channel(16);
        let outbound = tx.clone();

        tokio::spawn(async move {
            let mut registered_worker_id: Option<String> = None;
            let mut graceful_unregister = false;
            while let Some(message) = inbound.message().await.transpose() {
                match message {
                    Ok(message) => {
                        let context = WorkerMessageContext {
                            registry: &registry,
                            instances: &instances,
                            jobs: &jobs,
                            logs: &logs,
                            attempts: &attempts,
                            outbox: &outbox,
                            workflows: &workflows,
                            audit: &audit,
                            notifications: &notifications,
                            log_broadcaster: &log_broadcaster,
                            tx: &tx,
                            outbound: &outbound,
                        };
                        match handle_worker_message(&context, message).await {
                            Ok(WorkerMessageOutcome::Registered(worker_id)) => {
                                registered_worker_id = Some(worker_id);
                            }
                            Ok(WorkerMessageOutcome::GracefulUnregister) => {
                                graceful_unregister = true;
                            }
                            Ok(WorkerMessageOutcome::Continue) => {}
                            Err(_) => break,
                        }
                    }
                    Err(status) => {
                        if let Some(worker_id) = registered_worker_id.as_deref() {
                            registry
                                .mark_transport_error(
                                    worker_id,
                                    &format!("worker tunnel stream error: {status}"),
                                )
                                .await;
                        }
                        let _ = tx.send(Err(status)).await;
                        return;
                    }
                }
            }
            if !graceful_unregister && let Some(worker_id) = registered_worker_id.as_deref() {
                registry
                    .mark_transport_error(
                        worker_id,
                        "worker tunnel stream ended before graceful unregister",
                    )
                    .await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn subscribe_task_logs(
        &self,
        request: Request<SubscribeTaskLogsRequest>,
    ) -> Result<Response<Self::SubscribeTaskLogsStream>, Status> {
        let request = request.into_inner();
        let instance_id = request.instance_id.trim().to_owned();
        if instance_id.is_empty() {
            return Err(Status::invalid_argument("instance_id is required"));
        }
        let logs = self.logs.clone();
        let mut live = self.log_broadcaster.subscribe();
        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(async move {
            if request.replay_existing {
                match logs
                    .list_by_instance_after_sequence(&instance_id, request.after_sequence)
                    .await
                {
                    Ok(existing) => {
                        for log in existing {
                            if tx.send(Ok(task_log_from_summary(log))).await.is_err() {
                                return;
                            }
                        }
                    }
                    Err(error) => {
                        let _ = tx
                            .send(Err(Status::internal(format!(
                                "failed to replay task logs: {error}"
                            ))))
                            .await;
                        return;
                    }
                }
            }

            loop {
                match live.recv().await {
                    Ok(log)
                        if log.instance_id == instance_id
                            && log.sequence > request.after_sequence =>
                    {
                        if tx.send(Ok(log)).await.is_err() {
                            return;
                        }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        let _ = tx
                            .send(Err(Status::data_loss(format!(
                                "task log stream lagged by {skipped} messages"
                            ))))
                            .await;
                        return;
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

struct WorkerMessageContext<'a> {
    registry: &'a WorkerRegistry,
    instances: &'a JobInstanceRepository,
    jobs: &'a JobRepository,
    logs: &'a JobInstanceLogRepository,
    attempts: &'a JobInstanceAttemptRepository,
    outbox: &'a WorkerDispatchOutboxRepository,
    workflows: &'a WorkflowRepository,
    audit: &'a AuditLogRepository,
    notifications: &'a NotificationCenter,
    log_broadcaster: &'a TaskLogBroadcaster,
    tx: &'a mpsc::Sender<Result<ServerMessage, Status>>,
    outbound: &'a mpsc::Sender<Result<ServerMessage, Status>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WorkerMessageOutcome {
    Continue,
    Registered(String),
    GracefulUnregister,
}

async fn handle_worker_message(
    context: &WorkerMessageContext<'_>,
    message: WorkerMessage,
) -> Result<WorkerMessageOutcome, mpsc::error::SendError<Result<ServerMessage, Status>>> {
    match message.kind {
        Some(worker_message::Kind::Register(register)) => handle_register(context, register).await,
        Some(worker_message::Kind::Heartbeat(heartbeat)) => {
            handle_heartbeat(context, heartbeat).await
        }
        Some(worker_message::Kind::Unregister(unregister)) => {
            handle_unregister(context, unregister).await
        }
        Some(worker_message::Kind::TaskResult(result)) => {
            handle_task_result(context, result).await;
            Ok(WorkerMessageOutcome::Continue)
        }
        Some(worker_message::Kind::TaskLog(log)) => handle_task_log(context, log).await,
        Some(worker_message::Kind::TaskCheckpoint(checkpoint)) => {
            handle_task_checkpoint(context, checkpoint).await;
            Ok(WorkerMessageOutcome::Continue)
        }
        None => {
            context
                .tx
                .send(Err(Status::invalid_argument(
                    "worker message kind is required",
                )))
                .await?;
            Ok(WorkerMessageOutcome::Continue)
        }
    }
}

async fn handle_register(
    context: &WorkerMessageContext<'_>,
    register: tikeo_proto::worker::v1::RegisterWorker,
) -> Result<WorkerMessageOutcome, mpsc::error::SendError<Result<ServerMessage, Status>>> {
    let worker = context
        .registry
        .register(register, context.outbound.clone())
        .await;
    let worker_id = worker.worker_id.clone();
    context
        .tx
        .send(Ok(ServerMessage {
            kind: Some(server_message::Kind::Registered(WorkerRegistered {
                worker_id: worker.worker_id,
                lease_seconds: DEFAULT_LEASE_SECONDS,
                generation: worker.generation,
                fencing_token: worker.fencing_token,
            })),
        }))
        .await?;
    Ok(WorkerMessageOutcome::Registered(worker_id))
}

async fn handle_heartbeat(
    context: &WorkerMessageContext<'_>,
    heartbeat: Heartbeat,
) -> Result<WorkerMessageOutcome, mpsc::error::SendError<Result<ServerMessage, Status>>> {
    let Heartbeat {
        worker_id,
        sequence,
        generation,
        fencing_token,
    } = heartbeat;
    if context
        .registry
        .heartbeat(&worker_id, sequence, generation, &fencing_token)
        .await
        .is_some()
    {
        context
            .tx
            .send(Ok(ServerMessage {
                kind: Some(server_message::Kind::Ping(Ping { sequence })),
            }))
            .await?;
    } else {
        context
            .tx
            .send(Err(Status::failed_precondition(
                "stale worker heartbeat rejected",
            )))
            .await?;
    }
    Ok(WorkerMessageOutcome::Continue)
}

async fn handle_unregister(
    context: &WorkerMessageContext<'_>,
    unregister: UnregisterWorker,
) -> Result<WorkerMessageOutcome, mpsc::error::SendError<Result<ServerMessage, Status>>> {
    let UnregisterWorker {
        worker_id,
        generation,
        fencing_token,
    } = unregister;
    if context
        .registry
        .unregister(&worker_id, generation, &fencing_token)
        .await
        .is_some()
    {
        Ok(WorkerMessageOutcome::GracefulUnregister)
    } else {
        context
            .tx
            .send(Err(Status::failed_precondition(
                "stale worker unregister rejected",
            )))
            .await?;
        Ok(WorkerMessageOutcome::Continue)
    }
}

async fn handle_task_log(
    context: &WorkerMessageContext<'_>,
    log: TaskLog,
) -> Result<WorkerMessageOutcome, mpsc::error::SendError<Result<ServerMessage, Status>>> {
    let TaskLog {
        worker_id,
        instance_id,
        level,
        message,
        sequence,
        assignment_token,
    } = log;
    if !context
        .attempts
        .accepts_assignment_token(&instance_id, &worker_id, &assignment_token)
        .await
        .unwrap_or(false)
    {
        metrics::counter!("tikeo_worker_stale_messages_total", "kind" => "task_log").increment(1);
        return Ok(WorkerMessageOutcome::Continue);
    }
    let _ = context
        .outbox
        .mark_acked_by_assignment(&instance_id, &worker_id, &assignment_token)
        .await
        .map_err(|error| tracing::warn!(%error, "failed to ack worker dispatch outbox row from task log"));
    match context
        .logs
        .append(AppendJobInstanceLog {
            instance_id,
            worker_id,
            level,
            message,
            sequence,
        })
        .await
    {
        Ok(Some(saved)) => context
            .log_broadcaster
            .publish(task_log_from_summary(saved)),
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, "failed to persist task log"),
    }
    Ok(WorkerMessageOutcome::Continue)
}

async fn handle_task_checkpoint(context: &WorkerMessageContext<'_>, checkpoint: TaskCheckpoint) {
    let TaskCheckpoint {
        worker_id,
        instance_id,
        checkpoint_json,
        sequence,
        assignment_token,
    } = checkpoint;
    if !context
        .attempts
        .accepts_assignment_token(&instance_id, &worker_id, &assignment_token)
        .await
        .unwrap_or(false)
    {
        metrics::counter!("tikeo_worker_stale_messages_total", "kind" => "task_checkpoint")
            .increment(1);
        return;
    }
    let _ = context
        .outbox
        .mark_acked_by_assignment(&instance_id, &worker_id, &assignment_token)
        .await
        .map_err(|error| tracing::warn!(%error, "failed to ack worker dispatch outbox row from task checkpoint"));
    if let Err(error) = context
        .logs
        .append(AppendJobInstanceLog {
            instance_id,
            worker_id,
            level: "checkpoint".to_owned(),
            message: checkpoint_json,
            sequence,
        })
        .await
    {
        tracing::warn!(%error, "failed to persist task checkpoint");
    }
}

async fn handle_task_result(context: &WorkerMessageContext<'_>, result: TaskResult) {
    let TaskResult {
        worker_id,
        instance_id,
        success,
        message,
        assignment_token,
    } = result;
    if !context
        .attempts
        .accepts_assignment_token(&instance_id, &worker_id, &assignment_token)
        .await
        .unwrap_or(false)
    {
        metrics::counter!("tikeo_worker_stale_messages_total", "kind" => "task_result")
            .increment(1);
        return;
    }
    let status = if success {
        InstanceStatus::Succeeded
    } else {
        InstanceStatus::Failed
    };
    let _ = context
        .outbox
        .mark_completed_by_assignment(&instance_id, &worker_id, &assignment_token)
        .await
        .map_err(|error| tracing::warn!(%error, "failed to complete worker dispatch outbox row"));
    let execution_mode = context
        .instances
        .get(&instance_id)
        .await
        .ok()
        .flatten()
        .map_or(ExecutionMode::Single, |instance| instance.execution_mode);
    if execution_mode == ExecutionMode::Broadcast {
        match context
            .attempts
            .update_status(&instance_id, &worker_id, status)
            .await
        {
            Ok(Some(_)) => {
                persist_broadcast_task_result(context, &worker_id, &instance_id, success, &message)
                    .await;
                if let Err(error) = refresh_broadcast_parent(
                    context.instances,
                    context.attempts,
                    context.notifications,
                    &instance_id,
                    &message,
                )
                .await
                {
                    tracing::warn!(%error, %instance_id, "failed to refresh broadcast parent status");
                }
            }
            Ok(None) => {
                metrics::counter!("tikeo_worker_stale_messages_total", "kind" => "task_result_missing_attempt")
                    .increment(1);
            }
            Err(error) => {
                tracing::warn!(%error, %instance_id, %worker_id, "failed to persist attempt result");
            }
        }
    } else {
        persist_script_result_governance(
            context.logs,
            context.audit,
            &worker_id,
            &instance_id,
            &message,
        )
        .await;
        handle_single_task_result(context, &worker_id, &instance_id, success, status, &message)
            .await;
    }
}

async fn persist_broadcast_task_result(
    context: &WorkerMessageContext<'_>,
    worker_id: &str,
    instance_id: &str,
    success: bool,
    message: &str,
) {
    if let Err(error) = context
        .attempts
        .record_result(instance_id, worker_id, success, message)
        .await
    {
        tracing::warn!(%error, %instance_id, %worker_id, "failed to persist broadcast attempt result");
    }
    append_execution_log(
        context,
        instance_id,
        worker_id,
        if success { "info" } else { "error" },
        &format!("task result success={success} message={message}"),
    )
    .await;
    if !success {
        persist_script_result_governance(
            context.logs,
            context.audit,
            worker_id,
            instance_id,
            message,
        )
        .await;
    }
}

async fn persist_script_result_governance(
    logs: &JobInstanceLogRepository,
    audit: &AuditLogRepository,
    worker_id: &str,
    instance_id: &str,
    message: &str,
) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(message) else {
        return;
    };
    let Some(failure_class) = value
        .get("failure_class")
        .and_then(serde_json::Value::as_str)
    else {
        return;
    };
    let governance_message = value
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(message);
    let payload = governance::script_governance_payload(failure_class, governance_message);
    if let Err(error) = logs
        .append(AppendJobInstanceLog {
            instance_id: instance_id.to_owned(),
            worker_id: worker_id.to_owned(),
            level: "warn".to_owned(),
            message: payload.to_string(),
            sequence: 0,
        })
        .await
    {
        tracing::warn!(%error, %instance_id, %worker_id, "failed to persist script governance result log");
    }
    if let Err(error) = governance::materialize_script_governance_audit(
        audit,
        worker_id,
        instance_id,
        failure_class,
        governance_message,
    )
    .await
    {
        tracing::warn!(%error, %instance_id, %worker_id, "failed to persist script governance audit log");
    }
}

async fn handle_single_task_result(
    context: &WorkerMessageContext<'_>,
    worker_id: &str,
    instance_id: &str,
    success: bool,
    status: InstanceStatus,
    message: &str,
) {
    if let Err(error) = context
        .instances
        .record_result(instance_id, worker_id, success, message)
        .await
    {
        tracing::warn!(%error, %instance_id, "failed to persist concrete task result");
    }
    append_execution_log(
        context,
        instance_id,
        worker_id,
        if success { "info" } else { "error" },
        &format!("task result success={success} message={message}"),
    )
    .await;

    if !success && schedule_retry_after_failure(context, worker_id, instance_id, message).await {
        return;
    }

    match context.instances.update_status(instance_id, status).await {
        Ok(Some(updated)) => {
            let event = if !success && status == InstanceStatus::Failed {
                terminal_failure_notification_event(context, instance_id).await
            } else {
                JobNotificationEvent::from_terminal_status(status)
            };
            if let Some(event) = event {
                emit_job_instance_event_best_effort(
                    context.notifications,
                    &updated,
                    event,
                    Some(message),
                )
                .await;
            }
        }
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, %instance_id, "failed to persist task result"),
    }
    if let Err(error) = context
        .workflows
        .mark_dispatch_queue_done_by_instance(instance_id)
        .await
    {
        tracing::warn!(%error, %instance_id, "failed to close dispatch queue item after task result");
    }
    match context
        .workflows
        .complete_job_node_from_result(
            instance_id,
            status,
            Some(format!(
                "worker {worker_id} reported task success={success}: {message}"
            )),
        )
        .await
    {
        Ok(Some(outcome)) => {
            tracing::info!(
                workflow_instance_id = %outcome.workflow_instance_id,
                node_key = %outcome.node_key,
                status = %outcome.status,
                queued_nodes = ?outcome.queued_nodes,
                completed = outcome.completed,
                "workflow node advanced from worker task result"
            );
        }
        Ok(None) => {}
        Err(error) => {
            tracing::warn!(%error, %instance_id, "failed to advance workflow from task result");
        }
    }
}

async fn schedule_retry_after_failure(
    context: &WorkerMessageContext<'_>,
    worker_id: &str,
    instance_id: &str,
    message: &str,
) -> bool {
    let Ok(Some(instance)) = context.instances.get(instance_id).await else {
        return false;
    };
    let Ok(Some(job)) = context.jobs.get(&instance.job_id).await else {
        return false;
    };
    let Ok(Some(queue)) = context
        .workflows
        .dispatch_queue_for_instance(instance_id)
        .await
    else {
        return false;
    };
    if !job.retry_policy.allows_retry_after_attempt(queue.attempt) {
        append_execution_log(
            context,
            instance_id,
            "tikeo-retry",
            "error",
            &format!(
                "retry exhausted after attempt {}/{}; final failure from worker {worker_id}: {message}",
                queue.attempt, job.retry_policy.max_attempts
            ),
        )
        .await;
        return false;
    }
    let delay_seconds = job.retry_policy.delay_after_attempt_seconds(queue.attempt);
    match context
        .workflows
        .requeue_dispatch_queue_for_retry(instance_id, delay_seconds)
        .await
    {
        Ok(Some(requeued)) => {
            append_execution_log(
                context,
                instance_id,
                "tikeo-retry",
                "info",
                &format!(
                    "retry scheduled: completed attempt {}/{} failed on worker {worker_id}; next attempt after {}s at {}; result={message}",
                    queue.attempt, job.retry_policy.max_attempts, delay_seconds, requeued.run_after
                ),
            )
            .await;
            if let Ok(Some(updated)) = context.instances.get(instance_id).await {
                emit_job_instance_event_best_effort(
                    context.notifications,
                    &updated,
                    JobNotificationEvent::RetryScheduled,
                    Some(message),
                )
                .await;
            }
            true
        }
        Ok(None) => false,
        Err(error) => {
            tracing::warn!(%error, %instance_id, "failed to requeue failed task for retry");
            false
        }
    }
}

async fn terminal_failure_notification_event(
    context: &WorkerMessageContext<'_>,
    instance_id: &str,
) -> Option<JobNotificationEvent> {
    let Ok(Some(instance)) = context.instances.get(instance_id).await else {
        return Some(JobNotificationEvent::Failed);
    };
    let Ok(Some(job)) = context.jobs.get(&instance.job_id).await else {
        return Some(JobNotificationEvent::Failed);
    };
    let Ok(Some(queue)) = context
        .workflows
        .dispatch_queue_for_instance(instance_id)
        .await
    else {
        return Some(JobNotificationEvent::Failed);
    };
    let policy = job.retry_policy.normalized();
    if policy.enabled && policy.max_attempts > 1 && queue.attempt >= policy.max_attempts {
        Some(JobNotificationEvent::RetryExhausted)
    } else {
        Some(JobNotificationEvent::Failed)
    }
}

async fn append_execution_log(
    context: &WorkerMessageContext<'_>,
    instance_id: &str,
    worker_id: &str,
    level: &str,
    message: &str,
) {
    let sequence = context
        .logs
        .count_by_instance(instance_id)
        .await
        .map_or(0, |count| i64::try_from(count).unwrap_or(i64::MAX - 1) + 1);
    match context
        .logs
        .append(AppendJobInstanceLog {
            instance_id: instance_id.to_owned(),
            worker_id: worker_id.to_owned(),
            level: level.to_owned(),
            message: message.to_owned(),
            sequence,
        })
        .await
    {
        Ok(Some(saved)) => context
            .log_broadcaster
            .publish(task_log_from_summary(saved)),
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, %instance_id, "failed to append execution log"),
    }
}

async fn refresh_broadcast_parent(
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    notifications: &NotificationCenter,
    instance_id: &str,
    message: &str,
) -> Result<(), tikeo_storage::DbErr> {
    let children = attempts.list_by_instance(instance_id).await?;
    if children.is_empty() {
        return Ok(());
    }
    let all_done = children.iter().all(|attempt| {
        matches!(
            attempt.status,
            InstanceStatus::Succeeded | InstanceStatus::Failed
        )
    });
    if !all_done {
        return Ok(());
    }
    let status = if children
        .iter()
        .all(|attempt| attempt.status == InstanceStatus::Succeeded)
    {
        InstanceStatus::Succeeded
    } else {
        InstanceStatus::PartialFailed
    };
    let Some(current) = instances.get(instance_id).await? else {
        return Ok(());
    };
    if current.status == status {
        return Ok(());
    }
    if let Some(updated) = instances.update_status(instance_id, status).await?
        && let Some(event) = JobNotificationEvent::from_terminal_status(status)
    {
        emit_job_instance_event_best_effort(
            notifications,
            &updated,
            event,
            Some(&format!(
                "broadcast parent completed after worker result: {message}"
            )),
        )
        .await;
    }
    Ok(())
}

fn task_log_from_summary(value: JobInstanceLogSummary) -> TaskLog {
    TaskLog {
        worker_id: value.worker_id,
        instance_id: value.instance_id,
        level: value.level,
        message: value.message,
        sequence: value.sequence,
        assignment_token: String::new(),
    }
}

#[cfg(test)]
mod tests;
