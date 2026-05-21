//! Configuration loading for scheduler processes.

#![forbid(unsafe_code)]

use std::{net::SocketAddr, path::Path};

use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Top-level scheduler configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// HTTP/gRPC server settings.
    #[serde(default)]
    pub server: ServerConfig,
    /// Persistent storage settings.
    #[serde(default)]
    pub storage: StorageConfig,
    /// Server cluster coordination settings.
    #[serde(default)]
    pub cluster: ClusterConfig,
}

/// Server listener configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Address used by the HTTP management API listener.
    pub listen_addr: SocketAddr,
    /// Address used by the gRPC Worker Tunnel listener.
    pub worker_tunnel_addr: SocketAddr,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: SocketAddr::from(([0, 0, 0, 0], 9090)),
            worker_tunnel_addr: SocketAddr::from(([0, 0, 0, 0], 9998)),
        }
    }
}

/// Persistent storage configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Database URL consumed by SeaORM/sqlx.
    pub database_url: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite://scheduler-dev.db?mode=rwc".to_owned(),
        }
    }
}

/// Server cluster coordination settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Cluster mode: `standalone` or `raft`.
    #[serde(default)]
    pub mode: ClusterModeConfig,
    /// Stable node id used in cluster status and future Raft membership.
    pub node_id: String,
    /// Static peer list for future Raft bootstrap.
    #[serde(default)]
    pub peers: Vec<ClusterPeerConfig>,
    /// Optional shared token for internal Raft HTTP transport.
    #[serde(default)]
    pub transport_token: Option<String>,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            mode: ClusterModeConfig::Standalone,
            node_id: "standalone".to_owned(),
            peers: Vec::new(),
            transport_token: None,
        }
    }
}

/// Cluster mode configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterModeConfig {
    /// Single-node standalone mode.
    #[default]
    Standalone,
    /// Raft mode. Consensus implementation is still gated behind server cluster work.
    Raft,
}

/// Static cluster peer configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterPeerConfig {
    /// Peer node id.
    pub node_id: String,
    /// Peer-to-peer endpoint URL or host:port reachable through container/K8s networking.
    pub endpoint: String,
}

/// Errors raised while loading scheduler configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The underlying configuration source could not be read or decoded.
    #[error("failed to load scheduler configuration: {0}")]
    Load(#[from] config::ConfigError),
}

/// Load configuration from an optional TOML file plus `SCHEDULER__*` environment overrides.
///
/// File values override defaults; environment variables override file values.
///
/// # Errors
///
/// Returns an error when the configuration file cannot be read, a value cannot be
/// converted to the expected type, or environment overrides are invalid.
pub fn load_config(path: Option<&Path>) -> Result<SchedulerConfig, ConfigError> {
    let mut builder = Config::builder()
        .set_default(
            "server.listen_addr",
            SchedulerConfig::default().server.listen_addr.to_string(),
        )?
        .set_default(
            "server.worker_tunnel_addr",
            SchedulerConfig::default()
                .server
                .worker_tunnel_addr
                .to_string(),
        )?
        .set_default(
            "storage.database_url",
            SchedulerConfig::default().storage.database_url,
        )?
        .set_default("cluster.mode", "standalone")?
        .set_default(
            "cluster.node_id",
            SchedulerConfig::default().cluster.node_id,
        )?;

    if let Some(path) = path {
        builder = builder.add_source(File::from(path).required(true));
    }

    let config = builder
        .add_source(Environment::with_prefix("SCHEDULER").separator("__"))
        .build()?;

    Ok(config.try_deserialize()?)
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::{ClusterModeConfig, SchedulerConfig, load_config};

    #[test]
    fn default_config_listens_on_all_interfaces() {
        let config =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert_eq!(
            config.server.listen_addr,
            SocketAddr::from(([0, 0, 0, 0], 9090))
        );
    }

    #[test]
    fn default_cluster_config_is_standalone() {
        let config =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert_eq!(config.cluster.mode, ClusterModeConfig::Standalone);
        assert_eq!(config.cluster.node_id, "standalone");
        assert!(config.cluster.peers.is_empty());
    }

    #[test]
    fn default_impl_matches_loader_defaults() {
        let loaded =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert_eq!(loaded, SchedulerConfig::default());
    }
}
