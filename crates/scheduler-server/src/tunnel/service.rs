//! gRPC Worker Tunnel service.

use scheduler_proto::worker::v1::{
    Heartbeat, Ping, ServerMessage, WorkerMessage, WorkerRegistered, server_message,
    worker_message, worker_tunnel_service_server::WorkerTunnelService,
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
}

impl WorkerTunnel {
    /// Create a Worker Tunnel service backed by an in-memory registry.
    #[must_use]
    pub const fn new(registry: WorkerRegistry) -> Self {
        Self { registry }
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
        let (tx, rx) = mpsc::channel(16);

        tokio::spawn(async move {
            while let Some(message) = inbound.message().await.transpose() {
                match message {
                    Ok(message) => {
                        if handle_worker_message(&registry, message, &tx)
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
    message: WorkerMessage,
    tx: &mpsc::Sender<Result<ServerMessage, Status>>,
) -> Result<(), mpsc::error::SendError<Result<ServerMessage, Status>>> {
    match message.kind {
        Some(worker_message::Kind::Register(register)) => {
            let worker = registry.register(register).await;
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
        None => {
            tx.send(Err(Status::invalid_argument(
                "worker message kind is required",
            )))
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use scheduler_proto::worker::v1::{
        RegisterWorker, WorkerMessage, server_message, worker_message,
    };
    use tokio::sync::mpsc;

    use super::{WorkerRegistry, handle_worker_message};

    #[tokio::test]
    async fn register_message_updates_registry_and_acknowledges_worker() {
        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(1);

        handle_worker_message(
            &registry,
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
}
