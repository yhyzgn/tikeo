//! Worker tunnel server implementation.

pub mod dispatcher;
pub mod governance;
pub mod lifecycle;
pub mod registry;
pub mod service;

pub use registry::{RegisteredWorker, WorkerRegistry};
pub use service::{TaskLogBroadcaster, WorkerTunnel};

use std::net::SocketAddr;

use anyhow::{Context, Result};
use tikee_config::TlsEndpointConfig;
use tikee_proto::worker::v1::worker_tunnel_service_server::WorkerTunnelServiceServer;
use tikee_storage::{
    AuditLogRepository, JobInstanceAttemptRepository, JobInstanceLogRepository,
    JobInstanceRepository, WorkflowRepository,
};
use tonic::transport::Server;
use tracing::info;

/// Runtime dependencies owned by the Worker Tunnel listener.
#[derive(Clone)]
pub struct WorkerTunnelRuntime {
    registry: WorkerRegistry,
    instances: JobInstanceRepository,
    logs: JobInstanceLogRepository,
    attempts: JobInstanceAttemptRepository,
    workflows: WorkflowRepository,
    audit: AuditLogRepository,
    log_broadcaster: TaskLogBroadcaster,
}

impl WorkerTunnelRuntime {
    /// Create a Worker Tunnel runtime dependency bundle.
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
    info!(addr = %listen_addr, "tikee Worker Tunnel listening");

    let mut server = Server::builder();
    if tls.tls_enabled {
        server = server
            .tls_config(crate::transport_security::tonic_server_tls_config(tls)?)
            .context("failed to configure Worker Tunnel TLS")?;
    }
    server
        .add_service(WorkerTunnelServiceServer::new(WorkerTunnel::new(
            runtime.registry,
            runtime.instances,
            runtime.logs,
            runtime.attempts,
            runtime.workflows,
            runtime.audit,
            runtime.log_broadcaster,
        )))
        .serve(listen_addr)
        .await
        .context("worker tunnel gRPC server failed")
}
