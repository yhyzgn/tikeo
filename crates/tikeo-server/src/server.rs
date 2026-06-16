//! Server process orchestration.

use anyhow::{Context, Result};
use tikeo_config::{
    AlertRetryConfig, AuthConfig, NotificationDeliveryConfig, ObservabilityConfig,
    ScriptGovernanceConfig, TikeoConfig, TransportSecurityConfig,
};
use tikeo_storage::{
    AuditLogRepository, JobInstanceAttemptRepository, JobInstanceLogRepository,
    JobInstanceRepository, JobRepository, NotificationChannelRepository,
    NotificationDeliveryAttemptRepository, NotificationMessageRepository,
    NotificationPolicyRepository, RaftRepository, ScriptRepository, UserRepository,
    WorkerLifecycleRepository, WorkflowRepository, connect_and_migrate,
};
use tokio::try_join;
use tracing::info;

use crate::{alert, cluster::coordinator_from_config_with_storage, http, tikeo, tunnel};

/// Run all tikeo server listeners.
///
/// # Errors
///
/// Returns an error when any listener fails to bind or serve.
pub async fn serve(config: TikeoConfig) -> Result<()> {
    let log_broadcaster = tunnel::TaskLogBroadcaster::default();
    let http_addr = config.server.listen_addr;
    let tunnel_addr = config.server.worker_tunnel_addr;
    let timestamp_offset = config.storage.timestamp_offset.clone();
    let database_url = config.storage.database_url;
    let cluster_config = config.cluster;
    let auth_config = config.auth;
    let transport_security = config.transport_security;
    let observability = config.observability;
    let alert_retry_config = config.alert_retry;
    let notification_delivery_config = config.notification_delivery;
    let public_console_base_url = notification_delivery_config.public_console_base_url.clone();
    let script_governance = config.script_governance;
    let raft_transport_token = cluster_config.transport_token.clone();
    let offset = tikeo_storage::parse_timestamp_offset(&timestamp_offset)
        .with_context(|| format!("invalid storage.timestamp_offset: {timestamp_offset}"))?;
    tikeo_storage::set_timestamp_offset(offset);
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
    let alerts = tikeo_storage::AlertRepository::new(db.clone());
    let notification_channels = NotificationChannelRepository::new(db.clone());
    let notification_policies = NotificationPolicyRepository::new(db.clone());
    let notification_center = crate::notification::NotificationCenter::new(
        notification_channels.clone(),
        notification_policies.clone(),
        NotificationMessageRepository::new(db.clone()),
        NotificationDeliveryAttemptRepository::new(db.clone()),
        tikeo_storage::NotificationTemplateRepository::new(db.clone()),
        jobs.clone(),
    )
    .with_public_console_base_url(public_console_base_url.clone());
    let plugins = tikeo_storage::PluginRepository::new(db.clone())
        .list_plugins()
        .await
        .context("failed to load plugin notification channel metadata for alert migration")?
        .into_iter()
        .filter(|plugin| plugin.enabled)
        .flat_map(|plugin| plugin.alert_channel_types)
        .collect::<Vec<_>>();
    let alert_backfill = crate::notification::backfill_alert_rule_notification_policies(
        &alerts,
        &notification_channels,
        &notification_policies,
        &plugins,
    )
    .await
    .context("failed to backfill legacy alert notification policies")?;
    if alert_backfill.policies_created > 0 || alert_backfill.already_backfilled > 0 {
        info!(
            rules_seen = alert_backfill.rules_seen,
            policies_created = alert_backfill.policies_created,
            channels_created = alert_backfill.channels_created,
            already_backfilled = alert_backfill.already_backfilled,
            "alert notification policy backfill completed"
        );
    }
    let worker_lifecycle = WorkerLifecycleRepository::new(db.clone());
    let registry = tunnel::WorkerRegistry::with_lifecycle(worker_lifecycle.clone())
        .with_gateway_node_id(cluster_config.node_id.clone())
        .with_relay(std::sync::Arc::new(
            tunnel::HttpWorkerRelayDispatch::from_peers(
                &cluster_config.peers,
                cluster_config.transport_token.clone(),
            ),
        ));
    let http_router = build_http_router(HttpRouterParts {
        jobs: jobs.clone(),
        instances: instances.clone(),
        logs: logs.clone(),
        attempts: attempts.clone(),
        users,
        scripts: scripts.clone(),
        workflows: workflows.clone(),
        audit: audit.clone(),
        registry: registry.clone(),
        cluster: cluster.clone(),
        auth_config,
        transport_security: transport_security.clone(),
        observability,
        script_governance,
        raft_transport_token,
        notification_public_console_base_url: public_console_base_url,
    });
    let tunnel_instances = instances.clone();
    let tikeo_instances = instances.clone();
    let dispatcher_jobs = jobs.clone();
    let dispatcher_instances = instances;
    let dispatcher_attempts = attempts.clone();
    let dispatcher_workflows = workflows.clone();
    let tick_cluster = cluster.clone();
    let dispatch_cluster = cluster.clone();
    let alert_retry_cluster = cluster.clone();
    let notification_delivery_cluster = cluster.clone();
    let tunnel_attempts = attempts;

    info!(%http_addr, %tunnel_addr, "starting tikeo listeners");

    try_join!(
        run_http_listener(http_addr, http_router, transport_security.http.clone()),
        tunnel::serve_with_security(
            tunnel_addr,
            tunnel::WorkerTunnelRuntime::new(tunnel::WorkerTunnelRuntimeParts {
                registry: registry.clone(),
                instances: tunnel_instances,
                jobs: jobs.clone(),
                logs: logs.clone(),
                attempts: tunnel_attempts,
                workflows: workflows.clone(),
                audit: audit.clone(),
                notifications: Some(notification_center.clone()),
                log_broadcaster,
            }),
            &transport_security.worker_tunnel,
        ),
        async {
            tikeo::run_tick_loop(jobs, tikeo_instances, tick_cluster).await;
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
                notification_center.clone(),
            )
            .await;
            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        },
        run_alert_retry_worker(alerts, alert_retry_cluster, alert_retry_config),
        run_notification_delivery_worker(
            NotificationChannelRepository::new(db.clone()),
            NotificationMessageRepository::new(db.clone()),
            NotificationDeliveryAttemptRepository::new(db.clone()),
            notification_delivery_cluster,
            notification_delivery_config,
        ),
        run_worker_lease_scanner(worker_lifecycle),
    )
    .context("tikeo listener failed")?;

    Ok(())
}

struct HttpRouterParts {
    jobs: JobRepository,
    instances: JobInstanceRepository,
    logs: JobInstanceLogRepository,
    attempts: JobInstanceAttemptRepository,
    users: UserRepository,
    scripts: ScriptRepository,
    workflows: WorkflowRepository,
    audit: AuditLogRepository,
    registry: tunnel::WorkerRegistry,
    cluster: crate::cluster::SharedClusterCoordinator,
    auth_config: AuthConfig,
    transport_security: TransportSecurityConfig,
    observability: ObservabilityConfig,
    script_governance: ScriptGovernanceConfig,
    raft_transport_token: Option<String>,
    notification_public_console_base_url: Option<String>,
}

fn build_http_router(parts: HttpRouterParts) -> axum::Router {
    http::router_with_state(
        http::AppState::new(
            parts.jobs,
            parts.instances,
            parts.logs,
            parts.attempts,
            parts.users,
            parts.scripts,
            parts.workflows,
            parts.audit,
            parts.registry,
            parts.cluster,
        )
        .with_auth_config(parts.auth_config)
        .with_transport_security_config(parts.transport_security)
        .with_observability_config(parts.observability)
        .with_script_governance_config(parts.script_governance)
        .with_raft_transport_token(parts.raft_transport_token)
        .with_notification_public_console_base_url(parts.notification_public_console_base_url),
    )
}

async fn run_worker_lease_scanner(lifecycle: WorkerLifecycleRepository) -> Result<()> {
    tunnel::lifecycle::run_lease_scanner(lifecycle, std::time::Duration::from_secs(10)).await;
    #[allow(unreachable_code)]
    Ok(())
}

async fn run_http_listener(
    http_addr: std::net::SocketAddr,
    http_router: axum::Router,
    tls: tikeo_config::TlsEndpointConfig,
) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(http_addr)
        .await
        .with_context(|| format!("failed to bind HTTP listener at {http_addr}"))?;
    http::serve_listener_with_state(listener, http_router, &tls).await
}

async fn run_alert_retry_worker(
    alerts: tikeo_storage::AlertRepository,
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

async fn run_notification_delivery_worker(
    channels: NotificationChannelRepository,
    messages: NotificationMessageRepository,
    attempts: NotificationDeliveryAttemptRepository,
    cluster: crate::cluster::SharedClusterCoordinator,
    config: NotificationDeliveryConfig,
) -> Result<()> {
    if config.enabled {
        crate::notification::run_delivery_loop(
            channels,
            messages,
            attempts,
            cluster,
            std::time::Duration::from_secs(config.interval_seconds.max(1)),
            config.batch_size.min(500),
            crate::notification::NotificationDeliveryPolicy {
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
