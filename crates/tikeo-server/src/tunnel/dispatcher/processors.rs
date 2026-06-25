use std::{
    net::IpAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use sha2::{Digest, Sha256};
use tokio::time;

#[derive(Debug, Clone)]
pub(super) struct SqlProcessorOutcome {
    pub(super) success: bool,
    pub(super) message: String,
}

/// Execute sql processor.
pub(super) async fn execute_sql_processor(config: &serde_json::Value) -> SqlProcessorOutcome {
    let Some(connection_url) = config
        .get("databaseUrl")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return SqlProcessorOutcome {
            success: false,
            message: "sql node requires config.databaseUrl".to_owned(),
        };
    };
    let Some(sql) = config
        .get("sql")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return SqlProcessorOutcome {
            success: false,
            message: "sql node requires config.sql".to_owned(),
        };
    };
    let allowed = string_array(config.get("allowedDatabaseUrls"));
    if allowed.is_empty() || !allowed.iter().any(|candidate| candidate == connection_url) {
        return SqlProcessorOutcome {
            success: false,
            message: "sql databaseUrl is not in allowedDatabaseUrls".to_owned(),
        };
    }
    let read_only = config
        .get("readOnly")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let dry_run = config
        .get("dryRun")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    if read_only && !is_read_only_sql(sql) {
        return SqlProcessorOutcome {
            success: false,
            message: "sql readOnly mode only allows SELECT/EXPLAIN/WITH statements".to_owned(),
        };
    }
    if dry_run {
        return SqlProcessorOutcome {
            success: true,
            message: "sql dry-run validated statement and datasource allowlist".to_owned(),
        };
    }
    if !connection_url.starts_with("sqlite:") {
        return SqlProcessorOutcome {
            success: false,
            message: "sql executor currently supports sqlite databaseUrl for direct execution"
                .to_owned(),
        };
    }
    let pool = match sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(connection_url)
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            return SqlProcessorOutcome {
                success: false,
                message: format!("sql connect failed: {error}"),
            };
        }
    };
    let result = if read_only {
        sqlx::query(sql)
            .fetch_all(&pool)
            .await
            .map(|rows| rows.len().to_string())
    } else {
        sqlx::query(sql)
            .execute(&pool)
            .await
            .map(|result| result.rows_affected().to_string())
    };
    match result {
        Ok(count) if read_only => SqlProcessorOutcome {
            success: true,
            message: format!("sql query returned {count} row(s)"),
        },
        Ok(count) => SqlProcessorOutcome {
            success: true,
            message: format!("sql statement affected {count} row(s)"),
        },
        Err(error) => SqlProcessorOutcome {
            success: false,
            message: format!("sql execution failed: {error}"),
        },
    }
}

fn is_read_only_sql(sql: &str) -> bool {
    let normalized = sql
        .trim_start_matches(|ch: char| ch.is_whitespace() || ch == ';')
        .to_ascii_lowercase();
    normalized.starts_with("select ")
        || normalized.starts_with("select\n")
        || normalized == "select"
        || normalized.starts_with("with ")
        || normalized.starts_with("explain ")
}

#[derive(Debug, Clone)]
pub(super) struct GrpcProcessorOutcome {
    pub(super) success: bool,
    pub(super) message: String,
}

/// Execute grpc processor.
pub(super) async fn execute_grpc_processor(config: &serde_json::Value) -> GrpcProcessorOutcome {
    let Some(endpoint) = config
        .get("endpoint")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return GrpcProcessorOutcome {
            success: false,
            message: "grpc node requires config.endpoint".to_owned(),
        };
    };
    let Some(service) = config
        .get("service")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return GrpcProcessorOutcome {
            success: false,
            message: "grpc node requires config.service".to_owned(),
        };
    };
    let Some(method) = config
        .get("method")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return GrpcProcessorOutcome {
            success: false,
            message: "grpc node requires config.method".to_owned(),
        };
    };
    let allowed_hosts = string_array(config.get("allowedHosts"));
    let url = match url::Url::parse(endpoint) {
        Ok(url) => url,
        Err(error) => {
            return GrpcProcessorOutcome {
                success: false,
                message: format!("invalid grpc endpoint: {error}"),
            };
        }
    };
    if url.scheme() != "http" && url.scheme() != "https" {
        return GrpcProcessorOutcome {
            success: false,
            message: "grpc endpoint only allows http/https schemes".to_owned(),
        };
    }
    let Some(host) = url.host_str() else {
        return GrpcProcessorOutcome {
            success: false,
            message: "grpc endpoint must include host".to_owned(),
        };
    };
    if is_private_or_loopback_host(host)
        && !config
            .get("allowPrivateHost")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    {
        return GrpcProcessorOutcome {
            success: false,
            message: "grpc node rejects loopback/private IP hosts by default".to_owned(),
        };
    }
    if !allowed_hosts.is_empty()
        && !allowed_hosts
            .iter()
            .any(|allowed| host_matches(host, allowed))
    {
        return GrpcProcessorOutcome {
            success: false,
            message: format!("grpc host {host} is not in allowedHosts"),
        };
    }
    let path = format!(
        "/{}/{}",
        service.trim_matches('/'),
        method.trim_matches('/')
    );
    let uri = match tonic::codegen::http::uri::PathAndQuery::from_maybe_shared(path.clone()) {
        Ok(uri) => uri,
        Err(error) => {
            return GrpcProcessorOutcome {
                success: false,
                message: format!("invalid grpc method path {path}: {error}"),
            };
        }
    };
    let payload = config
        .get("payload")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let type_url = payload
        .get("typeUrl")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("type.googleapis.com/tikeo.workflow.v1.JsonPayload")
        .to_owned();
    let value = payload
        .get("valueBase64")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value).ok()
        })
        .unwrap_or_else(|| {
            payload.get("json").map_or_else(Vec::new, |json| {
                serde_json::to_vec(json).unwrap_or_default()
            })
        });
    let any = prost_types::Any { type_url, value };
    let channel = match tonic::transport::Endpoint::from_shared(endpoint.to_owned()) {
        Ok(endpoint) => match endpoint.timeout(Duration::from_secs(15)).connect().await {
            Ok(channel) => channel,
            Err(error) => {
                return GrpcProcessorOutcome {
                    success: false,
                    message: format!("grpc connect failed: {error}"),
                };
            }
        },
        Err(error) => {
            return GrpcProcessorOutcome {
                success: false,
                message: format!("invalid grpc endpoint: {error}"),
            };
        }
    };
    let mut client = tonic::client::Grpc::new(channel);
    let mut request = tonic::Request::new(any);
    if let Some(metadata) = config
        .get("metadata")
        .and_then(serde_json::Value::as_object)
    {
        for (key, value) in metadata {
            if let Some(value) = value.as_str()
                && let Ok(name) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes())
                && let Ok(parsed) = value.parse()
            {
                request.metadata_mut().insert(name, parsed);
            }
        }
    }
    match client
        .unary(
            request,
            uri,
            tonic_prost::ProstCodec::<prost_types::Any, prost_types::Any>::default(),
        )
        .await
    {
        Ok(_) => GrpcProcessorOutcome {
            success: true,
            message: format!("grpc {service}/{method} succeeded"),
        },
        Err(status) => GrpcProcessorOutcome {
            success: false,
            message: format!("grpc {service}/{method} failed: {}", status.message()),
        },
    }
}

#[derive(Debug, Clone)]
pub(super) struct FileCleanupOutcome {
    pub(super) success: bool,
    pub(super) message: String,
}

/// Execute file cleanup processor.
pub(super) async fn execute_file_cleanup_processor(
    config: &serde_json::Value,
) -> FileCleanupOutcome {
    let dry_run = config
        .get("dryRun")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let allowed_roots = string_array(config.get("allowedRoots"))
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if allowed_roots.is_empty() {
        return FileCleanupOutcome {
            success: false,
            message: "file_cleanup requires non-empty config.allowedRoots".to_owned(),
        };
    }
    let mut paths = string_array(config.get("paths"));
    if let Some(path) = config.get("path").and_then(serde_json::Value::as_str) {
        paths.push(path.to_owned());
    }
    if paths.is_empty() {
        return FileCleanupOutcome {
            success: false,
            message: "file_cleanup requires config.paths".to_owned(),
        };
    }
    let recursive = config
        .get("recursive")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let mut cleaned = 0_usize;
    let mut planned = 0_usize;
    for raw in paths {
        let path = PathBuf::from(raw.trim());
        if !path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return FileCleanupOutcome {
                success: false,
                message: format!(
                    "file_cleanup path must be clean absolute path: {}",
                    path.display()
                ),
            };
        }
        if !is_under_allowed_root(&path, &allowed_roots) {
            return FileCleanupOutcome {
                success: false,
                message: format!(
                    "file_cleanup path is outside allowedRoots: {}",
                    path.display()
                ),
            };
        }
        planned = planned.saturating_add(1);
        if dry_run {
            continue;
        }
        match tokio::fs::metadata(&path).await {
            Ok(metadata) if metadata.is_dir() && recursive => {
                if let Err(error) = tokio::fs::remove_dir_all(&path).await {
                    return FileCleanupOutcome {
                        success: false,
                        message: format!(
                            "file_cleanup failed to remove directory {}: {error}",
                            path.display()
                        ),
                    };
                }
                cleaned = cleaned.saturating_add(1);
            }
            Ok(metadata) if metadata.is_file() => {
                if let Err(error) = tokio::fs::remove_file(&path).await {
                    return FileCleanupOutcome {
                        success: false,
                        message: format!(
                            "file_cleanup failed to remove file {}: {error}",
                            path.display()
                        ),
                    };
                }
                cleaned = cleaned.saturating_add(1);
            }
            Ok(metadata) if metadata.is_dir() => {
                return FileCleanupOutcome {
                    success: false,
                    message: format!(
                        "file_cleanup refusing directory without recursive=true: {}",
                        path.display()
                    ),
                };
            }
            Ok(_) => {
                return FileCleanupOutcome {
                    success: false,
                    message: format!(
                        "file_cleanup only supports regular files/directories: {}",
                        path.display()
                    ),
                };
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return FileCleanupOutcome {
                    success: false,
                    message: format!("file_cleanup cannot inspect {}: {error}", path.display()),
                };
            }
        }
    }
    FileCleanupOutcome {
        success: true,
        message: if dry_run {
            format!("file_cleanup dry-run planned {planned} path(s)")
        } else {
            format!("file_cleanup removed {cleaned} of {planned} path(s)")
        },
    }
}

fn is_under_allowed_root(path: &Path, allowed_roots: &[PathBuf]) -> bool {
    allowed_roots.iter().any(|root| path.starts_with(root))
}

#[derive(Debug, Clone)]
pub(super) struct HttpProcessorOutcome {
    pub(super) success: bool,
    pub(super) message: String,
}

/// Execute http processor.
pub(super) async fn execute_http_processor(config: &serde_json::Value) -> HttpProcessorOutcome {
    let Some(url) = config
        .get("url")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return HttpProcessorOutcome {
            success: false,
            message: "http node requires config.url".to_owned(),
        };
    };
    let method = config
        .get("method")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("GET")
        .to_ascii_uppercase();
    let allowed_hosts = string_array(config.get("allowedHosts"));
    let denied_hosts = string_array(config.get("deniedHosts"));
    let denied_cidrs = string_array(config.get("deniedCidrs"));
    let parsed = match url::Url::parse(url) {
        Ok(parsed) => parsed,
        Err(error) => {
            return HttpProcessorOutcome {
                success: false,
                message: format!("invalid http url: {error}"),
            };
        }
    };
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return HttpProcessorOutcome {
            success: false,
            message: "http node only allows http/https urls".to_owned(),
        };
    }
    let Some(host) = parsed.host_str() else {
        return HttpProcessorOutcome {
            success: false,
            message: "http node url must include host".to_owned(),
        };
    };
    let allow_insecure_loopback = config
        .get("allowInsecureLoopback")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if host_is_denied(host, &denied_hosts, &denied_cidrs) {
        return HttpProcessorOutcome {
            success: false,
            message: format!("http host {host} is denied by deniedHosts/deniedCidrs"),
        };
    }
    if is_private_or_loopback_host(host) && !allow_insecure_loopback {
        return HttpProcessorOutcome {
            success: false,
            message: "http node rejects loopback/private IP hosts by default".to_owned(),
        };
    }
    if !allowed_hosts.is_empty()
        && !allowed_hosts
            .iter()
            .any(|allowed| host_matches(host, allowed))
    {
        return HttpProcessorOutcome {
            success: false,
            message: format!("http host {host} is not in allowedHosts"),
        };
    }
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            return HttpProcessorOutcome {
                success: false,
                message: format!("http client build failed: {error}"),
            };
        }
    };
    let req_method = method.parse().unwrap_or(reqwest::Method::GET);
    let retries = config
        .get("maxRetries")
        .or_else(|| config.get("retries"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
        .min(5);
    let retry_backoff_ms = config
        .get("retryBackoffMs")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(100)
        .min(5_000);
    let body = config.get("body").cloned();
    let signature = http_signature_header(config, body.as_ref());
    let failure_threshold = http_circuit_failure_threshold(config);
    let mut consecutive_failures = 0_u64;
    let mut last_message = String::new();
    for attempt in 0..=retries {
        let mut request = client.request(req_method.clone(), parsed.clone());
        if let Some(body) = body.as_ref() {
            request = request.json(body);
        }
        if let Some((header, value)) = signature.as_ref() {
            request = request.header(header, value);
        }
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() || attempt == retries {
                    return HttpProcessorOutcome {
                        success: status.is_success(),
                        message: format!(
                            "http {} {url} -> {} attempts={}",
                            method,
                            status.as_u16(),
                            attempt + 1
                        ),
                    };
                }
                last_message = format!("http {} {url} -> {}", method, status.as_u16());
                consecutive_failures += 1;
            }
            Err(error) => {
                last_message = format!("http request failed: {error}");
                consecutive_failures += 1;
                if attempt == retries {
                    return HttpProcessorOutcome {
                        success: false,
                        message: format!("{last_message} attempts={}", attempt + 1),
                    };
                }
            }
        }
        if failure_threshold > 0 && consecutive_failures >= failure_threshold {
            return HttpProcessorOutcome {
                success: false,
                message: format!(
                    "http circuit breaker open after {consecutive_failures} failures: {last_message}"
                ),
            };
        }
        time::sleep(Duration::from_millis(retry_backoff_ms)).await;
    }
    HttpProcessorOutcome {
        success: false,
        message: last_message,
    }
}

fn http_circuit_failure_threshold(config: &serde_json::Value) -> u64 {
    config
        .get("circuitBreaker")
        .and_then(|value| value.get("failureThreshold"))
        .or_else(|| config.get("circuitBreakerFailureThreshold"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
        .min(10)
}

fn host_is_denied(host: &str, denied_hosts: &[String], denied_cidrs: &[String]) -> bool {
    denied_hosts.iter().any(|denied| host_matches(host, denied))
        || host
            .parse::<IpAddr>()
            .is_ok_and(|ip| denied_cidrs.iter().any(|cidr| ip_in_cidr(ip, cidr)))
}

fn ip_in_cidr(ip: IpAddr, cidr: &str) -> bool {
    let Some((network, prefix)) = cidr.split_once('/') else {
        return ip.to_string() == cidr;
    };
    let Ok(prefix) = prefix.parse::<u32>() else {
        return false;
    };
    match (ip, network.parse::<IpAddr>()) {
        (IpAddr::V4(ip), Ok(IpAddr::V4(network))) if prefix <= 32 => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix)
            };
            (u32::from(ip) & mask) == (u32::from(network) & mask)
        }
        (IpAddr::V6(ip), Ok(IpAddr::V6(network))) if prefix <= 128 => {
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - prefix)
            };
            (u128::from(ip) & mask) == (u128::from(network) & mask)
        }
        _ => false,
    }
}

fn http_signature_header(
    config: &serde_json::Value,
    body: Option<&serde_json::Value>,
) -> Option<(String, String)> {
    let signature = config.get("signature")?;
    let algorithm = signature
        .get("type")
        .or_else(|| signature.get("algorithm"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("sha256");
    if algorithm != "sha256" {
        return None;
    }
    let secret = signature
        .get("secret")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let header = signature
        .get("header")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("X-Tikeo-Signature")
        .to_owned();
    let body = body.map(serde_json::Value::to_string).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(b"\n");
    hasher.update(body.as_bytes());
    Some((header, format!("sha256:{}", hex::encode(hasher.finalize()))))
}

fn string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn host_matches(host: &str, allowed: &str) -> bool {
    host.eq_ignore_ascii_case(allowed)
        || allowed
            .strip_prefix("*.")
            .is_some_and(|suffix| host.ends_with(suffix))
}

fn is_private_or_loopback_host(host: &str) -> bool {
    host.parse::<IpAddr>().is_ok_and(|ip| {
        ip.is_loopback()
            || ip.is_unspecified()
            || match ip {
                IpAddr::V4(v4) => v4.is_private() || v4.is_link_local(),
                IpAddr::V6(v6) => v6.is_unique_local() || v6.is_unicast_link_local(),
            }
    })
}
