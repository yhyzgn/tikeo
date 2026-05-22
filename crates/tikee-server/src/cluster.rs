//! Server cluster coordination surfaces.
//!
//! Phase 2 intentionally starts with an explicit standalone coordinator so the
//! management API stops pretending that a real Raft leader exists. Real Raft
//! consensus will replace this coordinator behind the same trait boundary.

use std::sync::Arc;

mod raft_rs;

use raft::eraftpb::Message;
use tikee_config::{ClusterConfig, ClusterModeConfig};
use tikee_storage::{RaftRepository, UpsertRaftMember, UpsertRaftMetadata};

use self::raft_rs::{RAFT_RS_LIBRARY, RaftRuntimeCoordinator, validate_raft_rs_bootstrap};

/// Cluster operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterMode {
    /// Single server process; no consensus membership.
    Standalone,
    /// Raft-backed multi-server cluster.
    Raft,
}

impl ClusterMode {
    /// Stable wire value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standalone => "standalone",
            Self::Raft => "raft",
        }
    }
}

/// Current node role inside the cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterRole {
    /// Standalone node owns local scheduling loops.
    Standalone,
    /// Raft leader owns cluster-wide scheduling loops.
    Leader,
    /// Raft follower must not run ownership-sensitive scheduling loops.
    Follower,
    /// Node is starting or cannot determine role.
    Unknown,
}

impl ClusterRole {
    /// Stable wire value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standalone => "standalone",
            Self::Leader => "leader",
            Self::Follower => "follower",
            Self::Unknown => "unknown",
        }
    }
}

/// Cluster status reported to management clients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterStatus {
    /// Cluster mode.
    pub mode: ClusterMode,
    /// Current node role.
    pub role: ClusterRole,
    /// Stable current node identifier.
    pub node_id: String,
    /// Known server node count.
    pub nodes: u32,
    /// Whether this node may own tikee/dispatcher loops.
    pub can_schedule: bool,
    /// Optional leader fencing token; only real consensus may populate it.
    pub leader_fencing_token: Option<String>,
    /// Human-readable implementation note.
    pub detail: String,
}

/// Result of submitting an inbound raft-rs transport message to the local runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftMessageSubmission {
    /// Whether the message was accepted by the local runtime inbox.
    pub accepted: bool,
    /// Human-readable submission result.
    pub reason: String,
}

/// Validated Raft membership proposal passed from HTTP to the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftMembershipProposal {
    /// Client-provided proposal idempotency key.
    pub proposal_id: String,
    /// Membership action, for example `add_voter` or `remove_voter`.
    pub action: String,
    /// Target tikee node id.
    pub node_id: String,
    /// Peer endpoint captured in the proposal context.
    pub endpoint: Option<String>,
}

/// Result of submitting a membership proposal to the local raft-rs runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftMembershipProposalSubmission {
    /// Whether the runtime accepted the proposal for raft-rs processing.
    pub accepted: bool,
    /// Human-readable submission result.
    pub reason: String,
}

impl RaftMembershipProposalSubmission {
    /// Accepted by a running runtime.
    #[must_use]
    pub fn accepted(reason: impl Into<String>) -> Self {
        Self {
            accepted: true,
            reason: reason.into(),
        }
    }

    /// Rejected because the current coordinator cannot safely propose membership.
    #[must_use]
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            accepted: false,
            reason: reason.into(),
        }
    }
}

impl RaftMessageSubmission {
    /// Accepted by a running runtime inbox.
    #[must_use]
    pub fn accepted(message_type: impl std::fmt::Debug) -> Self {
        Self {
            accepted: true,
            reason: format!("raft-rs {message_type:?} message enqueued for runtime processing"),
        }
    }

    /// Rejected because the current coordinator cannot consume raft-rs messages.
    #[must_use]
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            accepted: false,
            reason: reason.into(),
        }
    }
}

/// Build a cluster coordinator from process configuration without storage bootstrap.
#[must_use]
pub fn coordinator_from_config(config: &ClusterConfig) -> SharedClusterCoordinator {
    match config.mode {
        ClusterModeConfig::Standalone => StandaloneCoordinator::shared(config.node_id.clone()),
        ClusterModeConfig::Raft => raft_blocked_coordinator(
            config,
            "raft mode configured but consensus runtime is not started yet",
        ),
    }
}

/// Build a cluster coordinator and persist safe Raft bootstrap metadata.
///
/// This is intentionally not a Raft runtime: it records node/member metadata and keeps
/// `can_schedule=false` until real consensus establishes leadership.
///
/// # Errors
///
/// Returns an error when storage persistence fails.
pub async fn coordinator_from_config_with_storage(
    config: &ClusterConfig,
    repository: &RaftRepository,
) -> Result<SharedClusterCoordinator, tikee_storage::DbErr> {
    match config.mode {
        ClusterModeConfig::Standalone => Ok(StandaloneCoordinator::shared(config.node_id.clone())),
        ClusterModeConfig::Raft => {
            repository
                .upsert_metadata(UpsertRaftMetadata {
                    cluster_id: "default".to_owned(),
                    node_id: config.node_id.clone(),
                    current_term: 0,
                    voted_for: None,
                    commit_index: 0,
                    applied_index: 0,
                    leader_fencing_token: None,
                    conf_state: None,
                })
                .await?;
            for peer in &config.peers {
                repository
                    .upsert_member(UpsertRaftMember {
                        node_id: peer.node_id.clone(),
                        endpoint: peer.endpoint.clone(),
                        status: "configured".to_owned(),
                    })
                    .await?;
            }
            RaftRuntimeCoordinator::start(config, repository.clone()).await
        }
    }
}

fn raft_blocked_coordinator(config: &ClusterConfig, detail: &str) -> SharedClusterCoordinator {
    let runtime_detail = raft_runtime_detail(config);
    StaticCoordinator::shared(ClusterStatus {
        mode: ClusterMode::Raft,
        role: ClusterRole::Unknown,
        node_id: config.node_id.clone(),
        nodes: u32::try_from(config.peers.len()).unwrap_or(u32::MAX).max(1),
        can_schedule: false,
        leader_fencing_token: None,
        detail: format!("{detail}; {runtime_detail}"),
    })
}

fn raft_runtime_detail(config: &ClusterConfig) -> String {
    match validate_raft_rs_bootstrap(config) {
        Ok(status) => format!(
            "{RAFT_RS_LIBRARY} bootstrap validated: raft_node_id={}, voters={}, initial_role={}, has_ready={}; consensus event loop is not started yet",
            status.raft_node_id,
            status.voter_ids.len(),
            status.initial_role,
            status.has_ready
        ),
        Err(error) => format!(
            "{RAFT_RS_LIBRARY} bootstrap validation failed: {error}; consensus event loop is not started yet"
        ),
    }
}

/// Cluster coordinator boundary used by HTTP and future scheduling gates.
#[async_trait::async_trait]
pub trait ClusterCoordinator: Send + Sync + std::fmt::Debug {
    /// Return current cluster status.
    async fn status(&self) -> ClusterStatus;

    /// Submit an inbound raft-rs transport message to the local runtime.
    async fn submit_raft_message(&self, message: Message) -> RaftMessageSubmission {
        let _message = message;
        RaftMessageSubmission::unavailable(
            "raft-rs runtime inbox is not available for this coordinator",
        )
    }

    /// Submit a validated membership proposal to the local raft-rs runtime.
    async fn propose_membership_change(
        &self,
        proposal: RaftMembershipProposal,
    ) -> RaftMembershipProposalSubmission {
        let _proposal = proposal;
        RaftMembershipProposalSubmission::unavailable(
            "raft-rs membership proposal runtime is not available for this coordinator",
        )
    }
}

/// Shared cluster coordinator handle.
pub type SharedClusterCoordinator = Arc<dyn ClusterCoordinator>;

/// Static coordinator useful for tests and future externally-driven roles.
#[derive(Debug, Clone)]
pub struct StaticCoordinator {
    status: ClusterStatus,
}

impl StaticCoordinator {
    /// Create a static coordinator from a fixed status.
    #[must_use]
    pub const fn new(status: ClusterStatus) -> Self {
        Self { status }
    }

    /// Create a shared static coordinator.
    #[must_use]
    pub fn shared(status: ClusterStatus) -> SharedClusterCoordinator {
        Arc::new(Self::new(status))
    }
}

#[async_trait::async_trait]
impl ClusterCoordinator for StaticCoordinator {
    async fn status(&self) -> ClusterStatus {
        self.status.clone()
    }
}

/// Standalone coordinator used until Raft is implemented.
#[derive(Debug, Clone)]
pub struct StandaloneCoordinator {
    node_id: String,
}

impl StandaloneCoordinator {
    /// Create a standalone coordinator with a stable node id.
    #[must_use]
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
        }
    }

    /// Create a shared standalone coordinator.
    #[must_use]
    pub fn shared(node_id: impl Into<String>) -> SharedClusterCoordinator {
        Arc::new(Self::new(node_id))
    }
}

#[async_trait::async_trait]
impl ClusterCoordinator for StandaloneCoordinator {
    async fn status(&self) -> ClusterStatus {
        ClusterStatus {
            mode: ClusterMode::Standalone,
            role: ClusterRole::Standalone,
            node_id: self.node_id.clone(),
            nodes: 1,
            can_schedule: true,
            leader_fencing_token: None,
            detail: "standalone node; raft consensus is not enabled".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use tikee_config::{ClusterConfig, ClusterModeConfig, ClusterPeerConfig};
    use tikee_storage::{RaftRepository, connect_and_migrate};

    use super::{
        ClusterCoordinator, ClusterMode, ClusterRole, StandaloneCoordinator,
        coordinator_from_config, coordinator_from_config_with_storage,
    };

    #[tokio::test]
    async fn standalone_status_is_explicit_not_fake_leader() {
        let coordinator = StandaloneCoordinator::new("node-a");
        let status = coordinator.status().await;

        assert_eq!(status.mode, ClusterMode::Standalone);
        assert_eq!(status.role, ClusterRole::Standalone);
        assert_eq!(status.node_id, "node-a");
        assert_eq!(status.nodes, 1);
        assert!(status.can_schedule);
    }

    #[tokio::test]
    async fn raft_config_persists_bootstrap_metadata_but_remains_unschedulable() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should initialize: {error}"));
        let repository = RaftRepository::new(db);
        let config = ClusterConfig {
            mode: ClusterModeConfig::Raft,
            node_id: "tikee-1".to_owned(),
            peers: vec![ClusterPeerConfig {
                node_id: "tikee-1".to_owned(),
                endpoint: "http://tikee-1:9999".to_owned(),
            }],
            transport_token: None,
        };

        let coordinator = coordinator_from_config_with_storage(&config, &repository)
            .await
            .unwrap_or_else(|error| panic!("coordinator should initialize: {error}"));
        let status = coordinator.status().await;
        let metadata = repository
            .get_metadata("tikee-1")
            .await
            .unwrap_or_else(|error| panic!("metadata should load: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));
        let members = repository
            .list_members()
            .await
            .unwrap_or_else(|error| panic!("members should load: {error}"));

        assert_eq!(status.mode, ClusterMode::Raft);
        assert_eq!(status.role, ClusterRole::Follower);
        assert!(!status.can_schedule);
        assert_eq!(metadata.current_term, 0);
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].node_id, "tikee-1");
    }

    #[tokio::test]
    async fn raft_config_status_is_unknown_and_not_schedulable() {
        let coordinator = coordinator_from_config(&ClusterConfig {
            mode: ClusterModeConfig::Raft,
            node_id: "tikee-1".to_owned(),
            peers: vec![
                ClusterPeerConfig {
                    node_id: "tikee-1".to_owned(),
                    endpoint: "http://tikee-1:9999".to_owned(),
                },
                ClusterPeerConfig {
                    node_id: "tikee-2".to_owned(),
                    endpoint: "http://tikee-2:9999".to_owned(),
                },
            ],
            transport_token: None,
        });
        let status = coordinator.status().await;

        assert_eq!(status.mode, ClusterMode::Raft);
        assert_eq!(status.role, ClusterRole::Unknown);
        assert_eq!(status.node_id, "tikee-1");
        assert_eq!(status.nodes, 2);
        assert!(!status.can_schedule);
    }
}
