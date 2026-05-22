//! Rust Worker SDK for active outbound scheduler Worker Tunnel connections.

#![forbid(unsafe_code)]

use std::{collections::HashMap, time::Duration};

/// Generated Worker Tunnel protocol bindings bundled for standalone SDK publishing.
pub mod proto {
    /// Worker tunnel protocol bindings.
    pub mod worker {
        /// Version 1 worker tunnel protocol.
        pub mod v1 {
            #![allow(
                missing_docs,
                clippy::default_trait_access,
                clippy::derive_partial_eq_without_eq,
                clippy::doc_markdown,
                clippy::missing_const_for_fn,
                clippy::missing_errors_doc
            )]
            tonic::include_proto!("scheduler.worker.v1");
        }
    }
}

use crate::proto::worker::v1::{
    DispatchTask, Heartbeat, Ping, RegisterWorker, ServerMessage, TaskLog, TaskResult,
    WorkerMessage, WorkerRegistered, server_message, worker_message,
    worker_tunnel_service_client::WorkerTunnelServiceClient,
};
use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Status, Streaming};

/// Worker runtime configuration used during registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerConfig {
    /// Scheduler Worker Tunnel endpoint, for example `http://0.0.0.0:9998`.
    pub endpoint: String,
    /// Optional client-side stable instance hint for observability/reconnect correlation.
    ///
    /// The scheduler assigns the authoritative `worker_id` during registration.
    pub client_instance_id: String,
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
    pub fn local(endpoint: impl Into<String>, client_instance_id: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client_instance_id: client_instance_id.into(),
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
                client_instance_id: self.client_instance_id.clone(),
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

    /// Emit one task log through the worker tunnel.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel is closed.
    pub async fn emit_log(
        &self,
        instance_id: impl Into<String>,
        level: impl Into<String>,
        message: impl Into<String>,
        sequence: i64,
    ) -> Result<(), WorkerSdkError> {
        self.outbound
            .send(WorkerMessage {
                kind: Some(worker_message::Kind::TaskLog(TaskLog {
                    worker_id: self.worker_id.clone(),
                    instance_id: instance_id.into(),
                    level: level.into(),
                    message: message.into(),
                    sequence,
                })),
            })
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)
    }

    /// Wait for one dispatched task, run it through the provided processor, and report the result.
    ///
    /// # Errors
    ///
    /// Returns an error when the tunnel closes, the scheduler returns a gRPC error,
    /// or the result cannot be sent back.
    pub async fn process_next<P>(&mut self, processor: &P) -> Result<TaskOutcome, WorkerSdkError>
    where
        P: TaskProcessor,
    {
        loop {
            let message = self.next_server_message().await?;
            if let Some(server_message::Kind::DispatchTask(task)) = message.kind {
                return self.process_task(processor, task).await;
            }
        }
    }

    async fn process_task<P>(
        &self,
        processor: &P,
        task: DispatchTask,
    ) -> Result<TaskOutcome, WorkerSdkError>
    where
        P: TaskProcessor,
    {
        let processor_name = if task.processor_name.is_empty() {
            task.job_id.clone()
        } else {
            task.processor_name
        };
        let context = TaskContext {
            job_id: task.job_id,
            processor_name,
            instance_id: task.instance_id,
            payload: task.payload,
        };
        let outcome = match processor.process(context.clone()).await {
            Ok(outcome) => outcome,
            Err(error) => TaskOutcome::Failed(error.to_string()),
        };
        self.outbound
            .send(WorkerMessage {
                kind: Some(worker_message::Kind::TaskResult(TaskResult {
                    worker_id: self.worker_id.clone(),
                    instance_id: context.instance_id,
                    success: matches!(outcome, TaskOutcome::Succeeded),
                    message: outcome.message().unwrap_or_default(),
                })),
            })
            .await
            .map_err(|_| WorkerSdkError::TunnelClosed)?;

        Ok(outcome)
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
        Some(server_message::Kind::Ping(_) | server_message::Kind::DispatchTask(_)) | None => {
            Err(WorkerSdkError::UnexpectedMessage)
        }
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
    /// Explicit processor key/name for SDK routing.
    pub processor_name: String,
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

impl TaskOutcome {
    fn message(&self) -> Option<String> {
        match self {
            Self::Succeeded => None,
            Self::Failed(message) => Some(message.clone()),
        }
    }
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
    use std::{net::SocketAddr, pin::Pin};

    use async_trait::async_trait;
    use tokio::{net::TcpListener, sync::mpsc, task::JoinHandle};
    use tokio_stream::{Stream, StreamExt, wrappers::TcpListenerStream};
    use tonic::{Request, Response, Status, transport::Server};

    use crate::proto::worker::v1::{
        DispatchTask, Ping, ServerMessage, WorkerMessage, WorkerRegistered, server_message,
        worker_message, worker_tunnel_service_server,
        worker_tunnel_service_server::WorkerTunnelServiceServer,
    };

    use super::{
        TaskContext, TaskOutcome, TaskProcessor, WorkerClient, WorkerConfig, WorkerSdkError,
    };

    #[tokio::test]
    async fn worker_client_registers_and_sends_heartbeat() {
        let (addr, server, _events) = start_mock_tunnel_server(None).await;
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

        assert_eq!(session.worker_id(), "mock-worker-sdk-1");
        assert_eq!(session.lease_seconds(), 30);
        assert_eq!(ping.sequence, 1);

        server.abort();
    }

    #[tokio::test]
    async fn worker_session_processes_dispatched_task_and_reports_result() {
        let (addr, server, mut events) = start_mock_tunnel_server(Some(DispatchTask {
            instance_id: "instance-1".to_owned(),
            job_id: "job-1".to_owned(),
            payload: b"hello".to_vec(),
            processor_name: "demo.echo".to_owned(),
            processor_binding: None,
        }))
        .await;

        let config = WorkerConfig::local(format!("http://{addr}"), "worker-sdk-2");
        let mut session = WorkerClient::new(config)
            .connect()
            .await
            .unwrap_or_else(|error| panic!("worker should register: {error}"));
        session
            .emit_log("instance-1", "info", "starting", 1)
            .await
            .unwrap_or_else(|error| panic!("log should emit: {error}"));

        let outcome = session
            .process_next(&EchoProcessor)
            .await
            .unwrap_or_else(|error| panic!("task should process: {error}"));
        assert_eq!(outcome, TaskOutcome::Succeeded);

        let mut saw_log = false;
        let mut saw_result = false;
        while let Some(message) = events.recv().await {
            match message.kind {
                Some(worker_message::Kind::TaskLog(log)) => {
                    saw_log = log.instance_id == "instance-1" && log.message == "starting";
                }
                Some(worker_message::Kind::TaskResult(result)) => {
                    saw_result = result.instance_id == "instance-1" && result.success;
                    break;
                }
                _ => {}
            }
        }
        assert!(saw_log, "mock tunnel should receive emitted task log");
        assert!(saw_result, "mock tunnel should receive task result");

        server.abort();
    }

    async fn start_mock_tunnel_server(
        dispatch: Option<DispatchTask>,
    ) -> (SocketAddr, JoinHandle<()>, mpsc::Receiver<WorkerMessage>) {
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap_or_else(|error| panic!("listener should bind: {error}"));
        let addr = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener should expose addr: {error}"));
        let incoming = TcpListenerStream::new(listener);
        let (events_tx, events_rx) = mpsc::channel(16);
        let service = WorkerTunnelServiceServer::new(MockTunnel {
            dispatch,
            events: events_tx,
        });
        let server = tokio::spawn(async move {
            Server::builder()
                .add_service(service)
                .serve_with_incoming(incoming)
                .await
                .unwrap_or_else(|error| panic!("test server should run: {error}"));
        });
        (addr, server, events_rx)
    }

    struct MockTunnel {
        dispatch: Option<DispatchTask>,
        events: mpsc::Sender<WorkerMessage>,
    }

    type ResponseStream = Pin<Box<dyn Stream<Item = Result<ServerMessage, Status>> + Send>>;

    #[tonic::async_trait]
    impl worker_tunnel_service_server::WorkerTunnelService for MockTunnel {
        type OpenTunnelStream = ResponseStream;
        type SubscribeTaskLogsStream =
            Pin<Box<dyn Stream<Item = Result<crate::proto::worker::v1::TaskLog, Status>> + Send>>;

        async fn open_tunnel(
            &self,
            request: Request<tonic::Streaming<WorkerMessage>>,
        ) -> Result<Response<Self::OpenTunnelStream>, Status> {
            let mut inbound = request.into_inner();
            let (outbound_tx, outbound_rx) = mpsc::channel(16);
            let events = self.events.clone();
            let dispatch = self.dispatch.clone();
            tokio::spawn(async move {
                while let Some(message) = inbound.next().await {
                    let Ok(message) = message else { break };
                    let _ = events.send(message.clone()).await;
                    match message.kind {
                        Some(worker_message::Kind::Register(register)) => {
                            let _ = outbound_tx
                                .send(Ok(ServerMessage {
                                    kind: Some(server_message::Kind::Registered(
                                        WorkerRegistered {
                                            worker_id: format!(
                                                "mock-{}",
                                                register.client_instance_id
                                            ),
                                            lease_seconds: 30,
                                        },
                                    )),
                                }))
                                .await;
                            if let Some(task) = dispatch.clone() {
                                let _ = outbound_tx
                                    .send(Ok(ServerMessage {
                                        kind: Some(server_message::Kind::DispatchTask(task)),
                                    }))
                                    .await;
                            }
                        }
                        Some(worker_message::Kind::Heartbeat(heartbeat)) => {
                            let _ = outbound_tx
                                .send(Ok(ServerMessage {
                                    kind: Some(server_message::Kind::Ping(Ping {
                                        sequence: heartbeat.sequence,
                                    })),
                                }))
                                .await;
                        }
                        Some(
                            worker_message::Kind::TaskResult(_) | worker_message::Kind::TaskLog(_),
                        )
                        | None => {}
                    }
                }
            });

            Ok(Response::new(Box::pin(
                tokio_stream::wrappers::ReceiverStream::new(outbound_rx),
            )))
        }

        async fn subscribe_task_logs(
            &self,
            _request: Request<crate::proto::worker::v1::SubscribeTaskLogsRequest>,
        ) -> Result<Response<Self::SubscribeTaskLogsStream>, Status> {
            Ok(Response::new(Box::pin(tokio_stream::empty())))
        }
    }

    struct EchoProcessor;

    #[async_trait]
    impl TaskProcessor for EchoProcessor {
        async fn process(&self, task: TaskContext) -> Result<TaskOutcome, WorkerSdkError> {
            assert_eq!(task.payload, b"hello");
            Ok(TaskOutcome::Succeeded)
        }
    }
}
