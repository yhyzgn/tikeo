//! Server process orchestration.

use anyhow::{Context, Result};
use tikee_config::{AlertRetryConfig, TikeeConfig};
use tikee_storage::{
    AuditLogRepository, JobInstanceAttemptRepository, JobInstanceLogRepository,
    JobInstanceRepository, JobRepository, RaftRepository, ScriptRepository, UserRepository,
    WorkerLifecycleRepository, WorkflowRepository, connect_and_migrate,
};
use tokio::try_join;
use tracing::info;

use crate::{alert, cluster::coordinator_from_config_with_storage, http, tikee, tunnel};

/// Run all tikee server listeners.
///
/// # Errors
///
/// Returns an error when any listener fails to bind or serve.
pub async fn serve(config: TikeeConfig) -> Result<()> {
    let log_broadcaster = tunnel::TaskLogBroadcaster::default();
    let http_addr = config.server.listen_addr;
    let tunnel_addr = config.server.worker_tunnel_addr;
    let database_url = config.storage.database_url;
    let cluster_config = config.cluster;
    let auth_config = config.auth;
    let transport_security = config.transport_security;
    let observability = config.observability;
    let alert_retry_config = config.alert_retry;
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
    let alerts = tikee_storage::AlertRepository::new(db.clone());
    let worker_lifecycle = WorkerLifecycleRepository::new(db.clone());
    let registry = tunnel::WorkerRegistry::with_lifecycle(worker_lifecycle);
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
        .with_transport_security_config(transport_security.clone())
        .with_observability_config(observability)
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
    let alert_retry_cluster = cluster.clone();
    let tunnel_attempts = attempts;

    info!(%http_addr, %tunnel_addr, "starting tikee listeners");

    try_join!(
        run_http_listener(http_addr, http_router, transport_security.http.clone()),
        tunnel::serve_with_security(
            tunnel_addr,
            tunnel_runtime(
                registry.clone(),
                tunnel_instances,
                logs.clone(),
                tunnel_attempts,
                workflows.clone(),
                audit.clone(),
                log_broadcaster,
            ),
            &transport_security.worker_tunnel,
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
        run_alert_retry_worker(alerts, alert_retry_cluster, alert_retry_config),
    )
    .context("tikee listener failed")?;

    Ok(())
}

async fn run_http_listener(
    http_addr: std::net::SocketAddr,
    http_router: axum::Router,
    tls: tikee_config::TlsEndpointConfig,
) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(http_addr)
        .await
        .with_context(|| format!("failed to bind HTTP listener at {http_addr}"))?;
    http::serve_listener_with_state(listener, http_router, &tls).await
}

const fn tunnel_runtime(
    registry: tunnel::WorkerRegistry,
    instances: JobInstanceRepository,
    logs: JobInstanceLogRepository,
    attempts: JobInstanceAttemptRepository,
    workflows: WorkflowRepository,
    audit: AuditLogRepository,
    log_broadcaster: tunnel::TaskLogBroadcaster,
) -> tunnel::WorkerTunnelRuntime {
    tunnel::WorkerTunnelRuntime::new(
        registry,
        instances,
        logs,
        attempts,
        workflows,
        audit,
        log_broadcaster,
    )
}

async fn run_alert_retry_worker(
    alerts: tikee_storage::AlertRepository,
    cluster: crate::cluster::SharedClusterCoordinator,
    config: AlertRetryConfig,
) -> Result<()> {
    if config.enabled {
        alert::run_retry_loop(
            alerts,
            cluster,
            std::time::Duration::from_secs(config.interval_seconds.max(1)),
            config.batch_size.min(500),
            alert::AlertRetryPolicy {
                max_attempts: config.max_attempts.clamp(1, 20),
                backoff_seconds: config.backoff_seconds.clamp(1, 86_400),
            },
        )
        .await;
    } else {
        std::future::pending::<()>().await;
    }
    #[allow(unreachable_code)]
    Ok(())
}
