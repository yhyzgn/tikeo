//! gRPC Worker Tunnel service.

use scheduler_core::InstanceStatus;
use scheduler_proto::worker::v1::{
    Heartbeat, Ping, ServerMessage, TaskLog, TaskResult, WorkerMessage, WorkerRegistered,
    server_message, worker_message, worker_tunnel_service_server::WorkerTunnelService,
};
use scheduler_storage::{
    AppendJobInstanceLog, JobInstanceAttemptRepository, JobInstanceLogRepository,
    JobInstanceRepository,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

use super::WorkerRegistry;

const DEFAULT_LEASE_SECONDS: u64 = 30;

/// Worker Tunnel gRPC service implementation.
#[derive(Debug, Clone)]
pub struct WorkerTunnel {
    registry: WorkerRegistry,
    instances: JobInstanceRepository,
    logs: JobInstanceLogRepository,
    attempts: JobInstanceAttemptRepository,
}

impl WorkerTunnel {
    /// Create a Worker Tunnel service backed by an in-memory registry.
    #[must_use]
    pub const fn new(
        registry: WorkerRegistry,
        instances: JobInstanceRepository,
        logs: JobInstanceLogRepository,
        attempts: JobInstanceAttemptRepository,
    ) -> Self {
        Self {
            registry,
            instances,
            logs,
            attempts,
        }
    }
}

#[tonic::async_trait]
impl WorkerTunnelService for WorkerTunnel {
    type OpenTunnelStream = ReceiverStream<Result<ServerMessage, Status>>;

    async fn open_tunnel(
        &self,
        request: Request<Streaming<WorkerMessage>>,
    ) -> Result<Response<Self::OpenTunnelStream>, Status> {
        let mut inbound = request.into_inner();
        let registry = self.registry.clone();
        let instances = self.instances.clone();
        let logs = self.logs.clone();
        let attempts = self.attempts.clone();
        let (tx, rx) = mpsc::channel(16);
        let outbound = tx.clone();

        tokio::spawn(async move {
            while let Some(message) = inbound.message().await.transpose() {
                match message {
                    Ok(message) => {
                        if handle_worker_message(
                            &registry, &instances, &logs, &attempts, message, &tx, &outbound,
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                    }
                    Err(status) => {
                        let _ = tx.send(Err(status)).await;
                        break;
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

async fn handle_worker_message(
    registry: &WorkerRegistry,
    instances: &JobInstanceRepository,
    logs: &JobInstanceLogRepository,
    attempts: &JobInstanceAttemptRepository,
    message: WorkerMessage,
    tx: &mpsc::Sender<Result<ServerMessage, Status>>,
    outbound: &mpsc::Sender<Result<ServerMessage, Status>>,
) -> Result<(), mpsc::error::SendError<Result<ServerMessage, Status>>> {
    match message.kind {
        Some(worker_message::Kind::Register(register)) => {
            let worker = registry.register(register, outbound.clone()).await;
            tx.send(Ok(ServerMessage {
                kind: Some(server_message::Kind::Registered(WorkerRegistered {
                    worker_id: worker.worker_id,
                    lease_seconds: DEFAULT_LEASE_SECONDS,
                })),
            }))
            .await
        }
        Some(worker_message::Kind::Heartbeat(Heartbeat {
            worker_id,
            sequence,
        })) => {
            let _ = registry.heartbeat(&worker_id, sequence).await;
            tx.send(Ok(ServerMessage {
                kind: Some(server_message::Kind::Ping(Ping { sequence })),
            }))
            .await
        }
        Some(worker_message::Kind::TaskResult(TaskResult {
            worker_id,
            instance_id,
            success,
            ..
        })) => {
            let status = if success {
                InstanceStatus::Succeeded
            } else {
                InstanceStatus::Failed
            };
            match attempts
                .update_status(&instance_id, &worker_id, status)
                .await
            {
                Ok(Some(_)) => {
                    if let Err(error) =
                        refresh_broadcast_parent(instances, attempts, &instance_id).await
                    {
                        tracing::warn!(%error, %instance_id, "failed to refresh broadcast parent status");
                    }
                }
                Ok(None) => {
                    if let Err(error) = instances.update_status(&instance_id, status).await {
                        tracing::warn!(%error, %instance_id, "failed to persist task result");
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, %instance_id, %worker_id, "failed to persist attempt result");
                }
            }
            Ok(())
        }
        Some(worker_message::Kind::TaskLog(TaskLog {
            worker_id,
            instance_id,
            level,
            message,
            sequence,
        })) => {
            if let Err(error) = logs
                .append(AppendJobInstanceLog {
                    instance_id,
                    worker_id,
                    level,
                    message,
                    sequence,
                })
                .await
            {
                tracing::warn!(%error, "failed to persist task log");
            }
            Ok(())
        }
        None => {
            tx.send(Err(Status::invalid_argument(
                "worker message kind is required",
            )))
            .await
        }
    }
}

async fn refresh_broadcast_parent(
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    instance_id: &str,
) -> Result<(), scheduler_storage::DbErr> {
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

#[cfg(test)]
mod tests {
    use scheduler_proto::worker::v1::{
        RegisterWorker, WorkerMessage, server_message, worker_message,
    };
    use scheduler_storage::{
        JobInstanceAttemptRepository, JobInstanceLogRepository, JobInstanceRepository,
        connect_and_migrate,
    };
    use tokio::sync::mpsc;

    use super::{WorkerRegistry, handle_worker_message};

    #[tokio::test]
    async fn register_message_updates_registry_and_acknowledges_worker() {
        let registry = WorkerRegistry::default();
        let instances = instances().await;
        let logs = logs().await;
        let (tx, mut rx) = mpsc::channel(1);

        let attempts = attempts().await;

        handle_worker_message(
            &registry,
            &instances,
            &logs,
            &attempts,
            WorkerMessage {
                kind: Some(worker_message::Kind::Register(RegisterWorker {
                    worker_id: "worker-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "finance".to_owned(),
                    cluster: "prod".to_owned(),
                    region: "cn".to_owned(),
                    capabilities: Vec::new(),
                    labels: std::collections::HashMap::default(),
                })),
            },
            &tx,
            &tx,
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
                assert_eq!(registered.worker_id, "worker-1");
            }
            other => panic!("unexpected ack: {other:?}"),
        }

        assert!(registry.get("worker-1").await.is_some());
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

    async fn logs() -> JobInstanceLogRepository {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        JobInstanceLogRepository::new(db)
    }
}
