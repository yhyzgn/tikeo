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
            listen_addr: SocketAddr::from(([127, 0, 0, 1], 9090)),
            worker_tunnel_addr: SocketAddr::from(([127, 0, 0, 1], 9091)),
        }
    }
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

    use super::{SchedulerConfig, load_config};

    #[test]
    fn default_config_uses_localhost_9090() {
        let config =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert_eq!(
            config.server.listen_addr,
            SocketAddr::from(([127, 0, 0, 1], 9090))
        );
    }

    #[test]
    fn default_impl_matches_loader_defaults() {
        let loaded =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert_eq!(loaded, SchedulerConfig::default());
    }
}
