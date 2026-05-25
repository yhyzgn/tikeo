//! Shared HTTP application state.

use std::time::SystemTime;

use crate::cluster::SharedClusterCoordinator;
use tikee_config::{
    AuthConfig, ObservabilityConfig, ScriptGovernanceConfig, TransportSecurityConfig,
};
use tikee_storage::{
    AlertRepository, AuditLogRepository, AuthSessionRepository, JobInstanceAttemptRepository,
    JobInstanceLogRepository, JobInstanceRepository, JobRepository, RaftRepository, RbacRepository,
    ScriptRepository, UserRepository, WorkerLifecycleRepository, WorkflowRepository,
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
    pub(crate) auth_config: AuthConfig,
    pub(crate) transport_security: TransportSecurityConfig,
    pub(crate) observability: ObservabilityConfig,
    pub(crate) script_governance: ScriptGovernanceConfig,
    pub(crate) raft: RaftRepository,
    pub(crate) sessions: SessionManager,
    pub(crate) rbac: RbacService,
    pub(crate) registry: crate::tunnel::WorkerRegistry,
    pub(crate) worker_lifecycle: WorkerLifecycleRepository,
    pub(crate) cluster: SharedClusterCoordinator,
    pub(crate) raft_transport_token: Option<String>,
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
        let worker_lifecycle = WorkerLifecycleRepository::new(db.clone());
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
            auth_config: AuthConfig::default(),
            transport_security: TransportSecurityConfig::default(),
            observability: ObservabilityConfig::default(),
            script_governance: ScriptGovernanceConfig::default(),
            raft,
            sessions,
            rbac,
            registry,
            worker_lifecycle,
            cluster,
            raft_transport_token: None,
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
}
