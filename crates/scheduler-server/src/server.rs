//! Server process orchestration.

use anyhow::{Context, Result};
use scheduler_config::SchedulerConfig;
use scheduler_storage::{
    AuditLogRepository, JobInstanceAttemptRepository, JobInstanceLogRepository,
    JobInstanceRepository, JobRepository, ScriptRepository, UserRepository, WorkflowRepository,
    connect_and_migrate,
};
use tokio::try_join;
use tracing::info;

use crate::{http, scheduler, tunnel};

/// Run all scheduler server listeners.
///
/// # Errors
///
/// Returns an error when any listener fails to bind or serve.
pub async fn serve(config: SchedulerConfig) -> Result<()> {
    let registry = tunnel::WorkerRegistry::default();
    let http_addr = config.server.listen_addr;
    let tunnel_addr = config.server.worker_tunnel_addr;
    let database_url = config.storage.database_url;
    let db = connect_and_migrate(&database_url)
        .await
        .with_context(|| format!("failed to initialize storage at {database_url}"))?;
    let instances = JobInstanceRepository::new(db.clone());
    let logs = JobInstanceLogRepository::new(db.clone());
    let attempts = JobInstanceAttemptRepository::new(db.clone());
    let jobs = JobRepository::new(db.clone());
    let users = UserRepository::new(db.clone());
    let scripts = ScriptRepository::new(db.clone());
    let workflows = WorkflowRepository::new(db.clone());
    let audit = AuditLogRepository::new(db.clone());
    let http_router = http::router_with_state(http::AppState::new(
        jobs.clone(),
        instances.clone(),
        logs.clone(),
        attempts.clone(),
        users,
        scripts,
        workflows.clone(),
        audit,
        registry.clone(),
    ));
    let tunnel_instances = instances.clone();
    let scheduler_instances = instances.clone();
    let dispatcher_jobs = jobs.clone();
    let dispatcher_instances = instances;
    let dispatcher_attempts = attempts.clone();
    let dispatcher_workflows = workflows.clone();
    let tunnel_attempts = attempts;

    info!(%http_addr, %tunnel_addr, "starting scheduler listeners");

    try_join!(
        http::serve_with_state(http_addr, http_router),
        tunnel::serve(
            tunnel_addr,
            registry.clone(),
            tunnel_instances,
            logs,
            tunnel_attempts
        ),
        async {
            scheduler::run_tick_loop(jobs, scheduler_instances).await;
            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        },
        async {
            tunnel::dispatcher::run(
                dispatcher_jobs,
                dispatcher_instances,
                dispatcher_attempts,
                dispatcher_workflows,
                registry,
            )
            .await;
            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        },
    )
    .context("scheduler listener failed")?;

    Ok(())
}
