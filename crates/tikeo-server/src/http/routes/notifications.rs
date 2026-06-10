#![allow(missing_docs, clippy::missing_errors_doc)]

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    http::{
        AppState, auth,
        dto::{ApiResponse, EmptyData},
        error::ApiError,
    },
    notification::{NotificationDeliveryPolicy, process_due_notification_delivery_attempts},
};

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationChannelTypeSummary {
    pub r#type: String,
    pub label: String,
    pub category: String,
    pub target_kind: String,
    pub description: String,
    pub required_config_keys: Vec<String>,
    pub secret_config_keys: Vec<String>,
    pub supports_test_send: bool,
    pub plugin_provided: bool,
    pub template: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotificationChannelRequest {
    pub scope_type: String,
    pub namespace: Option<String>,
    pub app: Option<String>,
    pub worker_pool: Option<String>,
    pub name: String,
    pub provider: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub secret_refs: serde_json::Value,
    pub safety_policy: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[allow(clippy::option_option)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotificationChannelRequest {
    pub scope_type: Option<String>,
    pub namespace: Option<Option<String>>,
    pub app: Option<Option<String>>,
    pub worker_pool: Option<Option<String>>,
    pub name: Option<String>,
    pub provider: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<serde_json::Value>,
    pub secret_refs: Option<serde_json::Value>,
    pub safety_policy: Option<Option<serde_json::Value>>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct NotificationChannelQuery {
    pub scope_type: Option<String>,
    pub namespace: Option<String>,
    pub app: Option<String>,
    pub worker_pool: Option<String>,
    pub provider: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotificationPolicyRequest {
    pub owner_type: String,
    pub owner_id: Option<String>,
    pub name: String,
    pub event_family: String,
    #[serde(default)]
    pub event_filter: serde_json::Value,
    #[serde(default)]
    pub channel_refs: Vec<serde_json::Value>,
    pub template_ref: Option<String>,
    pub severity: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_dedupe_seconds")]
    pub dedupe_seconds: i64,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[allow(clippy::option_option)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotificationPolicyRequest {
    pub owner_type: Option<String>,
    pub owner_id: Option<Option<String>>,
    pub name: Option<String>,
    pub event_family: Option<String>,
    pub event_filter: Option<serde_json::Value>,
    pub channel_refs: Option<Vec<serde_json::Value>>,
    pub template_ref: Option<Option<String>>,
    pub severity: Option<String>,
    pub enabled: Option<bool>,
    pub dedupe_seconds: Option<i64>,
    pub throttle: Option<Option<serde_json::Value>>,
    pub quiet_hours: Option<Option<serde_json::Value>>,
    pub escalation: Option<Option<serde_json::Value>>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct NotificationPolicyQuery {
    pub owner_type: Option<String>,
    pub owner_id: Option<String>,
    pub event_family: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct NotificationMessageQuery {
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub policy_id: Option<String>,
    pub event_type: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct NotificationDeliveryAttemptQuery {
    pub message_id: Option<String>,
    pub policy_id: Option<String>,
    pub channel_id: Option<String>,
    pub provider: Option<String>,
    pub retry_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDeliveryQueueStatusResponse {
    pub total_attempts: u64,
    pub delivered: u64,
    pub retry_pending: u64,
    pub dead_letter: u64,
    pub retry_consumed: u64,
    pub failed: u64,
    pub recent_dead_letters: Vec<tikeo_storage::NotificationDeliveryAttemptSummary>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDeliveryRetryRequest {
    pub limit: Option<u64>,
    pub max_attempts: Option<i32>,
    pub backoff_seconds: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/notification-channel-types",
    tag = "notifications"
)]
pub async fn list_notification_channel_types(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<NotificationChannelTypeSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let mut items = builtin_channel_types();
    let plugin_types = state
        .plugins
        .list_plugins()
        .await
        .map_err(|error| ApiError::storage(&error))?
        .into_iter()
        .filter(|plugin| plugin.enabled)
        .flat_map(|plugin| plugin.alert_channel_types)
        .map(|item| NotificationChannelTypeSummary {
            r#type: item.r#type,
            label: item.label,
            category: "plugin".to_owned(),
            target_kind: item.target_kind,
            description: item
                .description
                .unwrap_or_else(|| "Plugin-provided notification channel".to_owned()),
            required_config_keys: vec!["url".to_owned()],
            secret_config_keys: Vec::new(),
            supports_test_send: false,
            plugin_provided: true,
            template: item.template,
        });
    items.extend(plugin_types);
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(
    get,
    path = "/api/v1/notification-channels",
    tag = "notifications",
    params(NotificationChannelQuery)
)]
pub async fn list_notification_channels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<NotificationChannelQuery>,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::NotificationChannelSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let items = state
        .notification_channels
        .list_channels(tikeo_storage::NotificationChannelFilters {
            scope_type: query.scope_type,
            namespace: query.namespace,
            app: query.app,
            worker_pool: query.worker_pool,
            provider: query.provider,
            enabled: query.enabled,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/notification-channels", tag = "notifications", request_body = CreateNotificationChannelRequest)]
pub async fn create_notification_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateNotificationChannelRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::NotificationChannelSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "notifications", "manage").await?;
    validate_channel_request(
        &state,
        &request.scope_type,
        &request.provider,
        &request.name,
        &request.config,
        &request.secret_refs,
    )
    .await?;
    let created = state
        .notification_channels
        .create_channel(tikeo_storage::CreateNotificationChannel {
            scope_type: request.scope_type,
            namespace: request.namespace,
            app: request.app,
            worker_pool: request.worker_pool,
            name: request.name,
            provider: request.provider,
            enabled: request.enabled,
            config_json: json_to_string(&request.config),
            secret_refs_json: json_to_string(&request.secret_refs),
            safety_policy_json: request.safety_policy.map(|value| json_to_string(&value)),
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    super::common::audit(
        &state,
        &principal.username,
        "create",
        "notification_channel",
        &created.id,
        Some(format!(
            "name={}, provider={}",
            created.name, created.provider
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(created)))
}

#[utoipa::path(
    get,
    path = "/api/v1/notification-channels/{id}",
    tag = "notifications"
)]
pub async fn get_notification_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<tikeo_storage::NotificationChannelSummary>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let item = state
        .notification_channels
        .get_channel(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("notification channel not found"))?;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(patch, path = "/api/v1/notification-channels/{id}", tag = "notifications", request_body = UpdateNotificationChannelRequest)]
pub async fn update_notification_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateNotificationChannelRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::NotificationChannelSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "notifications", "manage").await?;
    let existing = state
        .notification_channels
        .get_channel(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("notification channel not found"))?;
    let provider = request.provider.as_ref().unwrap_or(&existing.provider);
    let name = request.name.as_ref().unwrap_or(&existing.name);
    let existing_config =
        serde_json::from_str(&existing.config_json).unwrap_or(serde_json::Value::Null);
    let config = request.config.as_ref().unwrap_or(&existing_config);
    let existing_secret_refs =
        serde_json::from_str(&existing.secret_refs_json).unwrap_or(serde_json::Value::Null);
    let secret_refs = request
        .secret_refs
        .as_ref()
        .unwrap_or(&existing_secret_refs);
    let scope_type = request.scope_type.as_ref().unwrap_or(&existing.scope_type);
    validate_channel_request(&state, scope_type, provider, name, config, secret_refs).await?;
    let updated = state
        .notification_channels
        .update_channel(
            &id,
            tikeo_storage::UpdateNotificationChannel {
                scope_type: request.scope_type,
                namespace: request.namespace,
                app: request.app,
                worker_pool: request.worker_pool,
                name: request.name,
                provider: request.provider,
                enabled: request.enabled,
                config_json: request.config.map(|value| json_to_string(&value)),
                secret_refs_json: request.secret_refs.map(|value| json_to_string(&value)),
                safety_policy_json: request
                    .safety_policy
                    .map(|value| value.map(|inner| json_to_string(&inner))),
                updated_by: Some(Some(principal.username.clone())),
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("notification channel not found"))?;
    super::common::audit(
        &state,
        &principal.username,
        "update",
        "notification_channel",
        &updated.id,
        Some(format!(
            "name={}, provider={}",
            updated.name, updated.provider
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(updated)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/notification-channels/{id}",
    tag = "notifications"
)]
pub async fn delete_notification_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EmptyData>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "notifications", "manage").await?;
    let result = state
        .notification_channels
        .delete_channel(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if result.referenced_by_policies > 0 {
        return Err(ApiError::conflict(format!(
            "notification channel is referenced by {} notification policy/policies",
            result.referenced_by_policies
        )));
    }
    if !result.deleted {
        return Err(ApiError::not_found("notification channel not found"));
    }
    super::common::audit(
        &state,
        &principal.username,
        "delete",
        "notification_channel",
        &id,
        None,
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(EmptyData {})))
}

#[utoipa::path(post, path = "/api/v1/notification-policies", tag = "notifications", request_body = CreateNotificationPolicyRequest)]
pub async fn create_notification_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateNotificationPolicyRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::NotificationPolicySummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "notifications", "manage").await?;
    validate_policy_request(
        &request.owner_type,
        &request.name,
        &request.event_family,
        &request.channel_refs,
    )?;
    validate_policy_channel_refs(&state, &request.channel_refs).await?;
    let created = state
        .notification_policies
        .create_policy(tikeo_storage::CreateNotificationPolicy {
            owner_type: request.owner_type,
            owner_id: request.owner_id,
            name: request.name,
            event_family: request.event_family,
            event_filter_json: json_to_string(&request.event_filter),
            channel_refs_json: json_to_string(&request.channel_refs),
            template_ref: request.template_ref,
            severity: request.severity,
            enabled: request.enabled,
            dedupe_seconds: request.dedupe_seconds,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    super::common::audit(
        &state,
        &principal.username,
        "create",
        "notification_policy",
        &created.id,
        Some(format!(
            "name={}, family={}",
            created.name, created.event_family
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(created)))
}

#[utoipa::path(
    get,
    path = "/api/v1/notification-policies",
    tag = "notifications",
    params(NotificationPolicyQuery)
)]
pub async fn list_notification_policies(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<NotificationPolicyQuery>,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::NotificationPolicySummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let items = state
        .notification_policies
        .list_policies(tikeo_storage::NotificationPolicyFilters {
            owner_type: query.owner_type,
            owner_id: query.owner_id,
            event_family: query.event_family,
            enabled: query.enabled,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(
    get,
    path = "/api/v1/notification-policies/{id}",
    tag = "notifications"
)]
pub async fn get_notification_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<tikeo_storage::NotificationPolicySummary>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let item = state
        .notification_policies
        .get_policy(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("notification policy not found"))?;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(patch, path = "/api/v1/notification-policies/{id}", tag = "notifications", request_body = UpdateNotificationPolicyRequest)]
pub async fn update_notification_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateNotificationPolicyRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::NotificationPolicySummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "notifications", "manage").await?;
    let existing = state
        .notification_policies
        .get_policy(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("notification policy not found"))?;
    let existing_channel_refs =
        serde_json::from_str::<Vec<serde_json::Value>>(&existing.channel_refs_json)
            .unwrap_or_default();
    validate_policy_request(
        request.owner_type.as_ref().unwrap_or(&existing.owner_type),
        request.name.as_ref().unwrap_or(&existing.name),
        request
            .event_family
            .as_ref()
            .unwrap_or(&existing.event_family),
        request
            .channel_refs
            .as_deref()
            .unwrap_or(&existing_channel_refs),
    )?;
    validate_policy_channel_refs(
        &state,
        request
            .channel_refs
            .as_deref()
            .unwrap_or(&existing_channel_refs),
    )
    .await?;
    let updated = state
        .notification_policies
        .update_policy(
            &id,
            tikeo_storage::UpdateNotificationPolicy {
                owner_type: request.owner_type,
                owner_id: request.owner_id,
                name: request.name,
                event_family: request.event_family,
                event_filter_json: request.event_filter.map(|value| json_to_string(&value)),
                channel_refs_json: request.channel_refs.map(|value| json_to_string(&value)),
                template_ref: request.template_ref,
                severity: request.severity,
                enabled: request.enabled,
                dedupe_seconds: request.dedupe_seconds,
                throttle_json: request
                    .throttle
                    .map(|value| value.map(|inner| json_to_string(&inner))),
                quiet_hours_json: request
                    .quiet_hours
                    .map(|value| value.map(|inner| json_to_string(&inner))),
                escalation_json: request
                    .escalation
                    .map(|value| value.map(|inner| json_to_string(&inner))),
                updated_by: Some(Some(principal.username.clone())),
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("notification policy not found"))?;
    super::common::audit(
        &state,
        &principal.username,
        "update",
        "notification_policy",
        &updated.id,
        Some(format!(
            "name={}, family={}",
            updated.name, updated.event_family
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(updated)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/notification-policies/{id}",
    tag = "notifications"
)]
pub async fn delete_notification_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EmptyData>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "notifications", "manage").await?;
    let deleted = state
        .notification_policies
        .delete_policy(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !deleted {
        return Err(ApiError::not_found("notification policy not found"));
    }
    super::common::audit(
        &state,
        &principal.username,
        "delete",
        "notification_policy",
        &id,
        None,
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(EmptyData {})))
}

#[utoipa::path(
    post,
    path = "/api/v1/notification-policies/{id}:validate",
    tag = "notifications"
)]
pub async fn validate_notification_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(policy_action): Path<String>,
) -> Result<Json<ApiResponse<tikeo_storage::NotificationPolicyValidationSummary>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let id = policy_action
        .strip_suffix(":validate")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "unsupported notification policy action: {policy_action}"
            ))
        })?;
    let result = state
        .notification_policies
        .validate_policy(id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("notification policy not found"))?;
    Ok(Json(ApiResponse::success(result)))
}

#[utoipa::path(
    get,
    path = "/api/v1/notification-messages",
    tag = "notifications",
    params(NotificationMessageQuery)
)]
pub async fn list_notification_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<NotificationMessageQuery>,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::NotificationMessageSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let items = state
        .notification_messages
        .list_messages(tikeo_storage::NotificationMessageFilters {
            source_type: query.source_type,
            source_id: query.source_id,
            policy_id: query.policy_id,
            event_type: query.event_type,
            severity: query.severity,
            status: query.status,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(
    get,
    path = "/api/v1/notification-delivery-attempts",
    tag = "notifications",
    params(NotificationDeliveryAttemptQuery)
)]
pub async fn list_notification_delivery_attempts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<NotificationDeliveryAttemptQuery>,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::NotificationDeliveryAttemptSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let items = state
        .notification_delivery_attempts
        .list_attempts(tikeo_storage::NotificationDeliveryAttemptFilters {
            message_id: query.message_id,
            policy_id: query.policy_id,
            channel_id: query.channel_id,
            provider: query.provider,
            retry_state: query.retry_state,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(
    get,
    path = "/api/v1/notification-delivery-attempts:queue-status",
    tag = "notifications"
)]
pub async fn notification_delivery_queue_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<NotificationDeliveryQueueStatusResponse>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let attempts = state
        .notification_delivery_attempts
        .list_attempts(tikeo_storage::NotificationDeliveryAttemptFilters::default())
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let mut response = NotificationDeliveryQueueStatusResponse {
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
    path = "/api/v1/notification-delivery-attempts:retry-due",
    tag = "notifications",
    request_body = NotificationDeliveryRetryRequest
)]
pub async fn retry_due_notification_delivery_attempts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<NotificationDeliveryRetryRequest>,
) -> Result<Json<ApiResponse<crate::notification::NotificationDeliveryProcessSummary>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "test").await?;
    let summary = process_due_notification_delivery_attempts(
        &state.notification_channels,
        &state.notification_messages,
        &state.notification_delivery_attempts,
        request.limit.unwrap_or(50).min(500),
        NotificationDeliveryPolicy {
            max_attempts: request.max_attempts.unwrap_or(3).clamp(1, 20),
            backoff_seconds: request.backoff_seconds.unwrap_or(300).clamp(1, 86_400),
        },
    )
    .await
    .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(summary)))
}

fn builtin_channel_types() -> Vec<NotificationChannelTypeSummary> {
    [
        (
            "webhook",
            "Generic Webhook",
            "webhook",
            "HTTP webhook",
            vec!["url"],
            vec!["authorization"],
        ),
        (
            "slack",
            "Slack Incoming Webhook",
            "office_bot",
            "Slack robot webhook",
            vec!["url"],
            vec![],
        ),
        (
            "dingtalk",
            "DingTalk Robot",
            "office_bot",
            "DingTalk robot webhook",
            vec!["url"],
            vec!["signingKey"],
        ),
        (
            "feishu",
            "Feishu/Lark Bot",
            "office_bot",
            "Feishu/Lark bot webhook",
            vec!["url"],
            vec!["signingKey"],
        ),
        (
            "wechat_work",
            "WeCom Bot",
            "office_bot",
            "WeChat Work/WeCom robot webhook",
            vec!["url"],
            vec![],
        ),
        (
            "pagerduty",
            "PagerDuty Events API",
            "incident",
            "PagerDuty Events v2 integration",
            vec!["routingKey"],
            vec!["routingKey"],
        ),
        (
            "email",
            "SMTP Email",
            "email",
            "SMTP email delivery",
            vec!["smtpUrl", "to"],
            vec![
                "password",
                "passwordSecretRef",
                "smtpUrl",
                "smtpUrlSecretRef",
            ],
        ),
    ]
    .into_iter()
    .map(|(r#type, label, category, description, required, secret)| {
        NotificationChannelTypeSummary {
            r#type: r#type.to_owned(),
            label: label.to_owned(),
            category: category.to_owned(),
            target_kind: if r#type == "email" {
                "email"
            } else {
                "webhook"
            }
            .to_owned(),
            description: description.to_owned(),
            required_config_keys: required.into_iter().map(str::to_owned).collect(),
            secret_config_keys: secret.into_iter().map(str::to_owned).collect(),
            supports_test_send: false,
            plugin_provided: false,
            template: serde_json::json!({}),
        }
    })
    .collect()
}

async fn validate_channel_request(
    state: &AppState,
    scope_type: &str,
    provider: &str,
    name: &str,
    config: &serde_json::Value,
    secret_refs: &serde_json::Value,
) -> Result<(), ApiError> {
    if !matches!(scope_type, "global" | "namespace" | "app" | "worker_pool") {
        return Err(ApiError::bad_request(
            "scopeType must be global, namespace, app, or worker_pool",
        ));
    }
    if name.trim().is_empty() {
        return Err(ApiError::bad_request(
            "notification channel name is required",
        ));
    }
    if !valid_slug(provider) {
        return Err(ApiError::bad_request("provider must be a lowercase slug"));
    }
    let provider_supported = builtin_channel_types()
        .iter()
        .any(|item| item.r#type == provider)
        || state
            .plugins
            .resolve_alert_channel_type(provider)
            .await
            .map_err(|error| ApiError::storage(&error))?
            .is_some();
    if !provider_supported {
        return Err(ApiError::bad_request(format!(
            "notification provider is not registered: {provider}"
        )));
    }
    if provider == "email" {
        if !json_field_present(config, "to") && !json_field_present(config, "recipients") {
            return Err(ApiError::bad_request(
                "email channel requires to or recipients",
            ));
        }
        if !json_field_present_any(config, &["smtpUrl", "smtp_url", "url"])
            && !json_field_present_any(secret_refs, &["smtpUrl", "smtp_url", "url"])
            && !json_field_present_any(config, &["smtpUrlSecretRef", "smtp_url_secret_ref"])
            && !json_field_present_any(secret_refs, &["smtpUrlSecretRef", "smtp_url_secret_ref"])
        {
            return Err(ApiError::bad_request(
                "email channel requires smtpUrl or smtpUrlSecretRef",
            ));
        }
        return Ok(());
    }
    if matches!(provider, "pagerduty") {
        if !json_field_present_any(
            config,
            &[
                "routingKey",
                "routing_key",
                "integrationKey",
                "integration_key",
            ],
        ) && !json_field_present_any(
            secret_refs,
            &[
                "routingKey",
                "routing_key",
                "integrationKey",
                "integration_key",
            ],
        ) {
            return Err(ApiError::bad_request(
                "pagerduty channel requires routingKey or integrationKey",
            ));
        }
        return Ok(());
    }
    if !json_field_present_any(config, &["url", "webhookUrl", "webhook_url"])
        && !json_field_present_any(secret_refs, &["url", "webhookUrl", "webhook_url"])
    {
        return Err(ApiError::bad_request("webhook-style channel requires url"));
    }
    Ok(())
}

fn validate_policy_request(
    owner_type: &str,
    name: &str,
    event_family: &str,
    channel_refs: &[serde_json::Value],
) -> Result<(), ApiError> {
    if !matches!(
        owner_type,
        "global"
            | "namespace"
            | "app"
            | "job"
            | "workflow"
            | "workflow_node"
            | "alert_rule"
            | "worker_pool"
    ) {
        return Err(ApiError::bad_request("ownerType is not supported"));
    }
    if name.trim().is_empty() {
        return Err(ApiError::bad_request(
            "notification policy name is required",
        ));
    }
    if !matches!(
        event_family,
        "job_instance" | "workflow" | "alert" | "worker" | "script_governance"
    ) {
        return Err(ApiError::bad_request("eventFamily is not supported"));
    }
    if channel_refs.is_empty() {
        return Err(ApiError::bad_request(
            "notification policy requires channelRefs",
        ));
    }
    if extract_channel_ref_ids(channel_refs).is_empty() {
        return Err(ApiError::bad_request(
            "notification policy channelRefs must include channelId values",
        ));
    }
    Ok(())
}

async fn validate_policy_channel_refs(
    state: &AppState,
    channel_refs: &[serde_json::Value],
) -> Result<(), ApiError> {
    let channel_ids = extract_channel_ref_ids(channel_refs);
    let channels = state
        .notification_channels
        .list_channels(tikeo_storage::NotificationChannelFilters::default())
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let mut missing = Vec::new();
    let mut disabled = Vec::new();
    for channel_id in channel_ids {
        match channels.iter().find(|channel| channel.id == channel_id) {
            Some(channel) if !channel.enabled => disabled.push(channel_id),
            Some(_) => {}
            None => missing.push(channel_id),
        }
    }
    if !missing.is_empty() {
        return Err(ApiError::bad_request(format!(
            "notification policy channel does not exist: {}",
            missing.join(", ")
        )));
    }
    if !disabled.is_empty() {
        return Err(ApiError::bad_request(format!(
            "notification policy channel is disabled: {}",
            disabled.join(", ")
        )));
    }
    Ok(())
}

fn extract_channel_ref_ids(channel_refs: &[serde_json::Value]) -> Vec<String> {
    channel_refs
        .iter()
        .filter_map(|item| {
            item.as_str().map(ToOwned::to_owned).or_else(|| {
                item.get("channelId")
                    .or_else(|| item.get("channel_id"))
                    .or_else(|| item.get("id"))
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned)
            })
        })
        .filter(|id| !id.trim().is_empty())
        .collect()
}

fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

fn json_field_present(value: &serde_json::Value, key: &str) -> bool {
    value.get(key).is_some_and(|field| match field {
        serde_json::Value::String(item) => !item.trim().is_empty(),
        serde_json::Value::Array(items) => !items.is_empty(),
        serde_json::Value::Null => false,
        _ => true,
    })
}

fn json_field_present_any(value: &serde_json::Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| json_field_present(value, key))
}

fn json_to_string<T: serde::Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_owned())
}

const fn default_enabled() -> bool {
    true
}

const fn default_dedupe_seconds() -> i64 {
    300
}
