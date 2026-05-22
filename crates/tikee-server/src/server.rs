//! Server process orchestration.

use anyhow::{Context, Result};
use tikee_config::TikeeConfig;
use tikee_storage::{
    AuditLogRepository, JobInstanceAttemptRepository, JobInstanceLogRepository,
    JobInstanceRepository, JobRepository, RaftRepository, ScriptRepository, UserRepository,
    WorkflowRepository, connect_and_migrate,
};
use tokio::try_join;
use tracing::info;

use crate::{cluster::coordinator_from_config_with_storage, http, tikee, tunnel};

/// Run all tikee server listeners.
///
/// # Errors
///
/// Returns an error when any listener fails to bind or serve.
pub async fn serve(config: TikeeConfig) -> Result<()> {
    let registry = tunnel::WorkerRegistry::default();
    let log_broadcaster = tunnel::TaskLogBroadcaster::default();
    let http_addr = config.server.listen_addr;
    let tunnel_addr = config.server.worker_tunnel_addr;
    let database_url = config.storage.database_url;
    let cluster_config = config.cluster;
    let auth_config = config.auth;
    let transport_security = config.transport_security;
    let raft_transport_token = cluster_config.transport_token.clone();
    let db = connect_and_migrate(&database_url)
        .await
        .with_context(|| format!("failed to initialize storage at {database_url}"))?;
    let raft = RaftRepository::new(db.clone());
    let cluster = coordinator_from_config_with_storage(&cluster_config, &raft)
        .await
        .context("failed to initialize cluster coordinator")?;
    let instances = JobInstanceRepository::new(db.clone());
    let logs = JobInstanceLogRepository::new(db.clone());
    let attempts = JobInstanceAttemptRepository::new(db.clone());
    let jobs = JobRepository::new(db.clone());
    let users = UserRepository::new(db.clone());
    let scripts = ScriptRepository::new(db.clone());
    let workflows = WorkflowRepository::new(db.clone());
    let audit = AuditLogRepository::new(db.clone());
    let http_router = http::router_with_state(
        http::AppState::new(
            jobs.clone(),
            instances.clone(),
            logs.clone(),
            attempts.clone(),
            users,
            scripts.clone(),
            workflows.clone(),
            audit.clone(),
            registry.clone(),
            cluster.clone(),
        )
        .with_auth_config(auth_config)
        .with_transport_security_config(transport_security)
        .with_raft_transport_token(raft_transport_token),
    );
    let tunnel_instances = instances.clone();
    let tikee_instances = instances.clone();
    let dispatcher_jobs = jobs.clone();
    let dispatcher_instances = instances;
    let dispatcher_attempts = attempts.clone();
    let dispatcher_workflows = workflows.clone();
    let tick_cluster = cluster.clone();
    let dispatch_cluster = cluster.clone();
    let tunnel_attempts = attempts;

    info!(%http_addr, %tunnel_addr, "starting tikee listeners");

    try_join!(
        http::serve_with_state(http_addr, http_router),
        tunnel::serve(
            tunnel_addr,
            registry.clone(),
            tunnel_instances,
            logs.clone(),
            tunnel_attempts,
            workflows.clone(),
            audit.clone(),
            log_broadcaster
        ),
        async {
            tikee::run_tick_loop(jobs, tikee_instances, tick_cluster).await;
            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        },
        async {
            tunnel::dispatcher::run(
                dispatcher_jobs,
                dispatcher_instances,
                dispatcher_attempts,
                dispatcher_workflows,
                scripts.clone(),
                logs.clone(),
                audit,
                registry,
                dispatch_cluster,
            )
            .await;
            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        },
    )
    .context("tikee listener failed")?;

    Ok(())
}
