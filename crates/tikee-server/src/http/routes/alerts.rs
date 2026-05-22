#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use serde::Deserialize;

use crate::http::{
    AppState, auth,
    dto::{
        AlertDeliveryChannelStatus, AlertDeliveryStatusResponse, AlertEventSummary,
        AlertNotificationSummary, AlertRuleSummary, ApiResponse, CreateAlertRuleRequest,
    },
    error::ApiError,
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
    let mut issues = Vec::new();
    let channels: Vec<_> = channel_values
        .iter()
        .enumerate()
        .map(|(index, value)| channel_status(index, value))
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

fn channel_status(index: usize, value: &serde_json::Value) -> AlertDeliveryChannelStatus {
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
    let target_configured = channel_target_configured(&provider, value);
    let secret_configured = channel_secret_configured(value);
    let mut issues = Vec::new();
    if provider == "unknown" {
        issues.push(format!("channels[{index}].type is required"));
    }
    if enabled && !target_configured {
        issues.push(format!(
            "channels[{index}] target is required for {provider}"
        ));
    }
    AlertDeliveryChannelStatus {
        provider,
        target_configured,
        secret_configured,
        enabled,
        issues,
    }
}

fn channel_target_configured(provider: &str, value: &serde_json::Value) -> bool {
    let keys: &[&str] = match provider {
        "webhook" | "slack" | "dingtalk" | "feishu" | "wechat_work" | "pagerduty" => {
            &["url", "webhook_url", "routing_key", "integration_key"]
        }
        "email" => &["to", "recipients"],
        _ => &["target", "url", "to"],
    };
    keys.iter().any(|key| json_field_present(value, key))
}

fn channel_secret_configured(value: &serde_json::Value) -> bool {
    [
        "secret",
        "token",
        "authorization",
        "routing_key",
        "integration_key",
    ]
    .iter()
    .any(|key| json_field_present(value, key))
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
        .create_rule(tikee_storage::CreateAlertRule {
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
        .list_events(tikee_storage::AlertEventFilters {
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
        .list_event_summaries(tikee_storage::AlertEventFilters {
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
