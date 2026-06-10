//! Worker tunnel server implementation.

pub mod capability;
pub mod dispatcher;
pub mod governance;
pub mod lifecycle;
pub mod registry;
pub mod service;

pub use registry::{RegisteredWorker, WorkerRegistry};
pub use service::{TaskLogBroadcaster, WorkerTunnel};

use std::net::SocketAddr;

use anyhow::{Context, Result};
use tikeo_config::TlsEndpointConfig;
use tikeo_proto::worker::v1::worker_tunnel_service_server::WorkerTunnelServiceServer;
use tikeo_storage::{
    AuditLogRepository, JobInstanceAttemptRepository, JobInstanceLogRepository,
    JobInstanceRepository, JobRepository, NotificationChannelRepository,
    NotificationDeliveryAttemptRepository, NotificationMessageRepository,
    NotificationPolicyRepository, WorkflowRepository,
};
use tonic::transport::Server;
use tracing::info;

/// Runtime dependencies owned by the Worker Tunnel listener.
#[derive(Clone)]
pub struct WorkerTunnelRuntime {
    registry: WorkerRegistry,
    instances: JobInstanceRepository,
    jobs: JobRepository,
    logs: JobInstanceLogRepository,
    attempts: JobInstanceAttemptRepository,
    workflows: WorkflowRepository,
    audit: AuditLogRepository,
    notifications: crate::notification::NotificationCenter,
    log_broadcaster: TaskLogBroadcaster,
}

/// Input bundle used to create a Worker Tunnel runtime.
pub struct WorkerTunnelRuntimeParts {
    /// Shared Worker registry.
    pub registry: WorkerRegistry,
    /// Job instance repository.
    pub instances: JobInstanceRepository,
    /// Job definition repository.
    pub jobs: JobRepository,
    /// Job instance log repository.
    pub logs: JobInstanceLogRepository,
    /// Job instance attempt repository.
    pub attempts: JobInstanceAttemptRepository,
    /// Workflow repository.
    pub workflows: WorkflowRepository,
    /// Audit log repository.
    pub audit: AuditLogRepository,
    /// Notification Center materializer.
    pub notifications: Option<crate::notification::NotificationCenter>,
    /// Live task log broadcaster.
    pub log_broadcaster: TaskLogBroadcaster,
}

impl WorkerTunnelRuntime {
    /// Create a Worker Tunnel runtime dependency bundle.
    #[must_use]
    pub fn new(parts: WorkerTunnelRuntimeParts) -> Self {
        let jobs = parts.jobs;
        let notifications = parts.notifications.unwrap_or_else(|| {
            let db = jobs.db();
            crate::notification::NotificationCenter::new(
                NotificationChannelRepository::new(db.clone()),
                NotificationPolicyRepository::new(db.clone()),
                NotificationMessageRepository::new(db.clone()),
                NotificationDeliveryAttemptRepository::new(db),
                jobs.clone(),
            )
        });
        Self {
            registry: parts.registry,
            instances: parts.instances,
            jobs,
            logs: parts.logs,
            attempts: parts.attempts,
            workflows: parts.workflows,
            audit: parts.audit,
            notifications,
            log_broadcaster: parts.log_broadcaster,
        }
    }
}

/// Run the gRPC Worker Tunnel listener.
///
/// # Errors
///
/// Returns an error when the listener fails to bind or serve.
pub async fn serve(listen_addr: SocketAddr, runtime: WorkerTunnelRuntime) -> Result<()> {
    serve_with_security(listen_addr, runtime, &TlsEndpointConfig::default()).await
}

/// Run the gRPC Worker Tunnel listener with optional TLS/mTLS.
///
/// # Errors
///
/// Returns an error when the listener fails to bind/serve or TLS material cannot be loaded.
pub async fn serve_with_security(
    listen_addr: SocketAddr,
    runtime: WorkerTunnelRuntime,
    tls: &TlsEndpointConfig,
) -> Result<()> {
    info!(addr = %listen_addr, "tikeo Worker Tunnel listening");

    let mut server = Server::builder();
    if tls.tls_enabled {
        server = server
            .tls_config(crate::transport_security::tonic_server_tls_config(tls)?)
            .context("failed to configure Worker Tunnel TLS")?;
    }
    server
        .add_service(WorkerTunnelServiceServer::new(WorkerTunnel::new(runtime)))
        .serve(listen_addr)
        .await
        .context("worker tunnel gRPC server failed")
}
