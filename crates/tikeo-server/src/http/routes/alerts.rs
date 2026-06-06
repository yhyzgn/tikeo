#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use serde::Deserialize;

use crate::{
    alert::{AlertRetryPolicy, process_due_alert_delivery_retries},
    http::{
        AppState, auth,
        dto::{
            AlertDeliveryChannelStatus, AlertDeliveryQueueStatusResponse,
            AlertDeliveryStatusResponse, AlertEventSummary, AlertNotificationSummary,
            AlertRuleSummary, ApiResponse, CreateAlertRuleRequest,
        },
        error::ApiError,
    },
};

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AlertEventQuery {
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub failure_class: Option<String>,
    pub rule_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AlertDeliveryAttemptQuery {
    pub event_id: Option<String>,
    pub rule_id: Option<String>,
    pub provider: Option<String>,
    pub retry_state: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
pub struct AlertDeliveryRetryRequest {
    pub limit: Option<u64>,
    pub max_attempts: Option<i32>,
    pub backoff_seconds: Option<i64>,
}

#[utoipa::path(get, path = "/api/v1/alert-rules", tag = "alerts")]
pub async fn list_alert_rules(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<AlertRuleSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "audit", "read").await?;
    let items = state
        .alerts
        .list_rules()
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(|rule| AlertRuleSummary {
            id: rule.id,
            name: rule.name,
            severity: rule.severity,
            condition: serde_json::from_str(&rule.condition_json)
                .unwrap_or(serde_json::Value::Null),
            channels: serde_json::from_str(&rule.channels_json).unwrap_or_default(),
            enabled: rule.enabled,
            dedupe_seconds: u64::try_from(rule.dedupe_seconds).unwrap_or(0),
            silenced_until: rule.silenced_until,
            created_at: rule.created_at,
            updated_at: rule.updated_at,
        })
        .collect();
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(get, path = "/api/v1/alert-rules/{id}/delivery-status", tag = "alerts")]
pub async fn alert_rule_delivery_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<AlertDeliveryStatusResponse>>, ApiError> {
    auth::require_permission(&headers, &state, "audit", "read").await?;
    let rule = state
        .alerts
        .get_rule(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("alert rule not found"))?;
    let channel_values: Vec<serde_json::Value> =
        serde_json::from_str(&rule.channels_json).unwrap_or_default();
    let plugin_channel_types = state
        .plugins
        .list_plugins()
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .filter(|plugin| plugin.enabled)
        .flat_map(|plugin| plugin.alert_channel_types)
        .collect::<Vec<_>>();
    let mut issues = Vec::new();
    let channels: Vec<_> = channel_values
        .iter()
        .enumerate()
        .map(|(index, value)| channel_status(index, value, &plugin_channel_types))
        .inspect(|status| issues.extend(status.issues.iter().cloned()))
        .collect();
    Ok(Json(ApiResponse::success(AlertDeliveryStatusResponse {
        rule_id: rule.id,
        ready: issues.is_empty(),
        channel_count: u64::try_from(channels.len()).unwrap_or(u64::MAX),
        channels,
        issues,
    })))
}

fn channel_status(
    index: usize,
    value: &serde_json::Value,
    plugin_channel_types: &[tikeo_storage::PluginAlertChannelTypeSummary],
) -> AlertDeliveryChannelStatus {
    let provider = value
        .get("type")
        .or_else(|| value.get("provider"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    let enabled = value
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let provider_supported = builtin_provider(&provider)
        || plugin_channel_types
            .iter()
            .any(|channel| channel.r#type == provider);
    let target_configured = channel_target_configured(&provider, value);
    let secret_configured = channel_secret_configured(value);
    let transport_security = channel_transport_security(&provider, value);
    let target_redacted = channel_target_redacted(&provider, value);
    let mut issues = Vec::new();
    if provider == "unknown" {
        issues.push(format!("channels[{index}].type is required"));
    } else if !provider_supported {
        issues.push(format!(
            "channels[{index}].type is not registered: {provider}"
        ));
    }
    if enabled && !target_configured {
        issues.push(format!(
            "channels[{index}] target is required for {provider}"
        ));
    }
    if enabled && provider == "email" && transport_security.as_deref() == Some("plain") {
        issues.push(format!(
            "channels[{index}] smtp:// is only allowed for explicit local smoke tests"
        ));
    }
    AlertDeliveryChannelStatus {
        provider,
        target_configured,
        secret_configured,
        enabled,
        target_redacted,
        transport_security,
        issues,
    }
}

fn builtin_provider(provider: &str) -> bool {
    matches!(
        provider,
        "webhook" | "slack" | "dingtalk" | "feishu" | "wechat_work" | "pagerduty" | "email"
    )
}

fn channel_target_configured(provider: &str, value: &serde_json::Value) -> bool {
    if provider == "email" {
        return ["to", "recipients"]
            .iter()
            .any(|key| json_field_present(value, key))
            && ["smtp_url", "url", "smtp_url_secret_ref"]
                .iter()
                .any(|key| json_field_present(value, key));
    }
    let keys: &[&str] = match provider {
        "webhook" | "slack" | "dingtalk" | "feishu" | "wechat_work" | "pagerduty" => {
            &["url", "webhook_url", "routing_key", "integration_key"]
        }
        _ => &["target", "url", "to"],
    };
    keys.iter().any(|key| json_field_present(value, key))
}

fn channel_secret_configured(value: &serde_json::Value) -> bool {
    [
        "secret",
        "secret_ref",
        "token",
        "token_ref",
        "authorization",
        "authorization_ref",
        "routing_key",
        "routing_key_ref",
        "integration_key",
        "integration_key_ref",
        "smtp_url_secret_ref",
        "password_secret_ref",
    ]
    .iter()
    .any(|key| json_field_present(value, key))
}

fn channel_transport_security(provider: &str, value: &serde_json::Value) -> Option<String> {
    if provider == "email" {
        let url = ["smtp_url", "url"]
            .iter()
            .find_map(|key| value.get(key).and_then(serde_json::Value::as_str));
        if value.get("smtp_url_secret_ref").is_some() {
            return Some("secret_ref".to_owned());
        }
        return url.map(|url| {
            if url.starts_with("smtps://") {
                "tls".to_owned()
            } else if url.starts_with("smtp+starttls://") {
                "starttls".to_owned()
            } else {
                "plain".to_owned()
            }
        });
    }
    target_url(value).map(|url| {
        if url.starts_with("https://") {
            "https".to_owned()
        } else {
            "insecure".to_owned()
        }
    })
}

fn channel_target_redacted(provider: &str, value: &serde_json::Value) -> Option<String> {
    if provider == "email" {
        return ["to", "recipients"]
            .iter()
            .find_map(|key| value.get(key))
            .map(redact_email_target);
    }
    target_url(value).map(redact_url_like)
}

fn target_url(value: &serde_json::Value) -> Option<&str> {
    ["url", "webhook_url"]
        .iter()
        .find_map(|key| value.get(key).and_then(serde_json::Value::as_str))
}

fn redact_email_target(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(item) => item.to_owned(),
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>()
            .join(","),
        _ => "email".to_owned(),
    }
}

fn redact_url_like(value: &str) -> String {
    url::Url::parse(value).map_or_else(
        |_| "invalid-url".to_owned(),
        |url| {
            let mut redacted = format!(
                "{}://{}",
                url.scheme(),
                url.host_str().unwrap_or("unknown-host")
            );
            if let Some(port) = url.port() {
                redacted.push(':');
                redacted.push_str(&port.to_string());
            }
            if url.path() != "/" && !url.path().is_empty() {
                redacted.push_str("/...");
            }
            redacted
        },
    )
}

fn json_field_present(value: &serde_json::Value, key: &str) -> bool {
    value.get(key).is_some_and(|field| match field {
        serde_json::Value::String(item) => !item.trim().is_empty(),
        serde_json::Value::Array(items) => !items.is_empty(),
        serde_json::Value::Null => false,
        _ => true,
    })
}

#[utoipa::path(post, path = "/api/v1/alert-rules", tag = "alerts", request_body = CreateAlertRuleRequest)]
pub async fn create_alert_rule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateAlertRuleRequest>,
) -> Result<Json<ApiResponse<AlertRuleSummary>>, ApiError> {
    auth::require_permission(&headers, &state, "audit", "read").await?;
    let created = state
        .alerts
        .create_rule(tikeo_storage::CreateAlertRule {
            name: request.name,
            severity: request.severity,
            condition_json: request.condition.to_string(),
            channels_json: serde_json::to_string(&request.channels)
                .unwrap_or_else(|_| "[]".to_owned()),
            enabled: request.enabled,
            dedupe_seconds: i64::try_from(request.dedupe_seconds.unwrap_or(1)).unwrap_or(i64::MAX),
            silenced_until: request.silenced_until,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(AlertRuleSummary {
        id: created.id,
        name: created.name,
        severity: created.severity,
        condition: serde_json::from_str(&created.condition_json).unwrap_or(serde_json::Value::Null),
        channels: serde_json::from_str(&created.channels_json).unwrap_or_default(),
        enabled: created.enabled,
        dedupe_seconds: u64::try_from(created.dedupe_seconds).unwrap_or(0),
        silenced_until: created.silenced_until,
        created_at: created.created_at,
        updated_at: created.updated_at,
    })))
}

#[utoipa::path(
    get,
    path = "/api/v1/alert-delivery-attempts",
    tag = "alerts",
    params(AlertDeliveryAttemptQuery)
)]
pub async fn list_alert_delivery_attempts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<AlertDeliveryAttemptQuery>,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::AlertDeliveryAttemptSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "audit", "read").await?;
    let items = state
        .alerts
        .list_delivery_attempts(tikeo_storage::AlertDeliveryAttemptFilters {
            event_id: query.event_id,
            rule_id: query.rule_id,
            provider: query.provider,
            retry_state: query.retry_state,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(
    get,
    path = "/api/v1/alert-delivery-attempts:queue-status",
    tag = "alerts"
)]
pub async fn alert_delivery_queue_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<AlertDeliveryQueueStatusResponse>>, ApiError> {
    auth::require_permission(&headers, &state, "audit", "read").await?;
    let attempts = state
        .alerts
        .list_delivery_attempts(tikeo_storage::AlertDeliveryAttemptFilters::default())
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let mut response = AlertDeliveryQueueStatusResponse {
        total_attempts: attempts.len() as u64,
        delivered: 0,
        retry_pending: 0,
        dead_letter: 0,
        retry_consumed: 0,
        failed: 0,
        recent_dead_letters: Vec::new(),
    };
    for attempt in attempts {
        match attempt.retry_state.as_str() {
            "delivered" => response.delivered += 1,
            "retry_pending" => response.retry_pending += 1,
            "dead_letter" => {
                response.dead_letter += 1;
                if response.recent_dead_letters.len() < 20 {
                    response.recent_dead_letters.push(attempt);
                }
            }
            "retry_consumed" => response.retry_consumed += 1,
            _ => response.failed += 1,
        }
    }
    Ok(Json(ApiResponse::success(response)))
}

#[utoipa::path(
    post,
    path = "/api/v1/alert-delivery-attempts:retry-due",
    tag = "alerts",
    request_body = AlertDeliveryRetryRequest
)]
pub async fn retry_due_alert_delivery_attempts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<AlertDeliveryRetryRequest>,
) -> Result<Json<ApiResponse<crate::alert::AlertRetryProcessSummary>>, ApiError> {
    auth::require_permission(&headers, &state, "audit", "read").await?;
    let policy = AlertRetryPolicy {
        max_attempts: request.max_attempts.unwrap_or(3).clamp(1, 20),
        backoff_seconds: request.backoff_seconds.unwrap_or(300).clamp(1, 86_400),
    };
    let summary = process_due_alert_delivery_retries(
        &state.alerts,
        request.limit.unwrap_or(50).min(500),
        policy,
    )
    .await
    .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(summary)))
}

#[utoipa::path(
    get,
    path = "/api/v1/alert-events",
    tag = "alerts",
    params(AlertEventQuery)
)]
pub async fn list_alert_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<AlertEventQuery>,
) -> Result<Json<ApiResponse<Vec<AlertEventSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "audit", "read").await?;
    let items = state
        .alerts
        .list_events(tikeo_storage::AlertEventFilters {
            resource_type: query.resource_type,
            resource_id: query.resource_id,
            failure_class: query.failure_class,
            rule_id: query.rule_id,
            status: query.status,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(|event| AlertEventSummary {
            id: event.id,
            rule_id: event.rule_id,
            rule_name: event.rule_name,
            severity: event.severity,
            status: event.status,
            event_type: event.event_type,
            resource_type: event.resource_type,
            resource_id: event.resource_id,
            failure_class: event.failure_class,
            message: event.message,
            dedupe_key: event.dedupe_key,
            created_at: event.created_at,
        })
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(
    get,
    path = "/api/v1/alert-events:summary",
    tag = "alerts",
    params(AlertEventQuery)
)]
pub async fn list_alert_event_summaries(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<AlertEventQuery>,
) -> Result<Json<ApiResponse<Vec<AlertNotificationSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "audit", "read").await?;
    let items = state
        .alerts
        .list_event_summaries(tikeo_storage::AlertEventFilters {
            resource_type: query.resource_type,
            resource_id: query.resource_id,
            failure_class: query.failure_class,
            rule_id: query.rule_id,
            status: query.status,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .map(|summary| AlertNotificationSummary {
            rule_id: summary.rule_id,
            rule_name: summary.rule_name,
            severity: summary.severity,
            resource_type: summary.resource_type,
            resource_id: summary.resource_id,
            failure_class: summary.failure_class,
            latest_status: summary.latest_status,
            latest_event_type: summary.latest_event_type,
            latest_message: summary.latest_message,
            event_count: summary.event_count,
            firing_count: summary.firing_count,
            suppressed_count: summary.suppressed_count,
            silenced_count: summary.silenced_count,
            recovered_count: summary.recovered_count,
            first_seen: summary.first_seen,
            last_seen: summary.last_seen,
        })
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/alert-events/{id}/resolve", tag = "alerts")]
pub async fn resolve_alert_event(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<AlertEventSummary>>, ApiError> {
    auth::require_permission(&headers, &state, "audit", "read").await?;
    let resolved = state
        .alerts
        .record_script_governance_recovery(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("alert event not found"))?;
    Ok(Json(ApiResponse::success(AlertEventSummary {
        id: resolved.id,
        rule_id: resolved.rule_id,
        rule_name: resolved.rule_name,
        severity: resolved.severity,
        status: resolved.status,
        event_type: resolved.event_type,
        resource_type: resolved.resource_type,
        resource_id: resolved.resource_id,
        failure_class: resolved.failure_class,
        message: resolved.message,
        dedupe_key: resolved.dedupe_key,
        created_at: resolved.created_at,
    })))
}
