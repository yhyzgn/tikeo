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
    alert::AlertDeliveryPolicy,
    http::{
        AppState, auth,
        dto::{ApiResponse, EmptyData},
        error::ApiError,
    },
    notification::{
        NotificationDeliveryPolicy, deliver_notification_channel_once,
        process_due_notification_delivery_attempts,
    },
};

use super::notification_providers::{
    ChannelValidationInput, NotificationChannelTypeSummary, builtin_channel_types,
    is_builtin_provider, json_to_string, validate_channel_request,
};

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
pub struct TestNotificationChannelRequest {
    pub subject: Option<String>,
    pub body: Option<String>,
    pub event_type: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub severity: Option<String>,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestNotificationChannelResponse {
    pub channel_id: String,
    pub message_id: String,
    pub attempt_id: String,
    pub provider: String,
    pub target_redacted: String,
    pub delivered: bool,
    pub status_code: Option<u16>,
    pub retry_state: String,
    pub error: Option<String>,
    pub rendered_payload: Option<serde_json::Value>,
    pub created_at: String,
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
            required_target_keys: vec!["url".to_owned()],
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
        ChannelValidationInput {
            scope_type: &request.scope_type,
            namespace: request.namespace.as_deref(),
            app: request.app.as_deref(),
            worker_pool: request.worker_pool.as_deref(),
            provider: &request.provider,
            name: &request.name,
            config: &request.config,
            secret_refs: &request.secret_refs,
        },
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
    let namespace = request
        .namespace
        .as_ref()
        .map_or(existing.namespace.as_deref(), Option::as_deref);
    let app = request
        .app
        .as_ref()
        .map_or(existing.app.as_deref(), Option::as_deref);
    let worker_pool = request
        .worker_pool
        .as_ref()
        .map_or(existing.worker_pool.as_deref(), Option::as_deref);
    validate_channel_request(
        &state,
        ChannelValidationInput {
            scope_type,
            namespace,
            app,
            worker_pool,
            provider,
            name,
            config,
            secret_refs,
        },
    )
    .await?;
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

#[utoipa::path(
    post,
    path = "/api/v1/notification-channels/{id}/test-send",
    tag = "notifications",
    request_body = TestNotificationChannelRequest
)]
pub async fn test_notification_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<TestNotificationChannelRequest>,
) -> Result<Json<ApiResponse<TestNotificationChannelResponse>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "notifications", "test").await?;
    let channel = state
        .notification_channels
        .get_channel_delivery_config(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("notification channel not found"))?;
    if !channel.enabled {
        return Err(ApiError::bad_request("notification channel is disabled"));
    }
    if !is_builtin_provider(&channel.provider) {
        return Err(ApiError::bad_request(format!(
            "notification provider does not support test send: {}",
            channel.provider
        )));
    }

    let event_type =
        clean_string(request.event_type).unwrap_or_else(|| "notification.channel_test".to_owned());
    let resource_type =
        clean_string(request.resource_type).unwrap_or_else(|| "notification_channel".to_owned());
    let resource_id = clean_string(request.resource_id).unwrap_or_else(|| channel.id.clone());
    let severity = clean_string(request.severity).unwrap_or_else(|| "info".to_owned());
    let subject = clean_string(request.subject)
        .unwrap_or_else(|| format!("Tikeo notification channel test: {}", channel.provider));
    let body = clean_string(request.body).unwrap_or_else(|| {
        format!(
            "This is a test notification sent through channel {}.",
            channel.id
        )
    });
    let payload = test_notification_payload(
        &request.payload,
        &channel.id,
        &channel.provider,
        &event_type,
        &resource_type,
        &resource_id,
        &severity,
        &subject,
        &body,
        &principal.username,
    );
    let dedupe_key = format!(
        "notification-channel-test:{}:{}",
        channel.id,
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    );
    let message = state
        .notification_messages
        .create_message(tikeo_storage::CreateNotificationMessage {
            source_type: "channel_test".to_owned(),
            source_id: channel.id.clone(),
            policy_id: "notification-channel-test".to_owned(),
            event_type,
            resource_type,
            resource_id,
            severity,
            subject,
            body,
            payload_json: payload.to_string(),
            dedupe_key,
            trace_id: headers
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned),
            status: "pending".to_owned(),
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let delivery_result =
        deliver_notification_channel_once(&channel, &message, AlertDeliveryPolicy::production())
            .await;
    let retry_state = if delivery_result.delivered {
        "delivered"
    } else {
        "dead_letter"
    }
    .to_owned();
    let attempt = state
        .notification_delivery_attempts
        .record_attempt(tikeo_storage::RecordNotificationDeliveryAttempt {
            message_id: message.id.clone(),
            policy_id: message.policy_id.clone(),
            channel_id: channel.id.clone(),
            provider: delivery_result.provider.clone(),
            target_redacted: delivery_result.target_redacted.clone(),
            attempt: 1,
            delivered: delivery_result.delivered,
            status_code: delivery_result.status_code.map(i32::from),
            error: delivery_result.error.clone(),
            retry_state: retry_state.clone(),
            next_retry_at: None,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let _message = state
        .notification_messages
        .update_message_status(
            &message.id,
            if delivery_result.delivered {
                "delivered"
            } else {
                "dead_letter"
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?;
    super::common::audit(
        &state,
        &principal.username,
        "test",
        "notification_channel",
        &channel.id,
        Some(format!(
            "provider={}, delivered={}",
            delivery_result.provider, delivery_result.delivered
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(
        TestNotificationChannelResponse {
            channel_id: channel.id,
            message_id: message.id,
            attempt_id: attempt.id,
            provider: delivery_result.provider,
            target_redacted: delivery_result.target_redacted,
            delivered: delivery_result.delivered,
            status_code: delivery_result.status_code,
            retry_state,
            error: delivery_result.error,
            rendered_payload: delivery_result
                .rendered_payload
                .map(redact_notification_test_payload),
            created_at: attempt.created_at,
        },
    )))
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
    validate_policy_template_ref(
        &state,
        request.template_ref.as_deref(),
        &request.channel_refs,
    )
    .await?;
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
    let next_template_ref = request
        .template_ref
        .as_ref()
        .map_or(existing.template_ref.as_deref(), |value| value.as_deref());
    validate_policy_template_ref(
        &state,
        next_template_ref,
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
    Ok(Json(ApiResponse::success(
        append_template_validation_issues(&state, result).await?,
    )))
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

fn clean_string(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
}

#[allow(clippy::too_many_arguments)]
fn test_notification_payload(
    payload: &serde_json::Value,
    channel_id: &str,
    provider: &str,
    event_type: &str,
    resource_type: &str,
    resource_id: &str,
    severity: &str,
    subject: &str,
    body: &str,
    requested_by: &str,
) -> serde_json::Value {
    let mut value = payload
        .as_object()
        .cloned()
        .map_or_else(|| serde_json::json!({}), serde_json::Value::Object);
    value = redact_notification_test_payload(value);
    if let Some(map) = value.as_object_mut() {
        for (key, item) in [
            ("kind", serde_json::Value::String("channel_test".to_owned())),
            (
                "channelId",
                serde_json::Value::String(channel_id.to_owned()),
            ),
            ("provider", serde_json::Value::String(provider.to_owned())),
            (
                "eventType",
                serde_json::Value::String(event_type.to_owned()),
            ),
            (
                "resourceType",
                serde_json::Value::String(resource_type.to_owned()),
            ),
            (
                "resourceId",
                serde_json::Value::String(resource_id.to_owned()),
            ),
            ("severity", serde_json::Value::String(severity.to_owned())),
            ("subject", serde_json::Value::String(subject.to_owned())),
            ("body", serde_json::Value::String(body.to_owned())),
            (
                "requestedBy",
                serde_json::Value::String(requested_by.to_owned()),
            ),
        ] {
            map.insert(key.to_owned(), item);
        }
    }
    value
}

fn redact_notification_test_payload(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .into_iter()
                .map(redact_notification_test_payload)
                .collect(),
        ),
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if sensitive_notification_payload_key(&key) {
                        (key, serde_json::Value::String("***redacted***".to_owned()))
                    } else {
                        (key, redact_notification_test_payload(value))
                    }
                })
                .collect(),
        ),
        other => other,
    }
}

fn sensitive_notification_payload_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['_', '-'], "");
    normalized.contains("secret")
        || normalized.contains("token")
        || normalized.contains("password")
        || normalized.contains("authorization")
        || normalized == "sign"
        || normalized.contains("signature")
        || normalized == "routingkey"
        || normalized == "integrationkey"
        || normalized == "signingkey"
        || normalized == "smtpurl"
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

async fn validate_policy_template_ref(
    state: &AppState,
    template_ref: Option<&str>,
    channel_refs: &[serde_json::Value],
) -> Result<(), ApiError> {
    let Some(template_ref) = template_ref.filter(|value| !value.trim().is_empty()) else {
        return Ok(());
    };
    let template = state
        .notification_templates
        .get_template(template_ref)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| {
            ApiError::bad_request(format!(
                "notification policy template does not exist: {template_ref}"
            ))
        })?;
    if !template.enabled {
        return Err(ApiError::bad_request(format!(
            "notification policy template is disabled: {template_ref}"
        )));
    }
    let providers = policy_channel_providers(state, channel_refs).await?;
    let mismatched: Vec<_> = providers
        .into_iter()
        .filter(|provider| provider != &template.provider)
        .collect();
    if !mismatched.is_empty() {
        return Err(ApiError::bad_request(format!(
            "notification policy template provider {} does not match channel provider(s): {}",
            template.provider,
            mismatched.join(", ")
        )));
    }
    Ok(())
}

async fn policy_channel_providers(
    state: &AppState,
    channel_refs: &[serde_json::Value],
) -> Result<Vec<String>, ApiError> {
    let channel_ids = extract_channel_ref_ids(channel_refs);
    let channels = state
        .notification_channels
        .list_channels(tikeo_storage::NotificationChannelFilters::default())
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(channel_ids
        .into_iter()
        .filter_map(|channel_id| {
            channels
                .iter()
                .find(|channel| channel.id == channel_id)
                .map(|channel| channel.provider.clone())
        })
        .collect())
}

async fn append_template_validation_issues(
    state: &AppState,
    mut result: tikeo_storage::NotificationPolicyValidationSummary,
) -> Result<tikeo_storage::NotificationPolicyValidationSummary, ApiError> {
    let Some(policy) = state
        .notification_policies
        .get_policy(&result.policy_id)
        .await
        .map_err(|error| ApiError::storage(&error))?
    else {
        return Ok(result);
    };
    let channel_refs: Vec<serde_json::Value> =
        serde_json::from_str(&policy.channel_refs_json).unwrap_or_default();
    let template_result =
        validate_policy_template_ref(state, policy.template_ref.as_deref(), &channel_refs).await;
    if let Err(error) = template_result {
        result.valid = false;
        result.issues.push(error.message());
    }
    Ok(result)
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

const fn default_enabled() -> bool {
    true
}

const fn default_dedupe_seconds() -> i64 {
    300
}
