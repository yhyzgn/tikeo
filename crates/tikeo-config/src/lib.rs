//! Configuration loading for tikeo processes.

#![forbid(unsafe_code)]

use std::{collections::BTreeMap, fmt::Write as _, net::SocketAddr, path::Path};

use config::{Config, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Top-level tikeo configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TikeoConfig {
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
    /// Generic Notification Center delivery worker settings.
    #[serde(default)]
    pub notification_delivery: NotificationDeliveryConfig,
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
    /// Structured database connection settings.
    #[serde(default)]
    pub database: DatabaseConfig,
    /// RFC3339 offset used when the application writes DB timestamps.
    pub timestamp_offset: String,
}

impl StorageConfig {
    /// Build the effective SeaORM/sqlx connection URL consumed by storage.
    #[must_use]
    /// Effective connection url.
    pub fn effective_connection_url(&self) -> String {
        self.database
            .to_url()
            .unwrap_or_else(|| DatabaseConfig::default().to_url().unwrap_or_default())
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            timestamp_offset: "+00:00".to_owned(),
        }
    }
}

/// Structured database settings that avoid URL escaping issues for passwords containing `@`, `/`, `:`, etc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database driver: `sqlite`, `postgres`, `mysql`, or `cockroachdb`.
    #[serde(rename = "type", default = "default_database_type")]
    pub kind: String,
    /// Hostname for network databases.
    #[serde(default)]
    pub host: Option<String>,
    /// Port for network databases.
    #[serde(default)]
    pub port: Option<u16>,
    /// Username for network databases.
    #[serde(default)]
    pub username: Option<String>,
    /// Password for network databases. May contain special characters without URL escaping.
    #[serde(default)]
    pub password: Option<String>,
    /// Database/schema name for network databases.
    #[serde(default)]
    pub database: Option<String>,
    /// `SQLite` file path.
    #[serde(default = "default_sqlite_path")]
    pub path: String,
    /// Query parameters appended to the generated URL.
    #[serde(default = "default_database_params")]
    pub params: BTreeMap<String, String>,
}

impl DatabaseConfig {
    /// Convert structured settings to the connection URL expected by SeaORM/sqlx.
    #[must_use]
    /// To url.
    pub fn to_url(&self) -> Option<String> {
        match self.kind.trim().to_ascii_lowercase().as_str() {
            "sqlite" => {
                let mut url = format!("sqlite://{}", self.path);
                let mut params = self.params.clone();
                if params.is_empty() {
                    params.insert("mode".to_owned(), "rwc".to_owned());
                }
                append_query_params(&mut url, &params);
                Some(url)
            }
            "postgres" | "postgresql" | "cockroach" | "cockroachdb" => {
                Some(self.network_url("postgres", 5432))
            }
            "mysql" | "mariadb" => Some(self.network_url("mysql", 3306)),
            _ => None,
        }
    }

    fn network_url(&self, scheme: &str, default_port: u16) -> String {
        let host = self.host.as_deref().unwrap_or("127.0.0.1");
        let port = self.port.unwrap_or(default_port);
        let database = self.database.as_deref().unwrap_or("tikeo");
        let mut url = self
            .username
            .as_deref()
            .filter(|value| !value.is_empty())
            .map_or_else(
                || format!("{scheme}://{host}:{port}/{database}"),
                |username| {
                    let encoded_username = percent_encode_url_component(username);
                    let encoded_password = self
                        .password
                        .as_deref()
                        .map(percent_encode_url_component)
                        .unwrap_or_default();
                    format!(
                        "{scheme}://{encoded_username}:{encoded_password}@{host}:{port}/{database}"
                    )
                },
            );
        append_query_params(&mut url, &self.params);
        url
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            kind: default_database_type(),
            host: None,
            port: None,
            username: None,
            password: None,
            database: None,
            path: default_sqlite_path(),
            params: default_database_params(),
        }
    }
}

fn default_database_type() -> String {
    "sqlite".to_owned()
}

fn default_sqlite_path() -> String {
    ".dev/tikeo-dev.db".to_owned()
}

const fn default_database_params() -> BTreeMap<String, String> {
    BTreeMap::new()
}

fn append_query_params(url: &mut String, params: &BTreeMap<String, String>) {
    if params.is_empty() {
        return;
    }
    url.push('?');
    for (index, (key, value)) in params.iter().enumerate() {
        if index > 0 {
            url.push('&');
        }
        url.push_str(&percent_encode_url_component(key));
        url.push('=');
        url.push_str(&percent_encode_url_component(value));
    }
}

fn percent_encode_url_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => write!(&mut encoded, "%{byte:02X}")
                .unwrap_or_else(|error| unreachable!("writing to String cannot fail: {error}")),
        }
    }
    encoded
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
    /// Monotonic scheduler shard map version used by dispatch queue hashing.
    #[serde(default = "default_scheduler_shard_map_version")]
    pub scheduler_shard_map_version: i64,
    /// Number of logical scheduler shards in the current shard map.
    #[serde(default = "default_scheduler_shard_count")]
    pub scheduler_shard_count: i32,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            mode: ClusterModeConfig::Standalone,
            node_id: "standalone".to_owned(),
            peers: Vec::new(),
            transport_token: None,
            scheduler_shard_map_version: default_scheduler_shard_map_version(),
            scheduler_shard_count: default_scheduler_shard_count(),
        }
    }
}

const fn default_scheduler_shard_map_version() -> i64 {
    1
}

const fn default_scheduler_shard_count() -> i32 {
    64
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
    /// Local login enabled value.
    pub local_login_enabled: bool,
    /// Durable API token lifecycle policy.
    #[serde(default)]
    /// Api tokens value.
    pub api_tokens: ApiTokenConfig,
    /// Optional OIDC/SSO provider configuration.
    #[serde(default)]
    /// Oidc value.
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
    /// Default ttl seconds value.
    pub default_ttl_seconds: i64,
    /// Minimum accepted requested token TTL in seconds.
    #[serde(default = "default_api_token_min_ttl_seconds")]
    /// Min ttl seconds value.
    pub min_ttl_seconds: i64,
    /// Maximum accepted requested token TTL in seconds.
    #[serde(default = "default_api_token_max_ttl_seconds")]
    /// Max ttl seconds value.
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
    /// Boolean state flag.
    pub enabled: bool,
    /// OIDC issuer URL.
    #[serde(default)]
    /// Issuer url value.
    pub issuer_url: Option<String>,
    /// OAuth/OIDC client id.
    #[serde(default)]
    /// Identifier value.
    pub client_id: Option<String>,
    /// OAuth/OIDC client secret; never returned by status APIs.
    #[serde(default)]
    /// Client secret value.
    pub client_secret: Option<String>,
    /// Requested scopes.
    #[serde(default = "default_oidc_scopes")]
    /// Scopes value.
    pub scopes: Vec<String>,
}

/// Script release governance settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptGovernanceConfig {
    /// Optional `env:NAME` secret reference used to verify local release signatures.
    #[serde(default)]
    /// Release signature secret ref value.
    pub release_signature_secret_ref: Option<String>,
}

/// Alert provider secret reference settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertSecretConfig {
    /// Allow `env:NAME` references for alert provider secrets.
    #[serde(default = "default_true")]
    /// Allow env refs value.
    pub allow_env_refs: bool,
    /// Prefix that env secret names should use in production deployments.
    #[serde(default = "default_alert_secret_env_prefix")]
    /// Env prefix value.
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
    /// Boolean state flag.
    pub enabled: bool,
    /// Interval between due-attempt scans.
    #[serde(default = "default_alert_retry_interval_seconds")]
    /// Interval seconds value.
    pub interval_seconds: u64,
    /// Maximum due attempts scanned per iteration.
    #[serde(default = "default_alert_retry_batch_size")]
    /// Batch size value.
    pub batch_size: u64,
    /// Maximum delivery attempts before dead-lettering.
    #[serde(default = "default_alert_retry_max_attempts")]
    /// Max attempts value.
    pub max_attempts: i32,
    /// Backoff before the next retry attempt.
    #[serde(default = "default_alert_retry_backoff_seconds")]
    /// Backoff seconds value.
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

/// Generic Notification Center delivery worker settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationDeliveryConfig {
    /// Run the background Notification Center delivery worker.
    #[serde(default = "default_true")]
    /// Boolean state flag.
    pub enabled: bool,
    /// Optional externally reachable Web base URL used for notification card public-console links.
    #[serde(default)]
    /// Public console base url value.
    pub public_console_base_url: Option<String>,
    /// Interval between due-attempt scans.
    #[serde(default = "default_notification_delivery_interval_seconds")]
    /// Interval seconds value.
    pub interval_seconds: u64,
    /// Maximum due attempts scanned per iteration.
    #[serde(default = "default_notification_delivery_batch_size")]
    /// Batch size value.
    pub batch_size: u64,
    /// Maximum delivery attempts before dead-lettering.
    #[serde(default = "default_notification_delivery_max_attempts")]
    /// Max attempts value.
    pub max_attempts: i32,
    /// Backoff before the next retry attempt.
    #[serde(default = "default_notification_delivery_backoff_seconds")]
    /// Backoff seconds value.
    pub backoff_seconds: i64,
}

impl Default for NotificationDeliveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            public_console_base_url: None,
            interval_seconds: default_notification_delivery_interval_seconds(),
            batch_size: default_notification_delivery_batch_size(),
            max_attempts: default_notification_delivery_max_attempts(),
            backoff_seconds: default_notification_delivery_backoff_seconds(),
        }
    }
}

/// Observability export configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Local and remote runtime logging sinks.
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Distributed tracing export settings.
    #[serde(default)]
    pub tracing: TracingConfig,
}

/// Runtime logging configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Deprecated flat minimum log level accepted for backward compatibility.
    #[serde(default)]
    pub level: Option<String>,
    /// Deprecated flat log directory accepted for backward compatibility.
    #[serde(default)]
    pub log_dir: Option<String>,
    /// Root log filter.
    #[serde(default)]
    pub root: LogLevelConfig,
    /// Console sink.
    #[serde(default)]
    pub console: LogSinkConfig,
    /// Main file sink.
    #[serde(default)]
    pub file: FileLogSinkConfig,
    /// Error-only file sink.
    #[serde(default, rename = "error-file", alias = "error_file")]
    pub error_file: FileLogSinkConfig,
    /// ELK/log collector sink.
    #[serde(default)]
    pub elk: ElkLogConfig,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: None,
            log_dir: None,
            root: LogLevelConfig::default(),
            console: LogSinkConfig::default(),
            file: FileLogSinkConfig::default(),
            error_file: FileLogSinkConfig {
                enabled: false,
                level: "error".to_owned(),
                path: default_app_log_path(),
            },
            elk: ElkLogConfig::default(),
        }
    }
}

/// Single log level block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogLevelConfig {
    /// Minimum level.
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LogLevelConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

/// Console-like log sink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogSinkConfig {
    /// Enable this sink.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Minimum level for this sink.
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LogSinkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: default_log_level(),
        }
    }
}

/// File log sink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileLogSinkConfig {
    /// Enable this sink.
    #[serde(default)]
    pub enabled: bool,
    /// Minimum level for this sink.
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Log directory path.
    #[serde(default = "default_app_log_path")]
    pub path: String,
}

impl Default for FileLogSinkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            level: default_log_level(),
            path: default_app_log_path(),
        }
    }
}

/// ELK/log collector sink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElkLogConfig {
    /// Enable this sink.
    #[serde(default)]
    pub enabled: bool,
    /// Comma-separated collector endpoints in `host:port` form.
    #[serde(default = "default_elk_servers")]
    pub servers: String,
    /// Collector topic or logical index.
    #[serde(default = "default_elk_topic")]
    pub topic: String,
    /// Minimum level for this sink.
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Optional SASL metadata for compatible collectors.
    #[serde(default)]
    pub sasl: ElkSaslConfig,
}

impl Default for ElkLogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            servers: default_elk_servers(),
            topic: default_elk_topic(),
            level: default_log_level(),
            sasl: ElkSaslConfig::default(),
        }
    }
}

/// ELK/log collector SASL metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElkSaslConfig {
    /// Enable SASL metadata on collector connections.
    #[serde(default)]
    pub enabled: bool,
    /// SASL username.
    #[serde(default)]
    pub username: String,
    /// SASL password.
    #[serde(default)]
    pub password: String,
}

/// OpenTelemetry tracing exporter configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Enable trace exporting beyond local spans and trace-id propagation.
    #[serde(default)]
    /// Boolean state flag.
    pub enabled: bool,
    /// OTLP HTTP/gRPC collector endpoint. Redacted from status APIs.
    #[serde(default)]
    /// Otlp endpoint value.
    pub otlp_endpoint: Option<String>,
    /// Optional header names configured for exporter authentication/tenancy. Values live outside status APIs.
    #[serde(default)]
    /// Headers value.
    pub headers: Vec<String>,
}

/// TLS/mTLS transport security configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportSecurityConfig {
    /// HTTP management listener TLS settings.
    #[serde(default)]
    /// Http value.
    pub http: TlsEndpointConfig,
    /// Worker Tunnel listener TLS/mTLS settings.
    #[serde(default)]
    /// Worker tunnel value.
    pub worker_tunnel: TlsEndpointConfig,
}

/// TLS endpoint configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsEndpointConfig {
    /// Enable TLS for this endpoint.
    #[serde(default)]
    /// Tls enabled value.
    pub tls_enabled: bool,
    /// Require client certificates.
    #[serde(default)]
    /// Mtls required value.
    pub mtls_required: bool,
    /// Server certificate path.
    #[serde(default)]
    /// Cert path value.
    pub cert_path: Option<String>,
    /// Server private key path.
    #[serde(default)]
    /// Key path value.
    pub key_path: Option<String>,
    /// Client CA bundle path for mTLS verification.
    #[serde(default)]
    /// Client ca path value.
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

fn default_log_level() -> String {
    "info".to_owned()
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
    "TIKEO_ALERT_SECRET_".to_owned()
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

const fn default_notification_delivery_interval_seconds() -> u64 {
    60
}

const fn default_notification_delivery_batch_size() -> u64 {
    50
}

const fn default_notification_delivery_max_attempts() -> i32 {
    3
}

const fn default_notification_delivery_backoff_seconds() -> i64 {
    300
}

fn default_oidc_scopes() -> Vec<String> {
    vec![
        "openid".to_owned(),
        "profile".to_owned(),
        "email".to_owned(),
    ]
}

fn default_app_log_path() -> String {
    "/logs".to_owned()
}

fn default_elk_servers() -> String {
    "203.83.233.63:8094,36.111.150.189:8094,106.63.7.44:8094".to_owned()
}

fn default_elk_topic() -> String {
    "ivs-dev".to_owned()
}

/// Errors raised while loading tikeo configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The underlying configuration source could not be read or decoded.
    #[error("failed to load tikeo configuration: {0}")]
    Load(#[from] config::ConfigError),
}

/// Load configuration from an optional YAML file plus `TIKEO__*` environment overrides.
///
/// File values override defaults; environment variables override file values.
///
/// # Errors
///
/// Returns an error when the configuration file cannot be read, a value cannot be
/// converted to the expected type, or environment overrides are invalid.
pub fn load_config(path: Option<&Path>) -> Result<TikeoConfig, ConfigError> {
    let mut builder = Config::builder()
        .set_default(
            "server.listen_addr",
            TikeoConfig::default().server.listen_addr.to_string(),
        )?
        .set_default(
            "server.worker_tunnel_addr",
            TikeoConfig::default().server.worker_tunnel_addr.to_string(),
        )?
        .set_default("storage.database.type", default_database_type())?
        .set_default("storage.database.path", default_sqlite_path())?
        .set_default(
            "storage.timestamp_offset",
            TikeoConfig::default().storage.timestamp_offset,
        )?
        .set_default("cluster.mode", "standalone")?
        .set_default("cluster.node_id", TikeoConfig::default().cluster.node_id)?
        .set_default(
            "cluster.scheduler_shard_map_version",
            default_scheduler_shard_map_version(),
        )?
        .set_default(
            "cluster.scheduler_shard_count",
            default_scheduler_shard_count(),
        )?
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
        .set_default("observability.logging.root.level", default_log_level())?
        .set_default("observability.logging.console.enabled", true)?
        .set_default("observability.logging.console.level", default_log_level())?
        .set_default("observability.logging.file.enabled", false)?
        .set_default("observability.logging.file.level", default_log_level())?
        .set_default("observability.logging.file.path", default_app_log_path())?
        .set_default("observability.logging.error-file.enabled", false)?
        .set_default("observability.logging.error-file.level", "error")?
        .set_default(
            "observability.logging.error-file.path",
            default_app_log_path(),
        )?
        .set_default("observability.logging.elk.enabled", false)?
        .set_default("observability.logging.elk.servers", default_elk_servers())?
        .set_default("observability.logging.elk.topic", default_elk_topic())?
        .set_default("observability.logging.elk.level", default_log_level())?
        .set_default("observability.logging.elk.sasl.enabled", false)?
        .set_default("observability.logging.elk.sasl.username", "")?
        .set_default("observability.logging.elk.sasl.password", "")?
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
        .set_default("alert_retry.backoff_seconds", 300)?
        .set_default("notification_delivery.enabled", true)?
        .set_default(
            "notification_delivery.public_console_base_url",
            Option::<String>::None,
        )?
        .set_default("notification_delivery.interval_seconds", 60)?
        .set_default("notification_delivery.batch_size", 50)?
        .set_default("notification_delivery.max_attempts", 3)?
        .set_default("notification_delivery.backoff_seconds", 300)?;

    if let Some(path) = path {
        let content = std::fs::read_to_string(path).map_err(|error| {
            config::ConfigError::Message(format!(
                "failed to read config file {}: {error}",
                path.display()
            ))
        })?;
        let expanded = expand_config_placeholders(&content);
        builder = builder.add_source(File::from_str(&expanded, FileFormat::Yaml));
    }

    let config = builder
        .add_source(Environment::with_prefix("TIKEO").separator("__"))
        .build()?;

    let mut config: TikeoConfig = config.try_deserialize()?;
    apply_legacy_logging_compat(&mut config.observability.logging);
    Ok(config)
}

fn apply_legacy_logging_compat(logging: &mut LoggingConfig) {
    if let Some(level) = logging
        .level
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        logging.root.level.clone_from(level);
        logging.console.level.clone_from(level);
        if logging.file.level == default_log_level() {
            logging.file.level.clone_from(level);
        }
    }
    if let Some(path) = logging
        .log_dir
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        logging.file.enabled = true;
        logging.file.path.clone_from(path);
    }
}

fn expand_config_placeholders(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            output.push_str(&rest[start..]);
            return output;
        };
        let expression = &after[..end];
        output.push_str(&resolve_config_placeholder(expression));
        rest = &after[end + 1..];
    }
    output.push_str(rest);
    output
}

fn resolve_config_placeholder(expression: &str) -> String {
    let (key, fallback) = expression
        .split_once(':')
        .map_or((expression, ""), |(key, fallback)| (key, fallback));
    std::env::var(key).unwrap_or_else(|_| fallback.to_owned())
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::{ClusterModeConfig, TikeoConfig, load_config};

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
        assert_eq!(config.cluster.scheduler_shard_map_version, 1);
        assert_eq!(config.cluster.scheduler_shard_count, 64);
        assert!(config.cluster.peers.is_empty());
    }

    #[test]
    fn file_config_overrides_scheduler_shard_policy() {
        let path =
            std::env::temp_dir().join(format!("tikeo-shard-policy-{}.yml", std::process::id()));
        std::fs::write(
            &path,
            "cluster:
  scheduler_shard_map_version: 7
  scheduler_shard_count: 128
",
        )
        .unwrap_or_else(|error| panic!("temp config should write: {error}"));
        let config = load_config(Some(&path))
            .unwrap_or_else(|error| panic!("temp config should load: {error}"));
        std::fs::remove_file(&path)
            .unwrap_or_else(|error| panic!("temp config should delete: {error}"));

        assert_eq!(config.cluster.scheduler_shard_map_version, 7);
        assert_eq!(config.cluster.scheduler_shard_count, 128);
    }

    #[test]
    fn structured_postgres_database_config_percent_encodes_special_password_chars() {
        let path = std::env::temp_dir().join(format!(
            "tikeo-structured-postgres-{}.yml",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"storage:
  database:
    type: postgres
    host: postgres
    port: 5432
    username: tikeo
    password: "p@ss/word:with#chars"
    database: tikeo
    params:
      sslmode: disable
"#,
        )
        .unwrap_or_else(|error| panic!("temp config should write: {error}"));
        let config = load_config(Some(&path))
            .unwrap_or_else(|error| panic!("temp config should load: {error}"));
        std::fs::remove_file(&path)
            .unwrap_or_else(|error| panic!("temp config should delete: {error}"));

        assert_eq!(
            config.storage.effective_connection_url(),
            "postgres://tikeo:p%40ss%2Fword%3Awith%23chars@postgres:5432/tikeo?sslmode=disable"
        );
    }

    #[test]
    fn structured_network_database_config_does_not_inherit_sqlite_params() {
        let path = std::env::temp_dir().join(format!(
            "tikeo-structured-network-no-sqlite-params-{}.yml",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"storage:
  database:
    type: mysql
    host: mysql
    port: 3306
    username: tikeo
    password: "p@ss/word:with#chars"
    database: tikeo
"#,
        )
        .unwrap_or_else(|error| panic!("temp config should write: {error}"));
        let config = load_config(Some(&path))
            .unwrap_or_else(|error| panic!("temp config should load: {error}"));
        std::fs::remove_file(&path)
            .unwrap_or_else(|error| panic!("temp config should delete: {error}"));

        assert_eq!(
            config.storage.effective_connection_url(),
            "mysql://tikeo:p%40ss%2Fword%3Awith%23chars@mysql:3306/tikeo"
        );
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

        assert_eq!(config.observability.logging.root.level, "info");
        assert!(config.observability.logging.console.enabled);
        assert_eq!(config.observability.logging.console.level, "info");
        assert!(!config.observability.logging.file.enabled);
        assert_eq!(config.observability.logging.file.path, "/logs");
        assert!(!config.observability.logging.error_file.enabled);
        assert_eq!(config.observability.logging.error_file.level, "error");
        assert_eq!(config.observability.logging.error_file.path, "/logs");
        assert!(!config.observability.logging.elk.enabled);
        assert_eq!(
            config.observability.logging.elk.servers,
            "203.83.233.63:8094,36.111.150.189:8094,106.63.7.44:8094"
        );
        assert_eq!(config.observability.logging.elk.topic, "ivs-dev");
        assert!(!config.observability.tracing.enabled);
        assert!(config.observability.tracing.otlp_endpoint.is_none());
        assert!(config.observability.tracing.headers.is_empty());
    }

    #[test]
    fn nested_logging_config_resolves_env_default_log_path_and_elk_defaults() {
        let path = std::env::temp_dir().join(format!(
            "tikeo-nested-logging-config-{}.yml",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"observability:
  logging:
    root:
      level: WARN
    console:
      enabled: false
      level: ERROR
    file:
      enabled: true
      level: INFO
      path: "${TIKEO_LOG_PATH:./var/tikeo/logs}"
    error-file:
      enabled: true
      level: ERROR
      path: "${TIKEO_LOG_PATH:./var/tikeo/logs}"
    elk:
      enabled: ${ELK_ENABLED:false}
      servers: "${ELK_SERVERS:203.83.233.63:8094,36.111.150.189:8094,106.63.7.44:8094}"
      topic: "${ELK_TOPIC:ivs-dev}"
      level: INFO
      sasl:
        enabled: ${ELK_SASL_ENABLED:false}
        username: "${ELK_USERNAME:}"
        password: "${ELK_PASSWORD:}"
"#,
        )
        .unwrap_or_else(|error| panic!("temp config should write: {error}"));
        let config = load_config(Some(&path))
            .unwrap_or_else(|error| panic!("temp config should load: {error}"));
        std::fs::remove_file(&path)
            .unwrap_or_else(|error| panic!("temp config should delete: {error}"));

        assert_eq!(config.observability.logging.root.level, "WARN");
        assert!(!config.observability.logging.console.enabled);
        assert_eq!(config.observability.logging.console.level, "ERROR");
        assert!(config.observability.logging.file.enabled);
        assert_eq!(config.observability.logging.file.path, "./var/tikeo/logs");
        assert!(config.observability.logging.error_file.enabled);
        assert_eq!(
            config.observability.logging.error_file.path,
            "./var/tikeo/logs"
        );
        assert!(!config.observability.logging.elk.enabled);
        assert_eq!(config.observability.logging.elk.topic, "ivs-dev");
        assert_eq!(config.observability.logging.elk.sasl.username, "");
        assert_eq!(config.observability.logging.elk.sasl.password, "");
    }

    #[test]
    fn legacy_flat_logging_config_maps_to_nested_runtime_sinks() {
        let path = std::env::temp_dir().join(format!(
            "tikeo-legacy-logging-config-{}.yml",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"observability:
  logging:
    level: debug
    log_dir: "./legacy-logs"
"#,
        )
        .unwrap_or_else(|error| panic!("temp config should write: {error}"));
        let config = load_config(Some(&path))
            .unwrap_or_else(|error| panic!("temp config should load: {error}"));
        std::fs::remove_file(&path)
            .unwrap_or_else(|error| panic!("temp config should delete: {error}"));

        assert_eq!(config.observability.logging.level.as_deref(), Some("debug"));
        assert_eq!(
            config.observability.logging.log_dir.as_deref(),
            Some("./legacy-logs")
        );
        assert_eq!(config.observability.logging.root.level, "debug");
        assert_eq!(config.observability.logging.console.level, "debug");
        assert!(config.observability.logging.file.enabled);
        assert_eq!(config.observability.logging.file.level, "debug");
        assert_eq!(config.observability.logging.file.path, "./legacy-logs");
    }

    #[test]
    fn checked_in_configs_use_nested_logging_shape() {
        for name in ["dev.yml", "tikeo.yml"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../config")
                .join(name);
            let config = load_config(Some(&path))
                .unwrap_or_else(|error| panic!("{name} should load: {error}"));

            assert_eq!(config.observability.logging.root.level, "INFO");
            assert!(config.observability.logging.console.enabled);
            assert!(config.observability.logging.file.enabled);
            assert!(config.observability.logging.error_file.enabled);
            assert!(!config.observability.logging.elk.enabled);
            let expected_log_path = if name == "dev.yml" {
                ".dev/logs"
            } else {
                "/logs"
            };
            assert_eq!(config.observability.logging.file.path, expected_log_path);
            assert_eq!(
                config.observability.logging.error_file.path,
                expected_log_path
            );
        }
    }

    #[test]
    fn default_notification_delivery_config_enables_generic_delivery_worker() {
        let config =
            load_config(None).unwrap_or_else(|error| panic!("default config should load: {error}"));

        assert!(config.notification_delivery.enabled);
        assert!(
            config
                .notification_delivery
                .public_console_base_url
                .is_none()
        );
        assert_eq!(config.notification_delivery.interval_seconds, 60);
        assert_eq!(config.notification_delivery.batch_size, 50);
        assert_eq!(config.notification_delivery.max_attempts, 3);
        assert_eq!(config.notification_delivery.backoff_seconds, 300);
    }

    #[test]
    fn dev_config_sets_public_console_base_url_for_local_notification_cards() {
        let dev_config =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/dev.yml");
        let config = load_config(Some(&dev_config))
            .unwrap_or_else(|error| panic!("dev config should load: {error}"));

        assert_eq!(
            config
                .notification_delivery
                .public_console_base_url
                .as_deref(),
            Some("http://localhost:5173")
        );
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
        assert_eq!(config.alert_secrets.env_prefix, "TIKEO_ALERT_SECRET_");
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

        assert_eq!(loaded, TikeoConfig::default());
    }
}
