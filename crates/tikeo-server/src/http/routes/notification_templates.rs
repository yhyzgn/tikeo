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
        dto::{ApiResponse, EmptyData, NullableStringUpdate},
        error::ApiError,
    },
    notification::{render_notification_template_preview, validate_notification_template_tokens},
};

use super::notification_providers::{
    json_to_string, valid_slug, validate_provider_message_template_for_state,
};

#[derive(Debug, Clone, Default, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct NotificationTemplateQuery {
    pub provider: Option<String>,
    /// Message type value.
    pub message_type: Option<String>,
    /// Boolean state flag.
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotificationTemplateRequest {
    /// Template key value.
    pub template_key: String,
    pub name: String,
    pub description: Option<String>,
    pub provider: String,
    /// Message type value.
    pub message_type: String,
    #[serde(default = "default_enabled")]
    /// Boolean state flag.
    pub enabled: bool,
    #[serde(default)]
    pub body: serde_json::Value,
    #[serde(default)]
    pub variables: serde_json::Value,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotificationTemplateRequest {
    /// Template key value.
    pub template_key: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub description: NullableStringUpdate,
    pub provider: Option<String>,
    /// Message type value.
    pub message_type: Option<String>,
    /// Boolean state flag.
    pub enabled: Option<bool>,
    pub body: Option<serde_json::Value>,
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderNotificationTemplateRequest {
    pub provider: Option<String>,
    /// Message type value.
    pub message_type: Option<String>,
    #[serde(default)]
    pub template: serde_json::Value,
    #[serde(default)]
    pub sample: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderNotificationTemplateResponse {
    pub provider: String,
    /// Message type value.
    pub message_type: String,
    pub rendered: serde_json::Value,
}

#[utoipa::path(
    get,
    path = "/api/v1/notification-templates",
    tag = "notifications",
    params(NotificationTemplateQuery)
)]
/// List notification templates.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn list_notification_templates(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<NotificationTemplateQuery>,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::NotificationTemplateSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let items = state
        .notification_templates
        .list_templates(tikeo_storage::NotificationTemplateFilters {
            provider: query.provider,
            message_type: query.message_type,
            enabled: query.enabled,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/notification-templates", tag = "notifications", request_body = CreateNotificationTemplateRequest)]
/// Create notification template.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn create_notification_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateNotificationTemplateRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::NotificationTemplateSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "notifications", "manage").await?;
    validate_template_request(
        &state,
        &request.template_key,
        &request.name,
        &request.provider,
        &request.message_type,
        &request.body,
    )
    .await?;
    let created = state
        .notification_templates
        .create_template(tikeo_storage::CreateNotificationTemplate {
            template_key: request.template_key,
            name: request.name,
            description: request.description,
            provider: request.provider,
            message_type: request.message_type,
            enabled: request.enabled,
            body_json: json_to_string(&request.body),
            variables_json: json_to_string(&request.variables),
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    super::common::audit(
        &state,
        &principal.username,
        "create",
        "notification_template",
        &created.id,
        Some(format!(
            "key={}, provider={}",
            created.template_key, created.provider
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(created)))
}

#[utoipa::path(
    get,
    path = "/api/v1/notification-templates/{id}",
    tag = "notifications"
)]
/// Get notification template.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn get_notification_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<tikeo_storage::NotificationTemplateSummary>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let item = state
        .notification_templates
        .get_template(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("notification template not found"))?;
    Ok(Json(ApiResponse::success(item)))
}

#[utoipa::path(patch, path = "/api/v1/notification-templates/{id}", tag = "notifications", request_body = UpdateNotificationTemplateRequest)]
/// Update notification template.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn update_notification_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateNotificationTemplateRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::NotificationTemplateSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "notifications", "manage").await?;
    let existing = state
        .notification_templates
        .get_template(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("notification template not found"))?;
    let existing_body =
        serde_json::from_str(&existing.body_json).unwrap_or(serde_json::Value::Null);
    validate_template_request(
        &state,
        request
            .template_key
            .as_ref()
            .unwrap_or(&existing.template_key),
        request.name.as_ref().unwrap_or(&existing.name),
        request.provider.as_ref().unwrap_or(&existing.provider),
        request
            .message_type
            .as_ref()
            .unwrap_or(&existing.message_type),
        request.body.as_ref().unwrap_or(&existing_body),
    )
    .await?;
    let updated = state
        .notification_templates
        .update_template(
            &existing.id,
            tikeo_storage::UpdateNotificationTemplate {
                template_key: request.template_key,
                name: request.name,
                description: request.description.into_option_option(),
                provider: request.provider,
                message_type: request.message_type,
                enabled: request.enabled,
                body_json: request.body.map(|value| json_to_string(&value)),
                variables_json: request.variables.map(|value| json_to_string(&value)),
                updated_by: Some(Some(principal.username.clone())),
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("notification template not found"))?;
    super::common::audit(
        &state,
        &principal.username,
        "update",
        "notification_template",
        &updated.id,
        Some(format!(
            "key={}, provider={}",
            updated.template_key, updated.provider
        )),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(updated)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/notification-templates/{id}",
    tag = "notifications"
)]
/// Delete notification template.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn delete_notification_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EmptyData>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "notifications", "manage").await?;
    let deleted = state
        .notification_templates
        .delete_template(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !deleted {
        return Err(ApiError::not_found("notification template not found"));
    }
    super::common::audit(
        &state,
        &principal.username,
        "delete",
        "notification_template",
        &id,
        None,
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(EmptyData {})))
}

#[utoipa::path(post, path = "/api/v1/notification-templates/{id}:render", tag = "notifications", request_body = RenderNotificationTemplateRequest)]
/// Render notification template.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn render_notification_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(template_action): Path<String>,
    Json(request): Json<RenderNotificationTemplateRequest>,
) -> Result<Json<ApiResponse<RenderNotificationTemplateResponse>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    let id = template_action
        .strip_suffix(":render")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "unsupported notification template action: {template_action}"
            ))
        })?;
    render_notification_template_for_id(&state, id, request).await
}

#[utoipa::path(post, path = "/api/v1/notification-templates/{id}/render", tag = "notifications", request_body = RenderNotificationTemplateRequest)]
/// Render notification template by id.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn render_notification_template_by_id(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<RenderNotificationTemplateRequest>,
) -> Result<Json<ApiResponse<RenderNotificationTemplateResponse>>, ApiError> {
    auth::require_permission(&headers, &state, "notifications", "read").await?;
    render_notification_template_for_id(&state, &id, request).await
}

async fn render_notification_template_for_id(
    state: &AppState,
    id: &str,
    request: RenderNotificationTemplateRequest,
) -> Result<Json<ApiResponse<RenderNotificationTemplateResponse>>, ApiError> {
    let existing = state
        .notification_templates
        .get_template(id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let provider = request
        .provider
        .or_else(|| existing.as_ref().map(|item| item.provider.clone()))
        .unwrap_or_else(|| "webhook".to_owned());
    let message_type = request
        .message_type
        .or_else(|| existing.as_ref().map(|item| item.message_type.clone()))
        .unwrap_or_else(|| "json".to_owned());
    let template = if request.template.is_null() {
        existing
            .as_ref()
            .and_then(|item| serde_json::from_str(&item.body_json).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        request.template
    };
    validate_template_request(state, id, id, &provider, &message_type, &template).await?;
    Ok(Json(ApiResponse::success(
        RenderNotificationTemplateResponse {
            provider,
            message_type,
            rendered: render_notification_template_preview(&template, &request.sample),
        },
    )))
}

async fn validate_template_request(
    state: &AppState,
    template_key: &str,
    name: &str,
    provider: &str,
    message_type: &str,
    body: &serde_json::Value,
) -> Result<(), ApiError> {
    if !valid_template_key(template_key) {
        return Err(ApiError::bad_request(
            "templateKey must contain only letters, numbers, dot, underscore, or dash",
        ));
    }
    if name.trim().is_empty() {
        return Err(ApiError::bad_request(
            "notification template name is required",
        ));
    }
    if !valid_slug(provider) {
        return Err(ApiError::bad_request("provider must be a lowercase slug"));
    }
    validate_notification_template_tokens(body).map_err(|error| {
        ApiError::bad_request(format!("notification template is unsafe: {error}"))
    })?;
    let config = serde_json::json!({
        "messageType": message_type,
        "template": body,
    });
    validate_provider_message_template_for_state(state, provider, &config).await
}

fn valid_template_key(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

const fn default_enabled() -> bool {
    true
}
