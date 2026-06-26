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
    WorkerDispatchOutboxRepository, WorkerLifecycleRepository, WorkflowRepository,
    connect_and_migrate,
};
use tokio::try_join;
use tracing::{debug, error, info, trace, warn};

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
    let connection_url = config.storage.effective_connection_url();
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
    info!(
        http_addr = %http_addr,
        tunnel_addr = %tunnel_addr,
        cluster_mode = ?cluster_config.mode,
        node_id = %cluster_config.node_id,
        database_type = %config.storage.database.kind,
        log_console_enabled = observability.logging.channels.console.enabled,
        log_file_enabled = observability.logging.channels.file.enabled,
        log_error_file_enabled = observability.logging.channels.error_file.enabled,
        log_elk_enabled = observability.logging.channels.elk.enabled,
        "starting tikeo server runtime"
    );
    tikeo_storage::set_timestamp_offset(offset);
    let shard_policy = tikeo_storage::set_scheduler_shard_policy(
        cluster_config.scheduler_shard_map_version,
        cluster_config.scheduler_shard_count,
    )
    .map_err(|error| anyhow::anyhow!("invalid cluster scheduler shard policy: {error}"))?;
    info!(
        shard_map_version = shard_policy.shard_map_version,
        shard_count = shard_policy.shard_count,
        "configured scheduler shard policy"
    );
    info!(database_type = %config.storage.database.kind, "initializing storage and migrations");
    let db = match connect_and_migrate(&connection_url).await {
        Ok(db) => db,
        Err(error) => {
            error!(database_type = %config.storage.database.kind, %error, "storage initialization failed");
            return Err(error)
                .with_context(|| format!("failed to initialize storage at {connection_url}"));
        }
    };
    info!(database_type = %config.storage.database.kind, "storage initialized and migrations applied");
    debug!("constructing repository handles");
    let raft = RaftRepository::new(db.clone());
    let cluster = coordinator_from_config_with_storage(&cluster_config, &raft)
        .await
        .context("failed to initialize cluster coordinator")?;
    let cluster_status = cluster.status().await;
    info!(
        node_id = %cluster_status.node_id,
        role = cluster_status.role.as_str(),
        can_schedule = cluster_status.can_schedule,
        "cluster coordinator initialized"
    );
    let instances = JobInstanceRepository::new(db.clone());
    let logs = JobInstanceLogRepository::new(db.clone());
    let attempts = JobInstanceAttemptRepository::new(db.clone());
    let outbox = WorkerDispatchOutboxRepository::new(db.clone());
    let jobs = JobRepository::new(db.clone());
    let users = UserRepository::new(db.clone());
    let scripts = ScriptRepository::new(db.clone());
    let workflows = WorkflowRepository::new(db.clone());
    let audit = AuditLogRepository::new(db.clone());
    let alerts = tikeo_storage::AlertRepository::new(db.clone());
    let notification_channels = NotificationChannelRepository::new(db.clone());
    let notification_policies = NotificationPolicyRepository::new(db.clone());
    let notification_delivery_trigger = crate::notification::NotificationDeliveryTrigger::new();
    let notification_center = crate::notification::NotificationCenter::new(
        notification_channels.clone(),
        notification_policies.clone(),
        NotificationMessageRepository::new(db.clone()),
        NotificationDeliveryAttemptRepository::new(db.clone()),
        tikeo_storage::NotificationTemplateRepository::new(db.clone()),
        jobs.clone(),
    )
    .with_public_console_base_url(public_console_base_url.clone())
    .with_delivery_trigger(Some(notification_delivery_trigger.clone()));
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
    trace!("building HTTP router state");
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
        notification_delivery_trigger: Some(notification_delivery_trigger.clone()),
    });
    let tunnel_instances = instances.clone();
    let tikeo_instances = instances.clone();
    let dispatcher_jobs = jobs.clone();
    let dispatcher_instances = instances;
    let dispatcher_attempts = attempts.clone();
    let dispatcher_outbox = outbox.clone();
    let dispatcher_workflows = workflows.clone();
    let tick_cluster = cluster.clone();
    let dispatch_cluster = cluster.clone();
    let outbox_delivery_registry = registry.clone();
    let outbox_delivery_repo = outbox.clone();
    let outbox_delivery_node_id = cluster_config.node_id.clone();
    let alert_retry_cluster = cluster.clone();
    let notification_delivery_cluster = cluster.clone();
    let tunnel_attempts = attempts;

    info!(%http_addr, %tunnel_addr, "starting tikeo listeners");

    let run_result = try_join!(
        run_http_listener(http_addr, http_router, transport_security.http.clone()),
        tunnel::serve_with_security(
            tunnel_addr,
            tunnel::WorkerTunnelRuntime::new(tunnel::WorkerTunnelRuntimeParts {
                registry: registry.clone(),
                instances: tunnel_instances,
                jobs: jobs.clone(),
                logs: logs.clone(),
                attempts: tunnel_attempts,
                outbox: outbox.clone(),
                workflows: workflows.clone(),
                audit: audit.clone(),
                notifications: Some(notification_center.clone()),
                log_broadcaster,
            }),
            &transport_security.worker_tunnel,
        ),
        async {
            tikeo::run_tick_loop(jobs, tikeo_instances, tick_cluster).await;
            Ok::<(), anyhow::Error>(())
        },
        async {
            tunnel::dispatcher::run(tunnel::dispatcher::DispatcherContext {
                jobs: dispatcher_jobs,
                instances: dispatcher_instances,
                attempts: dispatcher_attempts,
                outbox: dispatcher_outbox,
                workflows: dispatcher_workflows,
                scripts: scripts.clone(),
                logs: logs.clone(),
                audit,
                registry,
                cluster: dispatch_cluster,
                notifications: notification_center.clone(),
            })
            .await;
            Ok::<(), anyhow::Error>(())
        },
        async {
            tunnel::outbox_delivery::run(
                outbox_delivery_repo,
                outbox_delivery_registry,
                outbox_delivery_node_id,
            )
            .await;
            Ok::<(), anyhow::Error>(())
        },
        run_alert_retry_worker(alerts, alert_retry_cluster, alert_retry_config),
        run_notification_delivery_worker(
            NotificationChannelRepository::new(db.clone()),
            NotificationMessageRepository::new(db.clone()),
            NotificationDeliveryAttemptRepository::new(db.clone()),
            notification_delivery_cluster,
            notification_delivery_config,
            Some(notification_delivery_trigger),
        ),
        run_worker_lease_scanner(worker_lifecycle),
    );
    match run_result {
        Ok(_) => {
            info!("all tikeo runtime loops exited");
            Ok(())
        }
        Err(error) => {
            error!(%error, "tikeo listener failed");
            Err(error).context("tikeo listener failed")
        }
    }
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
    notification_delivery_trigger: Option<crate::notification::NotificationDeliveryTrigger>,
}

fn build_http_router(parts: HttpRouterParts) -> axum::Router {
    http::router_with_state(
        http::AppState::new(http::AppStateParts {
            jobs: parts.jobs,
            instances: parts.instances,
            logs: parts.logs,
            attempts: parts.attempts,
            users: parts.users,
            scripts: parts.scripts,
            workflows: parts.workflows,
            audit: parts.audit,
            registry: parts.registry,
            cluster: parts.cluster,
        })
        .with_auth_config(parts.auth_config)
        .with_transport_security_config(parts.transport_security)
        .with_observability_config(parts.observability)
        .with_script_governance_config(parts.script_governance)
        .with_raft_transport_token(parts.raft_transport_token)
        .with_notification_public_console_base_url(parts.notification_public_console_base_url)
        .with_notification_delivery_trigger(parts.notification_delivery_trigger),
    )
}

async fn run_worker_lease_scanner(lifecycle: WorkerLifecycleRepository) -> Result<()> {
    debug!(interval_seconds = 10, "configuring worker lease scanner");
    info!(interval_seconds = 10, "starting worker lease scanner");
    tunnel::lifecycle::run_lease_scanner(lifecycle, std::time::Duration::from_secs(10)).await;
    Ok(())
}

async fn run_http_listener(
    http_addr: std::net::SocketAddr,
    http_router: axum::Router,
    tls: tikeo_config::TlsEndpointConfig,
) -> Result<()> {
    debug!(%http_addr, tls_enabled = tls.tls_enabled, mtls_required = tls.mtls_required, "binding HTTP listener");
    let listener = tokio::net::TcpListener::bind(http_addr)
        .await
        .with_context(|| format!("failed to bind HTTP listener at {http_addr}"))?;
    info!(%http_addr, tls_enabled = tls.tls_enabled, mtls_required = tls.mtls_required, "HTTP listener bound");
    http::serve_listener_with_state(listener, http_router, &tls).await
}

async fn run_alert_retry_worker(
    alerts: tikeo_storage::AlertRepository,
    cluster: crate::cluster::SharedClusterCoordinator,
    config: AlertRetryConfig,
) -> Result<()> {
    if config.enabled {
        info!(
            interval_seconds = config.interval_seconds.max(1),
            batch_size = config.batch_size.min(500),
            max_attempts = config.max_attempts.clamp(1, 20),
            "starting alert retry worker"
        );
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
        warn!("alert retry worker disabled by configuration");
        std::future::pending::<()>().await;
    }
    Ok(())
}

async fn run_notification_delivery_worker(
    channels: NotificationChannelRepository,
    messages: NotificationMessageRepository,
    attempts: NotificationDeliveryAttemptRepository,
    cluster: crate::cluster::SharedClusterCoordinator,
    config: NotificationDeliveryConfig,
    trigger: Option<crate::notification::NotificationDeliveryTrigger>,
) -> Result<()> {
    if config.enabled {
        info!(
            interval_seconds = config.interval_seconds.max(1),
            batch_size = config.batch_size.min(500),
            max_attempts = config.max_attempts.clamp(1, 20),
            "starting notification delivery worker"
        );
        let repositories = crate::notification::NotificationDeliveryRepositories::new(
            channels, messages, attempts,
        );
        let context = crate::notification::NotificationDeliveryLoopContext::new(
            repositories,
            cluster,
            std::time::Duration::from_secs(config.interval_seconds.max(1)),
            config.batch_size.min(500),
            crate::notification::NotificationDeliveryPolicy {
                max_attempts: config.max_attempts.clamp(1, 20),
                backoff_seconds: config.backoff_seconds.clamp(1, 86_400),
            },
            trigger,
        );
        crate::notification::run_delivery_loop(context).await;
    } else {
        warn!("notification delivery worker disabled by configuration");
        std::future::pending::<()>().await;
    }
    Ok(())
}
