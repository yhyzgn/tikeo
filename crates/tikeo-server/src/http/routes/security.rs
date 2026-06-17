#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{Json, extract::State, http::HeaderMap};
use tikeo_config::TlsEndpointConfig;
use tikeo_core::ScriptExecutionPolicy;

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, ClusterTransportPosture, NotificationSafetyPosture, ScriptGovernancePosture,
        SecurityPolicyDenial, SecurityPostureApiResponse, SecurityPostureCheck,
        SecurityPostureResponse, TlsEndpointStatus, TransportSecurityStatusApiResponse,
        TransportSecurityStatusResponse,
    },
    error::ApiError,
};

#[utoipa::path(get, path = "/api/v1/security/posture", tag = "security")]
pub async fn security_posture(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SecurityPostureApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "security", "read").await?;
    let mut checks = Vec::new();
    let transport = transport_status_from_state(&state);
    checks.push(SecurityPostureCheck {
        id: "transport.http".to_owned(),
        label: "HTTP TLS listener".to_owned(),
        status: if transport.http.listener_mode == "tls_config_error" {
            "warning"
        } else {
            "ok"
        }
        .to_owned(),
        source: "config".to_owned(),
        detail: format!(
            "HTTP endpoint listener mode is {}",
            transport.http.listener_mode
        ),
        evidence_count: u64::from(transport.http.tls_enabled),
    });
    checks.push(SecurityPostureCheck {
        id: "transport.worker_tunnel".to_owned(),
        label: "Worker Tunnel TLS/mTLS".to_owned(),
        status: if transport.worker_tunnel.listener_mode == "tls_config_error" {
            "warning"
        } else {
            "ok"
        }
        .to_owned(),
        source: "config".to_owned(),
        detail: format!(
            "Worker Tunnel listener mode is {}",
            transport.worker_tunnel.listener_mode
        ),
        evidence_count: u64::from(transport.worker_tunnel.tls_enabled),
    });

    let scripts = state
        .scripts
        .list_scripts()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let mut safe_default_deny_scripts = 0_u64;
    let mut dangerous_policy_scripts = 0_u64;
    let mut released_scripts = 0_u64;
    let mut signed_releases = 0_u64;
    let mut releases_with_grants = 0_u64;
    for script in &scripts {
        let policy_result = serde_json::from_value::<ScriptExecutionPolicy>(script.policy.clone())
            .map_err(|error| {
                ApiError::bad_request(format!(
                    "invalid persisted script policy for {}: {error}",
                    script.id
                ))
            })?
            .validate_default_deny();
        if policy_result.is_ok() && !script.allow_network {
            safe_default_deny_scripts = safe_default_deny_scripts.saturating_add(1);
        } else {
            dangerous_policy_scripts = dangerous_policy_scripts.saturating_add(1);
        }
        if script.released_version_id.is_some() {
            released_scripts = released_scripts.saturating_add(1);
        }
        if script.release_signature.is_some() {
            signed_releases = signed_releases.saturating_add(1);
        }
        if script.release_grants.is_some() {
            releases_with_grants = releases_with_grants.saturating_add(1);
        }
    }
    let script_governance = ScriptGovernancePosture {
        total_scripts: scripts.len().try_into().unwrap_or(u64::MAX),
        safe_default_deny_scripts,
        dangerous_policy_scripts,
        released_scripts,
        signed_releases,
        releases_with_grants,
        release_signature_required: state
            .script_governance
            .release_signature_secret_ref
            .is_some(),
        release_signature_secret_configured: state
            .script_governance
            .release_signature_secret_ref
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
    };
    checks.push(SecurityPostureCheck {
        id: "script.default_deny".to_owned(),
        label: "Script default-deny policy".to_owned(),
        status: if dangerous_policy_scripts == 0 { "ok" } else { "critical" }.to_owned(),
        source: "script_policy_snapshot".to_owned(),
        detail: format!("{safe_default_deny_scripts} script(s) validate as default-deny; {dangerous_policy_scripts} dangerous policy snapshot(s) found"),
        evidence_count: script_governance.total_scripts,
    });
    checks.push(SecurityPostureCheck {
        id: "script.release_signature".to_owned(),
        label: "Script release signature gate".to_owned(),
        status: if script_governance.release_signature_required {
            "ok"
        } else {
            "warning"
        }
        .to_owned(),
        source: "config".to_owned(),
        detail: if script_governance.release_signature_required {
            "Release signature secret reference is configured".to_owned()
        } else {
            "Release signature gate is optional until a secret ref is configured".to_owned()
        },
        evidence_count: signed_releases,
    });

    let channels = state
        .notification_channels
        .list_channels(tikeo_storage::NotificationChannelFilters::default())
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let notification_safety = NotificationSafetyPosture {
        total_channels: channels.len().try_into().unwrap_or(u64::MAX),
        enabled_channels: channels
            .iter()
            .filter(|channel| channel.enabled)
            .count()
            .try_into()
            .unwrap_or(u64::MAX),
        configured_targets: channels
            .iter()
            .filter(|channel| channel.target_configured)
            .count()
            .try_into()
            .unwrap_or(u64::MAX),
        redacted_targets: channels
            .iter()
            .filter(|channel| {
                channel.target_redacted.contains("***redacted***")
                    || channel.target_redacted.contains("secret-ref")
            })
            .count()
            .try_into()
            .unwrap_or(u64::MAX),
        channels_with_safety_policy: channels
            .iter()
            .filter(|channel| {
                channel
                    .safety_policy_json
                    .as_ref()
                    .is_some_and(|value| !value.trim().is_empty())
            })
            .count()
            .try_into()
            .unwrap_or(u64::MAX),
        direct_secret_values_redacted: channels
            .iter()
            .filter(|channel| channel.secret_configured)
            .count()
            .try_into()
            .unwrap_or(u64::MAX),
    };
    checks.push(SecurityPostureCheck {
        id: "notification.redaction".to_owned(),
        label: "Notification target redaction".to_owned(),
        status: "ok".to_owned(),
        source: "notification_channel".to_owned(),
        detail: format!(
            "{} configured target(s), {} redacted target(s)",
            notification_safety.configured_targets, notification_safety.redacted_targets
        ),
        evidence_count: notification_safety.total_channels,
    });

    let cluster_transport = ClusterTransportPosture {
        raft_transport_token_configured: state.raft_transport_token.is_some(),
        worker_tunnel_tls_ready: transport.worker_tunnel.listener_mode == "tls"
            || transport.worker_tunnel.listener_mode == "mtls",
        http_tls_ready: transport.http.listener_mode == "tls"
            || transport.http.listener_mode == "mtls",
    };
    checks.push(SecurityPostureCheck {
        id: "cluster.raft_transport_token".to_owned(),
        label: "Raft transport token".to_owned(),
        status: if cluster_transport.raft_transport_token_configured {
            "ok"
        } else {
            "warning"
        }
        .to_owned(),
        source: "config".to_owned(),
        detail: if cluster_transport.raft_transport_token_configured {
            "Internal Raft transport token is configured".to_owned()
        } else {
            "Internal Raft transport token is not configured".to_owned()
        },
        evidence_count: u64::from(cluster_transport.raft_transport_token_configured),
    });

    let denials = state
        .audit
        .list(tikeo_storage::AuditLogFilters {
            actor: None,
            action: None,
            resource_type: Some("script".to_owned()),
            resource_id: None,
            failure_reason: None,
            limit: Some(20),
            offset: None,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .filter(|item| item.result == "failed" && item.failure_reason.is_some())
        .map(|item| SecurityPolicyDenial {
            id: item.id,
            resource_type: item.resource_type,
            resource_id: item.resource_id,
            action: item.action,
            failure_reason: item.failure_reason.unwrap_or_default(),
            detail: item.detail,
            created_at: item.created_at,
        })
        .collect::<Vec<_>>();

    let overall_status = if checks.iter().any(|check| check.status == "critical") {
        "critical"
    } else if checks.iter().any(|check| check.status == "warning") {
        "warning"
    } else {
        "ok"
    }
    .to_owned();

    Ok(Json(ApiResponse::success(SecurityPostureResponse {
        overall_status,
        checks,
        transport,
        script_governance,
        notification_safety,
        cluster_transport,
        recent_denials: denials,
    })))
}

#[utoipa::path(get, path = "/api/v1/security/transport", tag = "security")]
pub async fn transport_security_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<TransportSecurityStatusApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "system", "read").await?;
    Ok(Json(ApiResponse::success(transport_status_from_state(
        &state,
    ))))
}

fn transport_status_from_state(state: &AppState) -> TransportSecurityStatusResponse {
    let mut issues = Vec::new();
    let http = endpoint_status("http", &state.transport_security.http, &mut issues);
    let worker_tunnel = endpoint_status(
        "worker_tunnel",
        &state.transport_security.worker_tunnel,
        &mut issues,
    );
    TransportSecurityStatusResponse {
        http,
        worker_tunnel,
        ready: issues.is_empty(),
        issues,
    }
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
