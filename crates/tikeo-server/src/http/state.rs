//! Shared HTTP application state.

use std::time::SystemTime;

use crate::cluster::SharedClusterCoordinator;
use tikeo_config::{
    AuthConfig, ObservabilityConfig, ScriptGovernanceConfig, TransportSecurityConfig,
};
use tikeo_storage::{
    AlertRepository, AuditLogRepository, AuthSessionRepository, ClusterShardOwnershipRepository,
    JobInstanceAttemptRepository, JobInstanceLogRepository, JobInstanceRepository, JobRepository,
    NotificationChannelRepository, NotificationDeliveryAttemptRepository,
    NotificationMessageRepository, NotificationPolicyRepository, NotificationTemplateRepository,
    PluginRepository, RaftRepository, RbacRepository, ScriptRepository, UserRepository,
    WorkerDispatchOutboxRepository, WorkerLifecycleRepository, WorkflowRepository,
};

use super::{
    services::RbacService,
    session::{DbMokaSessionStore, SessionManager},
};

/// Shared HTTP application state.
#[derive(Debug, Clone)]
pub struct AppState {
    pub(crate) started_at: SystemTime,
    pub(crate) jobs: JobRepository,
    pub(crate) instances: JobInstanceRepository,
    pub(crate) logs: JobInstanceLogRepository,
    pub(crate) attempts: JobInstanceAttemptRepository,
    pub(crate) users: UserRepository,
    pub(crate) scripts: ScriptRepository,
    pub(crate) workflows: WorkflowRepository,
    pub(crate) audit: AuditLogRepository,
    pub(crate) alerts: AlertRepository,
    pub(crate) notification_channels: NotificationChannelRepository,
    pub(crate) notification_policies: NotificationPolicyRepository,
    pub(crate) notification_templates: NotificationTemplateRepository,
    pub(crate) notification_messages: NotificationMessageRepository,
    pub(crate) notification_delivery_attempts: NotificationDeliveryAttemptRepository,
    pub(crate) plugins: PluginRepository,
    pub(crate) auth_config: AuthConfig,
    pub(crate) transport_security: TransportSecurityConfig,
    pub(crate) observability: ObservabilityConfig,
    pub(crate) script_governance: ScriptGovernanceConfig,
    pub(crate) raft: RaftRepository,
    pub(crate) sessions: SessionManager,
    pub(crate) rbac: RbacService,
    pub(crate) registry: crate::tunnel::WorkerRegistry,
    pub(crate) worker_lifecycle: WorkerLifecycleRepository,
    pub(crate) worker_dispatch_outbox: WorkerDispatchOutboxRepository,
    pub(crate) shard_ownership: ClusterShardOwnershipRepository,
    pub(crate) cluster: SharedClusterCoordinator,
    pub(crate) raft_transport_token: Option<String>,
    pub(crate) notification_public_console_base_url: Option<String>,
}

impl AppState {
    /// Create shared HTTP state.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        jobs: JobRepository,
        instances: JobInstanceRepository,
        logs: JobInstanceLogRepository,
        attempts: JobInstanceAttemptRepository,
        users: UserRepository,
        scripts: ScriptRepository,
        workflows: WorkflowRepository,
        audit: AuditLogRepository,
        registry: crate::tunnel::WorkerRegistry,
        cluster: SharedClusterCoordinator,
    ) -> Self {
        let db = users.db();
        let rbac = RbacService::new(RbacRepository::new(db.clone()));
        let raft = RaftRepository::new(db.clone());
        let alerts = AlertRepository::new(db.clone());
        let notification_channels = NotificationChannelRepository::new(db.clone());
        let notification_policies = NotificationPolicyRepository::new(db.clone());
        let notification_templates = NotificationTemplateRepository::new(db.clone());
        let notification_messages = NotificationMessageRepository::new(db.clone());
        let notification_delivery_attempts = NotificationDeliveryAttemptRepository::new(db.clone());
        let plugins = PluginRepository::new(db.clone());
        let worker_lifecycle = WorkerLifecycleRepository::new(db.clone());
        let worker_dispatch_outbox = WorkerDispatchOutboxRepository::new(db.clone());
        let shard_ownership = ClusterShardOwnershipRepository::new(db.clone());
        let sessions = SessionManager::new(DbMokaSessionStore::new(
            AuthSessionRepository::new(db.clone()),
            RbacRepository::new(db),
        ));
        Self {
            started_at: SystemTime::now(),
            jobs,
            instances,
            logs,
            attempts,
            users,
            scripts,
            workflows,
            audit,
            alerts,
            notification_channels,
            notification_policies,
            notification_templates,
            notification_messages,
            notification_delivery_attempts,
            plugins,
            auth_config: AuthConfig::default(),
            transport_security: TransportSecurityConfig::default(),
            observability: ObservabilityConfig::default(),
            script_governance: ScriptGovernanceConfig::default(),
            raft,
            sessions,
            rbac,
            registry,
            worker_lifecycle,
            worker_dispatch_outbox,
            shard_ownership,
            cluster,
            raft_transport_token: None,
            notification_public_console_base_url: None,
        }
    }

    /// Attach auth/SSO configuration metadata.
    #[must_use]
    pub fn with_auth_config(mut self, auth_config: AuthConfig) -> Self {
        self.auth_config = auth_config;
        self
    }

    /// Attach TLS/mTLS transport security configuration metadata.
    #[must_use]
    pub fn with_transport_security_config(
        mut self,
        transport_security: TransportSecurityConfig,
    ) -> Self {
        self.transport_security = transport_security;
        self
    }

    /// Attach observability exporter configuration metadata.
    #[must_use]
    pub fn with_observability_config(mut self, observability: ObservabilityConfig) -> Self {
        self.observability = observability;
        self
    }

    /// Attach script release governance configuration.
    #[must_use]
    pub fn with_script_governance_config(
        mut self,
        script_governance: ScriptGovernanceConfig,
    ) -> Self {
        self.script_governance = script_governance;
        self
    }

    /// Attach the optional internal Raft transport token.
    #[must_use]
    pub fn with_raft_transport_token(mut self, token: Option<String>) -> Self {
        self.raft_transport_token = token.filter(|value| !value.is_empty());
        self
    }

    /// Attach the optional externally reachable public console base URL for notification links.
    #[must_use]
    pub fn with_notification_public_console_base_url(mut self, base_url: Option<String>) -> Self {
        self.notification_public_console_base_url = base_url
            .map(|value| value.trim().trim_end_matches('/').to_owned())
            .filter(|value| !value.is_empty());
        self
    }
}
