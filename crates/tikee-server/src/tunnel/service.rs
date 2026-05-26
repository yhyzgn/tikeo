//! gRPC Worker Tunnel service.

use tikee_core::InstanceStatus;
use tikee_proto::worker::v1::{
    Heartbeat, Ping, ServerMessage, SubscribeTaskLogsRequest, TaskLog, TaskResult,
    UnregisterWorker, WorkerMessage, WorkerRegistered, server_message, worker_message,
    worker_tunnel_service_server::WorkerTunnelService,
};
use tikee_storage::{
    AppendJobInstanceLog, AuditLogRepository, JobInstanceAttemptRepository,
    JobInstanceLogRepository, JobInstanceLogSummary, JobInstanceRepository, WorkflowRepository,
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
    logs: JobInstanceLogRepository,
    attempts: JobInstanceAttemptRepository,
    workflows: WorkflowRepository,
    audit: AuditLogRepository,
    log_broadcaster: TaskLogBroadcaster,
}

impl WorkerTunnel {
    /// Create a Worker Tunnel service backed by an in-memory registry.
    #[must_use]
    pub const fn new(
        registry: WorkerRegistry,
        instances: JobInstanceRepository,
        logs: JobInstanceLogRepository,
        attempts: JobInstanceAttemptRepository,
        workflows: WorkflowRepository,
        audit: AuditLogRepository,
        log_broadcaster: TaskLogBroadcaster,
    ) -> Self {
        Self {
            registry,
            instances,
            logs,
            attempts,
            workflows,
            audit,
            log_broadcaster,
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
    register: tikee_proto::worker::v1::RegisterWorker,
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
        metrics::counter!("tikee_worker_stale_messages_total", "kind" => "task_log").increment(1);
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
        metrics::counter!("tikee_worker_stale_messages_total", "kind" => "task_result")
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
            handle_single_task_result(context, &worker_id, &instance_id, success, status).await;
        }
        Err(error) => {
            tracing::warn!(%error, %instance_id, %worker_id, "failed to persist attempt result");
        }
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
) {
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
                "worker {worker_id} reported task success={success}"
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

async fn refresh_broadcast_parent(
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    instance_id: &str,
) -> Result<(), tikee_storage::DbErr> {
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
    use tikee_core::InstanceStatus;
    use tikee_proto::worker::v1::{
        RegisterWorker, SubscribeTaskLogsRequest, TaskLog, WorkerMessage, server_message,
        worker_message,
    };
    use tikee_storage::{
        AuditLogRepository, JobInstanceAttemptRepository, JobInstanceLogRepository,
        JobInstanceRepository, WorkflowRepository, connect_and_migrate,
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
        let logs = logs().await;
        let audit = audit().await;
        let (tx, mut rx) = mpsc::channel(1);

        let attempts = attempts().await;

        let workflows = workflows().await;
        let log_broadcaster = TaskLogBroadcaster::default();
        let context = WorkerMessageContext {
            registry: &registry,
            instances: &instances,
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
        use tikee_core::{ExecutionMode, TriggerType};
        use tikee_proto::worker::v1::{RegisterWorker, TaskResult};
        use tikee_storage::{CreateJob, CreateJobInstance, JobRepository};

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
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "assign-token".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: None,
                script_id: None,
                enabled: true,
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
    async fn subscribe_task_logs_replays_existing_and_streams_live_logs() {
        use tikee_core::{ExecutionMode, TriggerType};
        use tikee_proto::worker::v1::worker_tunnel_service_server::WorkerTunnelService;
        use tikee_storage::{CreateJob, CreateJobInstance, JobRepository};
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
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "log-stream".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: None,
                script_id: None,
                enabled: true,
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
        logs.append(tikee_storage::AppendJobInstanceLog {
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
        let service = super::WorkerTunnel::new(
            WorkerRegistry::default(),
            instances,
            logs,
            attempts,
            workflows,
            audit,
            broadcaster.clone(),
        );
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
