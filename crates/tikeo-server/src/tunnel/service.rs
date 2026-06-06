//! gRPC Worker Tunnel service.

use tikeo_core::InstanceStatus;
use tikeo_proto::worker::v1::{
    Heartbeat, Ping, ServerMessage, SubscribeTaskLogsRequest, TaskCheckpoint, TaskLog, TaskResult,
    UnregisterWorker, WorkerMessage, WorkerRegistered, server_message, worker_message,
    worker_tunnel_service_server::WorkerTunnelService,
};
use tikeo_storage::{
    AppendJobInstanceLog, AuditLogRepository, JobInstanceAttemptRepository,
    JobInstanceLogRepository, JobInstanceLogSummary, JobInstanceRepository, JobRepository,
    WorkflowRepository,
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
    workflows: WorkflowRepository,
    audit: AuditLogRepository,
    log_broadcaster: TaskLogBroadcaster,
}

impl WorkerTunnel {
    /// Create a Worker Tunnel service backed by an in-memory registry.
    #[must_use]
    pub fn new(runtime: super::WorkerTunnelRuntime) -> Self {
        Self {
            registry: runtime.registry,
            instances: runtime.instances,
            jobs: runtime.jobs,
            logs: runtime.logs,
            attempts: runtime.attempts,
            workflows: runtime.workflows,
            audit: runtime.audit,
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
        let workflows = self.workflows.clone();
        let audit = self.audit.clone();
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
                            workflows: &workflows,
                            audit: &audit,
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
    workflows: &'a WorkflowRepository,
    audit: &'a AuditLogRepository,
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
        .registry
        .accepts_worker_assignment(&worker_id, &assignment_token)
        .await
    {
        metrics::counter!("tikeo_worker_stale_messages_total", "kind" => "task_log").increment(1);
        return Ok(WorkerMessageOutcome::Continue);
    }
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
        .registry
        .accepts_worker_assignment(&worker_id, &assignment_token)
        .await
    {
        metrics::counter!("tikeo_worker_stale_messages_total", "kind" => "task_checkpoint")
            .increment(1);
        return;
    }
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
        .registry
        .accepts_worker_assignment(&worker_id, &assignment_token)
        .await
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
    match context
        .attempts
        .update_status(&instance_id, &worker_id, status)
        .await
    {
        Ok(Some(_)) => {
            persist_broadcast_task_result(context, &worker_id, &instance_id, success, &message)
                .await;
            if let Err(error) =
                refresh_broadcast_parent(context.instances, context.attempts, &instance_id).await
            {
                tracing::warn!(%error, %instance_id, "failed to refresh broadcast parent status");
            }
        }
        Ok(None) => {
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
        Err(error) => {
            tracing::warn!(%error, %instance_id, %worker_id, "failed to persist attempt result");
        }
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

    if let Err(error) = context.instances.update_status(instance_id, status).await {
        tracing::warn!(%error, %instance_id, "failed to persist task result");
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
            true
        }
        Ok(None) => false,
        Err(error) => {
            tracing::warn!(%error, %instance_id, "failed to requeue failed task for retry");
            false
        }
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
    instance_id: &str,
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
    let _ = instances.update_status(instance_id, status).await?;
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
mod tests {
    use tikeo_core::InstanceStatus;
    use tikeo_proto::worker::v1::{
        RegisterWorker, SubscribeTaskLogsRequest, TaskLog, WorkerMessage, server_message,
        worker_message,
    };
    use tikeo_storage::{
        AuditLogRepository, JobInstanceAttemptRepository, JobInstanceLogRepository,
        JobInstanceRepository, JobRepository, WorkflowRepository, connect_and_migrate,
    };
    use tokio::sync::mpsc;
    use tokio_stream::StreamExt;

    use super::{
        TaskLogBroadcaster, WorkerMessageContext, WorkerRegistry, handle_task_result,
        handle_worker_message,
    };

    #[tokio::test]
    async fn register_message_updates_registry_and_acknowledges_worker() {
        let registry = WorkerRegistry::default();
        let instances = instances().await;
        let jobs = jobs().await;
        let logs = logs().await;
        let audit = audit().await;
        let (tx, mut rx) = mpsc::channel(1);

        let attempts = attempts().await;

        let workflows = workflows().await;
        let log_broadcaster = TaskLogBroadcaster::default();
        let context = WorkerMessageContext {
            registry: &registry,
            instances: &instances,
            jobs: &jobs,
            logs: &logs,
            attempts: &attempts,
            workflows: &workflows,
            audit: &audit,
            log_broadcaster: &log_broadcaster,
            tx: &tx,
            outbound: &tx,
        };

        handle_worker_message(
            &context,
            WorkerMessage {
                kind: Some(worker_message::Kind::Register(RegisterWorker {
                    client_instance_id: "worker-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "finance".to_owned(),
                    cluster: "prod".to_owned(),
                    region: "cn".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: None,
                    election: None,
                    labels: std::collections::HashMap::default(),
                })),
            },
        )
        .await
        .unwrap_or_else(|error| panic!("ack should send: {error}"));

        let ack = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("ack should exist"))
            .unwrap_or_else(|error| panic!("ack should be ok: {error}"));

        match ack.kind {
            Some(server_message::Kind::Registered(registered)) => {
                assert!(registered.worker_id.starts_with("wrk-"));
            }
            other => panic!("unexpected ack: {other:?}"),
        }

        let registered_id = registry
            .worker_ids()
            .await
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("registered worker id should exist"));
        assert!(registry.get(&registered_id).await.is_some());
    }

    #[tokio::test]
    async fn task_result_with_wrong_assignment_token_is_rejected() {
        use tikeo_core::{ExecutionMode, TriggerType};
        use tikeo_proto::worker::v1::{RegisterWorker, TaskResult};
        use tikeo_storage::{CreateJob, CreateJobInstance, JobRepository};

        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let logs = JobInstanceLogRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "assign-token".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        let (outbound, _rx) = mpsc::channel(8);
        let registry = WorkerRegistry::default();
        let worker = registry
            .register(
                RegisterWorker {
                    client_instance_id: "assign-worker".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: None,
                    election: None,
                    labels: std::collections::HashMap::default(),
                },
                outbound,
            )
            .await;
        let (tx, _events) = mpsc::channel(8);
        let broadcaster = TaskLogBroadcaster::default();
        let context = WorkerMessageContext {
            registry: &registry,
            instances: &instances,
            jobs: &jobs,
            logs: &logs,
            attempts: &attempts,
            workflows: &workflows,
            audit: &audit,
            log_broadcaster: &broadcaster,
            tx: &tx,
            outbound: &tx,
        };

        handle_task_result(
            &context,
            TaskResult {
                worker_id: worker.worker_id,
                instance_id: instance.id.clone(),
                success: true,
                message: "ok".to_owned(),
                assignment_token: "wrong".to_owned(),
            },
        )
        .await;

        let unchanged = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(unchanged.status, InstanceStatus::Pending);
    }

    #[tokio::test]
    async fn broadcast_task_result_persists_per_worker_attempt_result() {
        use tikeo_core::{ExecutionMode, InstanceStatus, TriggerType};
        use tikeo_proto::worker::v1::{DispatchTask, RegisterWorker, TaskResult, server_message};
        use tikeo_storage::{CreateJob, CreateJobInstance, JobRepository};

        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let logs = JobInstanceLogRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let audit = AuditLogRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "broadcast-result".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("demo.broadcast".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Broadcast,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));

        let (outbound, mut rx) = mpsc::channel(8);
        let registry = WorkerRegistry::default();
        let worker = registry
            .register(
                RegisterWorker {
                    client_instance_id: "broadcast-worker".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: None,
                    election: None,
                    labels: std::collections::HashMap::default(),
                },
                outbound,
            )
            .await;
        attempts
            .create_pending_for_workers(&instance.id, std::slice::from_ref(&worker.worker_id))
            .await
            .unwrap_or_else(|error| panic!("attempt should create: {error}"));
        registry
            .dispatch_to_worker(
                &worker.worker_id,
                DispatchTask {
                    instance_id: instance.id.clone(),
                    job_id: job.id,
                    payload: Vec::new(),
                    processor_name: "demo.broadcast".to_owned(),
                    processor_binding: None,
                    assignment_token: String::new(),
                },
            )
            .await
            .unwrap_or_else(|| panic!("task should dispatch"));
        let token = match rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("dispatch should arrive"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"))
            .kind
        {
            Some(server_message::Kind::DispatchTask(task)) => task.assignment_token,
            other => panic!("unexpected server message: {other:?}"),
        };
        let (tx, _events) = mpsc::channel(8);
        let broadcaster = TaskLogBroadcaster::default();
        let context = WorkerMessageContext {
            registry: &registry,
            instances: &instances,
            jobs: &jobs,
            logs: &logs,
            attempts: &attempts,
            workflows: &workflows,
            audit: &audit,
            log_broadcaster: &broadcaster,
            tx: &tx,
            outbound: &tx,
        };

        handle_task_result(
            &context,
            TaskResult {
                worker_id: worker.worker_id.clone(),
                instance_id: instance.id.clone(),
                success: true,
                message: "broadcast ok".to_owned(),
                assignment_token: token,
            },
        )
        .await;

        let persisted_attempts = attempts
            .list_by_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("attempts should load: {error}"));
        let result = persisted_attempts[0]
            .result
            .clone()
            .unwrap_or_else(|| panic!("attempt result should persist"));
        assert!(result.success);
        assert_eq!(result.worker_id, worker.worker_id);
        assert_eq!(result.message, "broadcast ok");

        let parent = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(parent.status, InstanceStatus::Succeeded);
    }

    #[tokio::test]
    async fn failed_single_task_result_schedules_retry_and_logs_result() {
        use tikeo_core::{ExecutionMode, InstanceStatus, TriggerType};
        use tikeo_proto::worker::v1::{DispatchTask, RegisterWorker, TaskResult, server_message};
        use tikeo_storage::{CreateJob, CreateJobInstance, JobRepository, JobRetryPolicy};

        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let logs = JobInstanceLogRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let audit = AuditLogRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "retry-runtime".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("demo.retry".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: Some(JobRetryPolicy {
                    enabled: true,
                    max_attempts: 3,
                    initial_delay_seconds: 5,
                    backoff_multiplier: 2,
                    max_delay_seconds: 60,
                }),
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        let claim = workflows
            .claim_next_job_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("queue should claim: {error}"))
            .unwrap_or_else(|| panic!("queue item should exist"));
        workflows
            .mark_dispatch_queue_running(&claim.item.id, "server-a")
            .await
            .unwrap_or_else(|error| panic!("queue should run: {error}"));
        instances
            .update_status(&instance.id, InstanceStatus::Running)
            .await
            .unwrap_or_else(|error| panic!("instance should run: {error}"));

        let (outbound, mut rx) = mpsc::channel(8);
        let registry = WorkerRegistry::default();
        let worker = registry
            .register(
                RegisterWorker {
                    client_instance_id: "retry-worker".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: None,
                    election: None,
                    labels: std::collections::HashMap::default(),
                },
                outbound,
            )
            .await;
        registry
            .dispatch_to_worker(
                &worker.worker_id,
                DispatchTask {
                    instance_id: instance.id.clone(),
                    job_id: job.id,
                    payload: Vec::new(),
                    processor_name: "demo.retry".to_owned(),
                    processor_binding: None,
                    assignment_token: String::new(),
                },
            )
            .await
            .unwrap_or_else(|| panic!("task should dispatch"));
        let token = match rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("dispatch should arrive"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"))
            .kind
        {
            Some(server_message::Kind::DispatchTask(task)) => task.assignment_token,
            other => panic!("unexpected server message: {other:?}"),
        };
        let (tx, _events) = mpsc::channel(8);
        let broadcaster = TaskLogBroadcaster::default();
        let context = WorkerMessageContext {
            registry: &registry,
            instances: &instances,
            jobs: &jobs,
            logs: &logs,
            attempts: &attempts,
            workflows: &workflows,
            audit: &audit,
            log_broadcaster: &broadcaster,
            tx: &tx,
            outbound: &tx,
        };

        handle_task_result(
            &context,
            TaskResult {
                worker_id: worker.worker_id.clone(),
                instance_id: instance.id.clone(),
                success: false,
                message: "runtime failed with exit 2".to_owned(),
                assignment_token: token,
            },
        )
        .await;

        let requeued_instance = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(requeued_instance.status, InstanceStatus::Pending);
        let result = requeued_instance
            .result
            .unwrap_or_else(|| panic!("result should persist"));
        assert!(!result.success);
        assert_eq!(result.message, "runtime failed with exit 2");

        let requeued = workflows
            .claim_next_job_queue_item("server-b", 30)
            .await
            .unwrap_or_else(|error| panic!("queue should load: {error}"));
        assert!(requeued.is_none(), "retry should wait for backoff");
        let persisted_logs = logs
            .list_by_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("logs should load: {error}"));
        assert!(
            persisted_logs
                .iter()
                .any(|log| log.message.contains("retry scheduled"))
        );
        assert!(
            persisted_logs
                .iter()
                .any(|log| log.message.contains("runtime failed with exit 2"))
        );
    }

    #[tokio::test]
    async fn subscribe_task_logs_replays_existing_and_streams_live_logs() {
        use tikeo_core::{ExecutionMode, TriggerType};
        use tikeo_proto::worker::v1::worker_tunnel_service_server::WorkerTunnelService;
        use tikeo_storage::{CreateJob, CreateJobInstance, JobRepository};
        use tonic::Request;

        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let logs = JobInstanceLogRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "log-stream".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        logs.append(tikeo_storage::AppendJobInstanceLog {
            instance_id: instance.id.clone(),
            worker_id: "wrk-existing".to_owned(),
            level: "INFO".to_owned(),
            message: "existing".to_owned(),
            sequence: 1,
        })
        .await
        .unwrap_or_else(|error| panic!("existing log should append: {error}"));

        let audit = audit().await;
        let broadcaster = TaskLogBroadcaster::default();
        let service = super::WorkerTunnel::new(crate::tunnel::WorkerTunnelRuntime::new(
            crate::tunnel::WorkerTunnelRuntimeParts {
                registry: WorkerRegistry::default(),
                instances,
                jobs,
                logs,
                attempts,
                workflows,
                audit,
                log_broadcaster: broadcaster.clone(),
            },
        ));
        let response = service
            .subscribe_task_logs(Request::new(SubscribeTaskLogsRequest {
                instance_id: instance.id.clone(),
                after_sequence: 0,
                replay_existing: true,
            }))
            .await
            .unwrap_or_else(|error| panic!("subscription should start: {error}"));
        let mut stream = response.into_inner();
        let replayed = stream
            .next()
            .await
            .unwrap_or_else(|| panic!("replayed log should exist"))
            .unwrap_or_else(|error| panic!("replay should stream: {error}"));
        assert_eq!(replayed.message, "existing");

        broadcaster.publish(TaskLog {
            worker_id: "wrk-live".to_owned(),
            instance_id: instance.id,
            level: "INFO".to_owned(),
            message: "live".to_owned(),
            sequence: 2,
            assignment_token: String::new(),
        });
        let live = stream
            .next()
            .await
            .unwrap_or_else(|| panic!("live log should exist"))
            .unwrap_or_else(|error| panic!("live log should stream: {error}"));
        assert_eq!(live.message, "live");
    }

    async fn jobs() -> JobRepository {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        JobRepository::new(db)
    }

    async fn instances() -> JobInstanceRepository {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        JobInstanceRepository::new(db)
    }

    async fn attempts() -> JobInstanceAttemptRepository {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        JobInstanceAttemptRepository::new(db)
    }

    async fn workflows() -> WorkflowRepository {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        WorkflowRepository::new(db)
    }

    async fn logs() -> JobInstanceLogRepository {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        JobInstanceLogRepository::new(db)
    }

    async fn audit() -> AuditLogRepository {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        AuditLogRepository::new(db)
    }
}
