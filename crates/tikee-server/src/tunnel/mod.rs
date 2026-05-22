//! Worker tunnel server implementation.

pub mod dispatcher;
pub mod registry;
pub mod service;

pub use registry::{RegisteredWorker, WorkerRegistry};
pub use service::{TaskLogBroadcaster, WorkerTunnel};

use std::net::SocketAddr;

use anyhow::{Context, Result};
use tikee_proto::worker::v1::worker_tunnel_service_server::WorkerTunnelServiceServer;
use tikee_storage::{
    JobInstanceAttemptRepository, JobInstanceLogRepository, JobInstanceRepository,
    WorkflowRepository,
};
use tonic::transport::Server;
use tracing::info;

/// Run the gRPC Worker Tunnel listener.
///
/// # Errors
///
/// Returns an error when the listener fails to bind or serve.
pub async fn serve(
    listen_addr: SocketAddr,
    registry: WorkerRegistry,
    instances: JobInstanceRepository,
    logs: JobInstanceLogRepository,
    attempts: JobInstanceAttemptRepository,
    workflows: WorkflowRepository,
    log_broadcaster: TaskLogBroadcaster,
) -> Result<()> {
    info!(addr = %listen_addr, "tikee Worker Tunnel listening");

    Server::builder()
        .add_service(WorkerTunnelServiceServer::new(WorkerTunnel::new(
            registry,
            instances,
            logs,
            attempts,
            workflows,
            log_broadcaster,
        )))
        .serve(listen_addr)
        .await
        .context("worker tunnel gRPC server failed")
}
