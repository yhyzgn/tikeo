//! Configuration loading for tikee processes.

#![forbid(unsafe_code)]

use std::{net::SocketAddr, path::Path};

use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Top-level tikee configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TikeeConfig {
    /// HTTP/gRPC server settings.
    #[serde(default)]
    pub server: ServerConfig,
    /// Persistent storage settings.
    #[serde(default)]
    pub storage: StorageConfig,
    /// Server cluster coordination settings.
    #[serde(default)]
    pub cluster: ClusterConfig,
    /// HTTP authentication and SSO settings.
    #[serde(default)]
    pub auth: AuthConfig,
    /// TLS/mTLS transport security settings.
    #[serde(default)]
    pub transport_security: TransportSecurityConfig,
    /// Observability export settings.
    #[serde(default)]
    pub observability: ObservabilityConfig,
    /// Alert delivery retry worker settings.
    #[serde(default)]
    pub alert_retry: AlertRetryConfig,
    /// Alert provider secret management settings.
    #[serde(default)]
    pub alert_secrets: AlertSecretConfig,
    /// Script release governance settings.
    #[serde(default)]
    pub script_governance: ScriptGovernanceConfig,
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
    /// RFC3339 offset used when the application writes DB timestamps.
    pub timestamp_offset: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite://tikee-dev.db?mode=rwc".to_owned(),
            timestamp_offset: "+00:00".to_owned(),
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

/// HTTP authentication configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Keep local username/password login enabled.
    #[serde(default = "default_true")]
    pub local_login_enabled: bool,
    /// Durable API token lifecycle policy.
    #[serde(default)]
    pub api_tokens: ApiTokenConfig,
    /// Optional OIDC/SSO provider configuration.
    #[serde(default)]
    pub oidc: OidcConfig,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            local_login_enabled: true,
            api_tokens: ApiTokenConfig::default(),
            oidc: OidcConfig::default(),
        }
    }
}

/// Durable API token lifecycle policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiTokenConfig {
    /// Default token time-to-live in seconds.
    #[serde(default = "default_api_token_ttl_seconds")]
    pub default_ttl_seconds: i64,
    /// Minimum accepted requested token TTL in seconds.
    #[serde(default = "default_api_token_min_ttl_seconds")]
    pub min_ttl_seconds: i64,
    /// Maximum accepted requested token TTL in seconds.
    #[serde(default = "default_api_token_max_ttl_seconds")]
    pub max_ttl_seconds: i64,
}

impl Default for ApiTokenConfig {
    fn default() -> Self {
        Self {
            default_ttl_seconds: default_api_token_ttl_seconds(),
            min_ttl_seconds: default_api_token_min_ttl_seconds(),
            max_ttl_seconds: default_api_token_max_ttl_seconds(),
        }
    }
}

/// OIDC provider configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OidcConfig {
    /// Whether OIDC login is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// OIDC issuer URL.
    #[serde(default)]
    pub issuer_url: Option<String>,
    /// OAuth/OIDC client id.
    #[serde(default)]
    pub client_id: Option<String>,
    /// OAuth/OIDC client secret; never returned by status APIs.
    #[serde(default)]
    pub client_secret: Option<String>,
    /// Requested scopes.
    #[serde(default = "default_oidc_scopes")]
    pub scopes: Vec<String>,
}

/// Script release governance settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptGovernanceConfig {
    /// Optional `env:NAME` secret reference used to verify local release signatures.
    #[serde(default)]
    pub release_signature_secret_ref: Option<String>,
}

/// Alert provider secret reference settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertSecretConfig {
    /// Allow `env:NAME` references for alert provider secrets.
    #[serde(default = "default_true")]
    pub allow_env_refs: bool,
    /// Prefix that env secret names should use in production deployments.
    #[serde(default = "default_alert_secret_env_prefix")]
    pub env_prefix: String,
}

impl Default for AlertSecretConfig {
    fn default() -> Self {
        Self {
            allow_env_refs: true,
            env_prefix: default_alert_secret_env_prefix(),
        }
    }
}

/// Alert delivery retry worker settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertRetryConfig {
    /// Run the background alert retry worker.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Interval between due-attempt scans.
    #[serde(default = "default_alert_retry_interval_seconds")]
    pub interval_seconds: u64,
    /// Maximum due attempts scanned per iteration.
    #[serde(default = "default_alert_retry_batch_size")]
    pub batch_size: u64,
    /// Maximum delivery attempts before dead-lettering.
    #[serde(default = "default_alert_retry_max_attempts")]
    pub max_attempts: i32,
    /// Backoff before the next retry attempt.
    #[serde(default = "default_alert_retry_backoff_seconds")]
    pub backoff_seconds: i64,
}

impl Default for AlertRetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: default_alert_retry_interval_seconds(),
            batch_size: default_alert_retry_batch_size(),
            max_attempts: default_alert_retry_max_attempts(),
            backoff_seconds: default_alert_retry_backoff_seconds(),
        }
    }
}

/// Observability export configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Distributed tracing export settings.
    #[serde(default)]
    pub tracing: TracingConfig,
}

/// OpenTelemetry tracing exporter configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Enable trace exporting beyond local spans and trace-id propagation.
    #[serde(default)]
    pub enabled: bool,
    /// OTLP HTTP/gRPC collector endpoint. Redacted from status APIs.
    #[serde(default)]
    pub otlp_endpoint: Option<String>,
    /// Optional header names configured for exporter authentication/tenancy. Values live outside status APIs.
    #[serde(default)]
    pub headers: Vec<String>,
}

/// TLS/mTLS transport security configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportSecurityConfig {
    /// HTTP management listener TLS settings.
    #[serde(default)]
    pub http: TlsEndpointConfig,
    /// Worker Tunnel listener TLS/mTLS settings.
    #[serde(default)]
    pub worker_tunnel: TlsEndpointConfig,
}

/// TLS endpoint configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsEndpointConfig {
    /// Enable TLS for this endpoint.
    #[serde(default)]
    pub tls_enabled: bool,
    /// Require client certificates.
    #[serde(default)]
    pub mtls_required: bool,
    /// Server certificate path.
    #[serde(default)]
    pub cert_path: Option<String>,
    /// Server private key path.
    #[serde(default)]
    pub key_path: Option<String>,
    /// Client CA bundle path for mTLS verification.
    #[serde(default)]
    pub client_ca_path: Option<String>,
}

impl Default for OidcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            issuer_url: None,
            client_id: None,
            client_secret: None,
            scopes: default_oidc_scopes(),
        }
    }
}

const fn default_true() -> bool {
    true
}

const fn default_api_token_ttl_seconds() -> i64 {
    12 * 60 * 60
}

const fn default_api_token_min_ttl_seconds() -> i64 {
    5 * 60
}

const fn default_api_token_max_ttl_seconds() -> i64 {
    30 * 24 * 60 * 60
}

fn default_alert_secret_env_prefix() -> String {
    "TIKEE_ALERT_SECRET_".to_owned()
}

const fn default_alert_retry_interval_seconds() -> u64 {
    60
}

const fn default_alert_retry_batch_size() -> u64 {
    50
}

const fn default_alert_retry_max_attempts() -> i32 {
    3
}

const fn default_alert_retry_backoff_seconds() -> i64 {
    300
}

fn default_oidc_scopes() -> Vec<String> {
    vec![
        "openid".to_owned(),
        "profile".to_owned(),
        "email".to_owned(),
    ]
}

/// Errors raised while loading tikee configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The underlying configuration source could not be read or decoded.
    #[error("failed to load tikee configuration: {0}")]
    Load(#[from] config::ConfigError),
}

/// Load configuration from an optional TOML file plus `TIKEE__*` environment overrides.
///
/// File values override defaults; environment variables override file values.
///
/// # Errors
///
/// Returns an error when the configuration file cannot be read, a value cannot be
/// converted to the expected type, or environment overrides are invalid.
pub fn load_config(path: Option<&Path>) -> Result<TikeeConfig, ConfigError> {
    let mut builder = Config::builder()
        .set_default(
            "server.listen_addr",
            TikeeConfig::default().server.listen_addr.to_string(),
        )?
        .set_default(
            "server.worker_tunnel_addr",
            TikeeConfig::default().server.worker_tunnel_addr.to_string(),
        )?
        .set_default(
            "storage.database_url",
            TikeeConfig::default().storage.database_url,
        )?
        .set_default(
            "storage.timestamp_offset",
            TikeeConfig::default().storage.timestamp_offset,
        )?
        .set_default("cluster.mode", "standalone")?
        .set_default("cluster.node_id", TikeeConfig::default().cluster.node_id)?
        .set_default("auth.local_login_enabled", true)?
        .set_default(
            "auth.api_tokens.default_ttl_seconds",
            default_api_token_ttl_seconds(),
        )?
        .set_default(
            "auth.api_tokens.min_ttl_seconds",
            default_api_token_min_ttl_seconds(),
        )?
        .set_default(
            "auth.api_tokens.max_ttl_seconds",
            default_api_token_max_ttl_seconds(),
        )?
        .set_default("auth.oidc.enabled", false)?
        .set_default("auth.oidc.scopes", default_oidc_scopes())?
        .set_default("transport_security.http.tls_enabled", false)?
        .set_default("transport_security.http.mtls_required", false)?
        .set_default("transport_security.worker_tunnel.tls_enabled", false)?
        .set_default("transport_security.worker_tunnel.mtls_required", false)?
        .set_default("observability.tracing.enabled", false)?
        .set_default("observability.tracing.headers", Vec::<String>::new())?
        .set_default("alert_secrets.allow_env_refs", true)?
        .set_default(
            "alert_secrets.env_prefix",
            default_alert_secret_env_prefix(),
        )?
        .set_default("alert_retry.enabled", true)?
        .set_default("alert_retry.interval_seconds", 60)?
        .set_default("alert_retry.batch_size", 50)?
        .set_default("alert_retry.max_attempts", 3)?
        .set_default("alert_retry.backoff_seconds", 300)?;

    if let Some(path) = path {
        builder = builder.add_source(File::from(path).required(true));
    }

    let config = builder
        .add_source(Environment::with_prefix("TIKEE").separator("__"))
        .build()?;

    Ok(config.try_deserialize()?)
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::{ClusterModeConfig, TikeeConfig, load_config};

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
    fn default_auth_config_keeps_local_login_and_disables_oidc() {
        let config =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert!(config.auth.local_login_enabled);
        assert_eq!(config.auth.api_tokens.default_ttl_seconds, 43_200);
        assert_eq!(config.auth.api_tokens.min_ttl_seconds, 300);
        assert_eq!(config.auth.api_tokens.max_ttl_seconds, 2_592_000);
        assert!(!config.auth.oidc.enabled);
        assert_eq!(config.auth.oidc.scopes, ["openid", "profile", "email"]);
    }

    #[test]
    fn default_transport_security_config_keeps_dev_plaintext() {
        let config =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert!(!config.transport_security.http.tls_enabled);
        assert!(!config.transport_security.worker_tunnel.tls_enabled);
        assert!(!config.transport_security.worker_tunnel.mtls_required);
    }

    #[test]
    fn default_observability_config_disables_otlp_export() {
        let config =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert!(!config.observability.tracing.enabled);
        assert!(config.observability.tracing.otlp_endpoint.is_none());
        assert!(config.observability.tracing.headers.is_empty());
    }

    #[test]
    fn default_alert_retry_config_enables_bounded_background_scheduler() {
        let config =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert!(config.alert_retry.enabled);
        assert_eq!(config.alert_retry.interval_seconds, 60);
        assert_eq!(config.alert_retry.batch_size, 50);
        assert_eq!(config.alert_retry.max_attempts, 3);
        assert_eq!(config.alert_retry.backoff_seconds, 300);
    }

    #[test]
    fn default_alert_secret_config_allows_prefixed_env_refs() {
        let config =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert!(config.alert_secrets.allow_env_refs);
        assert_eq!(config.alert_secrets.env_prefix, "TIKEE_ALERT_SECRET_");
    }

    #[test]
    fn default_script_governance_keeps_signature_verification_disabled() {
        let config =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert!(
            config
                .script_governance
                .release_signature_secret_ref
                .is_none()
        );
    }

    #[test]
    fn default_impl_matches_loader_defaults() {
        let loaded =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert_eq!(loaded, TikeeConfig::default());
    }
}
