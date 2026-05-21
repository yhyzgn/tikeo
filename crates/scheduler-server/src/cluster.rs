//! Server cluster coordination surfaces.
//!
//! Phase 2 intentionally starts with an explicit standalone coordinator so the
//! management API stops pretending that a real Raft leader exists. Real Raft
//! consensus will replace this coordinator behind the same trait boundary.

use std::sync::Arc;

use scheduler_config::{ClusterConfig, ClusterModeConfig};

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
    /// Whether this node may own scheduler/dispatcher loops.
    pub can_schedule: bool,
    /// Human-readable implementation note.
    pub detail: String,
}

/// Build a cluster coordinator from process configuration.
#[must_use]
pub fn coordinator_from_config(config: &ClusterConfig) -> SharedClusterCoordinator {
    match config.mode {
        ClusterModeConfig::Standalone => StandaloneCoordinator::shared(config.node_id.clone()),
        ClusterModeConfig::Raft => StaticCoordinator::shared(ClusterStatus {
            mode: ClusterMode::Raft,
            role: ClusterRole::Unknown,
            node_id: config.node_id.clone(),
            nodes: u32::try_from(config.peers.len()).unwrap_or(u32::MAX).max(1),
            can_schedule: false,
            detail: "raft mode configured but consensus runtime is not started yet".to_owned(),
        }),
    }
}

/// Cluster coordinator boundary used by HTTP and future scheduling gates.
#[async_trait::async_trait]
pub trait ClusterCoordinator: Send + Sync + std::fmt::Debug {
    /// Return current cluster status.
    async fn status(&self) -> ClusterStatus;
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
            detail: "standalone node; raft consensus is not enabled".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use scheduler_config::{ClusterConfig, ClusterModeConfig, ClusterPeerConfig};

    use super::{
        ClusterCoordinator, ClusterMode, ClusterRole, StandaloneCoordinator,
        coordinator_from_config,
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
    async fn raft_config_status_is_unknown_and_not_schedulable() {
        let coordinator = coordinator_from_config(&ClusterConfig {
            mode: ClusterModeConfig::Raft,
            node_id: "scheduler-1".to_owned(),
            peers: vec![
                ClusterPeerConfig {
                    node_id: "scheduler-1".to_owned(),
                    endpoint: "http://scheduler-1:9999".to_owned(),
                },
                ClusterPeerConfig {
                    node_id: "scheduler-2".to_owned(),
                    endpoint: "http://scheduler-2:9999".to_owned(),
                },
            ],
        });
        let status = coordinator.status().await;

        assert_eq!(status.mode, ClusterMode::Raft);
        assert_eq!(status.role, ClusterRole::Unknown);
        assert_eq!(status.node_id, "scheduler-1");
        assert_eq!(status.nodes, 2);
        assert!(!status.can_schedule);
    }
}
