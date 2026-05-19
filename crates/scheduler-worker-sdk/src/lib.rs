//! Rust Worker SDK for active outbound scheduler Worker Tunnel connections.

#![forbid(unsafe_code)]

use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use scheduler_proto::worker::v1::{
    Heartbeat, Ping, RegisterWorker, ServerMessage, WorkerMessage, WorkerRegistered,
    server_message, worker_message, worker_tunnel_service_client::WorkerTunnelServiceClient,
};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Status, Streaming};

/// Worker runtime configuration used during registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerConfig {
    /// Scheduler Worker Tunnel endpoint, for example `http://127.0.0.1:9091`.
    pub endpoint: String,
    /// Stable worker identity.
    pub worker_id: String,
    /// Application name.
    pub app: String,
    /// Namespace name.
    pub namespace: String,
    /// Cluster name reported by this worker.
    pub cluster: String,
    /// Region reported by this worker.
    pub region: String,
    /// Runtime capabilities.
    pub capabilities: Vec<String>,
    /// Worker labels.
    pub labels: HashMap<String, String>,
}

impl WorkerConfig {
    /// Build a minimal local-development worker configuration.
    #[must_use]
    pub fn local(endpoint: impl Into<String>, worker_id: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            worker_id: worker_id.into(),
            app: "default".to_owned(),
            namespace: "default".to_owned(),
            cluster: "local".to_owned(),
            region: "local".to_owned(),
            capabilities: Vec::new(),
            labels: HashMap::new(),
        }
    }

    fn register_message(&self) -> WorkerMessage {
        WorkerMessage {
            kind: Some(worker_message::Kind::Register(RegisterWorker {
                worker_id: self.worker_id.clone(),
                app: self.app.clone(),
                namespace: self.namespace.clone(),
                cluster: self.cluster.clone(),
                region: self.region.clone(),
                capabilities: self.capabilities.clone(),
                labels: self.labels.clone(),
            })),
        }
    }
}

/// Active Worker Tunnel session.
pub struct WorkerSession {
    worker_id: String,
    lease_seconds: u64,
    outbound: mpsc::Sender<WorkerMessage>,
    inbound: Streaming<ServerMessage>,
    heartbeat_sequence: u64,
}

impl WorkerSession {
    /// Registered worker id acknowledged by scheduler.
    #[must_use]
    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }

    /// Lease seconds returned by scheduler registration ack.
    #[must_use]
    pub const fn lease_seconds(&self) -> u64 {
        self.lease_seconds
    }

    /// Send one heartbeat and wait for the matching ping response.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel is closed, the scheduler returns a gRPC error,
    /// or the response type is unexpected.
    pub async fn heartbeat(&mut self) -> Result<Ping, WorkerSdkError> {
        self.heartbeat_sequence = self.heartbeat_sequence.saturating_add(1);
        let sequence = self.heartbeat_sequence;
        self.outbound
            .send(WorkerMessage {
                kind: Some(worker_message::Kind::Heartbeat(Heartbeat {
                    worker_id: self.worker_id.clone(),
                    sequence,
                })),
            })
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)?;

        loop {
            let message = self.next_server_message().await?;
            if let Some(server_message::Kind::Ping(ping)) = message.kind
                && ping.sequence == sequence
            {
                return Ok(ping);
            }
        }
    }

    /// Send heartbeats forever at the provided interval.
    ///
    /// # Errors
    ///
    /// Returns an error when a heartbeat fails.
    pub async fn heartbeat_loop(&mut self, interval: Duration) -> Result<(), WorkerSdkError> {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            self.heartbeat().await?;
        }
    }

    async fn next_server_message(&mut self) -> Result<ServerMessage, WorkerSdkError> {
        self.inbound
            .message()
            .await?
            .ok_or(WorkerSdkError::TunnelClosed)
    }
}

/// Worker tunnel client.
#[derive(Debug, Clone)]
pub struct WorkerClient {
    config: WorkerConfig,
}

impl WorkerClient {
    /// Create a client from worker configuration.
    #[must_use]
    pub const fn new(config: WorkerConfig) -> Self {
        Self { config }
    }

    /// Open the tunnel, register the worker, and return an active session.
    ///
    /// # Errors
    ///
    /// Returns an error when connecting, sending registration, or reading the ack fails.
    pub async fn connect(self) -> Result<WorkerSession, WorkerSdkError> {
        let mut client = WorkerTunnelServiceClient::connect(self.config.endpoint.clone()).await?;
        let (tx, rx) = mpsc::channel(16);
        tx.send(self.config.register_message())
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)?;

        let response = client.open_tunnel(ReceiverStream::new(rx)).await?;
        let mut inbound = response.into_inner();
        let registered = read_registration(&mut inbound).await?;

        Ok(WorkerSession {
            worker_id: registered.worker_id,
            lease_seconds: registered.lease_seconds,
            outbound: tx,
            inbound,
            heartbeat_sequence: 0,
        })
    }
}

async fn read_registration(
    inbound: &mut Streaming<ServerMessage>,
) -> Result<WorkerRegistered, WorkerSdkError> {
    let message = inbound
        .message()
        .await?
        .ok_or(WorkerSdkError::TunnelClosed)?;

    match message.kind {
        Some(server_message::Kind::Registered(registered)) => Ok(registered),
        Some(server_message::Kind::Ping(_)) | None => Err(WorkerSdkError::UnexpectedMessage),
    }
}

/// User-provided async processor interface for future task dispatch support.
#[async_trait]
pub trait TaskProcessor: Send + Sync + 'static {
    /// Execute one task payload.
    async fn process(&self, task: TaskContext) -> Result<TaskOutcome, WorkerSdkError>;
}

/// Minimal task context placeholder reserved for Worker dispatch protocol evolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskContext {
    /// Job identifier.
    pub job_id: String,
    /// Instance identifier.
    pub instance_id: String,
    /// Raw task payload.
    pub payload: Vec<u8>,
}

/// Minimal processor outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutcome {
    /// Task completed successfully.
    Succeeded,
    /// Task failed with a message safe to send back to scheduler.
    Failed(String),
}

/// Worker SDK errors.
#[derive(Debug, Error)]
pub enum WorkerSdkError {
    /// gRPC transport error.
    #[error("worker tunnel transport error: {0}")]
    Transport(#[from] tonic::transport::Error),
    /// gRPC status error.
    #[error("worker tunnel status error: {0}")]
    Status(#[from] Status),
    /// The tunnel closed before the expected response arrived.
    #[error("worker tunnel closed")]
    TunnelClosed,
    /// Scheduler returned an unexpected server message.
    #[error("unexpected worker tunnel server message")]
    UnexpectedMessage,
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use scheduler_proto::worker::v1::worker_tunnel_service_server::WorkerTunnelServiceServer;
    use scheduler_server::tunnel::{WorkerRegistry, WorkerTunnel};
    use tokio::{net::TcpListener, task::JoinHandle};
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::transport::Server;

    use super::{WorkerClient, WorkerConfig};

    #[tokio::test]
    async fn worker_client_registers_and_sends_heartbeat() {
        let (addr, server) = start_tunnel_server().await;
        let mut config = WorkerConfig::local(format!("http://{addr}"), "worker-sdk-1");
        config.app = "billing".to_owned();
        config.namespace = "default".to_owned();

        let mut session = WorkerClient::new(config)
            .connect()
            .await
            .unwrap_or_else(|error| panic!("worker should register: {error}"));
        let ping = session
            .heartbeat()
            .await
            .unwrap_or_else(|error| panic!("heartbeat should ping: {error}"));

        assert_eq!(session.worker_id(), "worker-sdk-1");
        assert_eq!(session.lease_seconds(), 30);
        assert_eq!(ping.sequence, 1);

        server.abort();
    }

    async fn start_tunnel_server() -> (SocketAddr, JoinHandle<()>) {
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap_or_else(|error| panic!("listener should bind: {error}"));
        let addr = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener should expose addr: {error}"));
        let incoming = TcpListenerStream::new(listener);
        let service = WorkerTunnelServiceServer::new(WorkerTunnel::new(WorkerRegistry::default()));
        let server = tokio::spawn(async move {
            Server::builder()
                .add_service(service)
                .serve_with_incoming(incoming)
                .await
                .unwrap_or_else(|error| panic!("test server should run: {error}"));
        });
        (addr, server)
    }
}
