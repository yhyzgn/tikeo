#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{Json, extract::State, http::HeaderMap};
use tikeo_config::TlsEndpointConfig;

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, TlsEndpointStatus, TransportSecurityStatusApiResponse,
        TransportSecurityStatusResponse,
    },
    error::ApiError,
};

#[utoipa::path(get, path = "/api/v1/security/transport", tag = "security")]
pub async fn transport_security_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<TransportSecurityStatusApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "system", "read").await?;
    let mut issues = Vec::new();
    let http = endpoint_status("http", &state.transport_security.http, &mut issues);
    let worker_tunnel = endpoint_status(
        "worker_tunnel",
        &state.transport_security.worker_tunnel,
        &mut issues,
    );
    Ok(Json(ApiResponse::success(
        TransportSecurityStatusResponse {
            http,
            worker_tunnel,
            ready: issues.is_empty(),
            issues,
        },
    )))
}

fn endpoint_status(
    name: &str,
    config: &TlsEndpointConfig,
    issues: &mut Vec<String>,
) -> TlsEndpointStatus {
    let cert_configured = is_present(config.cert_path.as_ref());
    let key_configured = is_present(config.key_path.as_ref());
    let ca_configured = is_present(config.client_ca_path.as_ref());
    if config.tls_enabled {
        if !cert_configured {
            issues.push(format!("{name}.cert_path is required when TLS is enabled"));
        } else if !is_readable_file(config.cert_path.as_ref()) {
            issues.push(format!("{name}.cert_path is not readable"));
        }
        if !key_configured {
            issues.push(format!("{name}.key_path is required when TLS is enabled"));
        } else if !is_readable_file(config.key_path.as_ref()) {
            issues.push(format!("{name}.key_path is not readable"));
        }
    }
    if config.mtls_required {
        if !config.tls_enabled {
            issues.push(format!(
                "{name}.tls_enabled is required when mTLS is required"
            ));
        }
        if !ca_configured {
            issues.push(format!(
                "{name}.client_ca_path is required when mTLS is required"
            ));
        } else if !is_readable_file(config.client_ca_path.as_ref()) {
            issues.push(format!("{name}.client_ca_path is not readable"));
        }
    }
    TlsEndpointStatus {
        tls_enabled: config.tls_enabled,
        mtls_required: config.mtls_required,
        cert_configured,
        key_configured,
        ca_configured,
        listener_mode: listener_mode(config, cert_configured, key_configured, ca_configured)
            .to_owned(),
    }
}

const fn listener_mode(
    config: &TlsEndpointConfig,
    cert_configured: bool,
    key_configured: bool,
    ca_configured: bool,
) -> &'static str {
    if !config.tls_enabled {
        return "plaintext";
    }
    if !cert_configured || !key_configured || (config.mtls_required && !ca_configured) {
        return "tls_config_error";
    }
    if config.mtls_required { "mtls" } else { "tls" }
}

fn is_present(value: Option<&String>) -> bool {
    value.is_some_and(|item| !item.trim().is_empty())
}

fn is_readable_file(value: Option<&String>) -> bool {
    value
        .map(String::as_str)
        .filter(|path| !path.trim().is_empty())
        .is_some_and(|path| std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file()))
}
