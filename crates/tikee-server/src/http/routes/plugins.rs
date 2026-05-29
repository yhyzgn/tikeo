#![allow(missing_docs)]

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::http::{
    AppState, auth,
    dto::{ApiResponse, EmptyData},
    error::ApiError,
};

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePluginRequest {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub processor_types: Vec<tikee_storage::PluginProcessorTypeSummary>,
    #[serde(default)]
    pub alert_channel_types: Vec<tikee_storage::PluginAlertChannelTypeSummary>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePluginRequest {
    pub name: Option<String>,
    pub kind: Option<String>,
    pub processor_types: Option<Vec<tikee_storage::PluginProcessorTypeSummary>>,
    pub alert_channel_types: Option<Vec<tikee_storage::PluginAlertChannelTypeSummary>>,
    pub enabled: Option<bool>,
}

#[utoipa::path(get, path = "/api/v1/plugins", tag = "plugins")]
pub async fn list_plugins(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<tikee_storage::PluginSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "tenants", "read").await?;
    let items = state
        .plugins
        .list_plugins()
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(Json(ApiResponse::success(items)))
}

#[utoipa::path(post, path = "/api/v1/plugins", tag = "plugins", request_body = CreatePluginRequest)]
pub async fn create_plugin(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreatePluginRequest>,
) -> Result<Json<ApiResponse<tikee_storage::PluginSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    validate_plugin_declaration(&request.processor_types, &request.alert_channel_types)?;
    let created = state
        .plugins
        .create_plugin(tikee_storage::CreatePlugin {
            name: request.name,
            kind: request.kind,
            processor_types: request.processor_types,
            alert_channel_types: request.alert_channel_types,
            enabled: request.enabled,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    super::common::audit(
        &state,
        &principal.username,
        "create",
        "plugin",
        &created.id,
        Some(format!("name={}", created.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(created)))
}

#[utoipa::path(patch, path = "/api/v1/plugins/{id}", tag = "plugins", request_body = UpdatePluginRequest)]
pub async fn update_plugin(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdatePluginRequest>,
) -> Result<Json<ApiResponse<tikee_storage::PluginSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    if let Some(processors) = &request.processor_types {
        validate_processor_types(processors)?;
    }
    if let Some(channels) = &request.alert_channel_types {
        validate_alert_channel_types(channels)?;
    }
    let updated = state
        .plugins
        .update_plugin(
            &id,
            tikee_storage::UpdatePlugin {
                name: request.name,
                kind: request.kind,
                processor_types: request.processor_types,
                alert_channel_types: request.alert_channel_types,
                enabled: request.enabled,
            },
        )
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found("plugin not found"))?;
    super::common::audit(
        &state,
        &principal.username,
        "update",
        "plugin",
        &updated.id,
        Some(format!("name={}", updated.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(updated)))
}

#[utoipa::path(delete, path = "/api/v1/plugins/{id}", tag = "plugins")]
pub async fn delete_plugin(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EmptyData>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "tenants", "manage").await?;
    let deleted = state
        .plugins
        .delete_plugin(&id)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if !deleted {
        return Err(ApiError::not_found("plugin not found"));
    }
    super::common::audit(
        &state,
        &principal.username,
        "delete",
        "plugin",
        &id,
        None,
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(EmptyData {})))
}

fn default_enabled() -> bool {
    true
}

fn validate_plugin_declaration(
    processors: &[tikee_storage::PluginProcessorTypeSummary],
    channels: &[tikee_storage::PluginAlertChannelTypeSummary],
) -> Result<(), ApiError> {
    validate_processor_types(processors)?;
    validate_alert_channel_types(channels)
}

fn validate_processor_types(
    processors: &[tikee_storage::PluginProcessorTypeSummary],
) -> Result<(), ApiError> {
    for processor in processors {
        if !valid_slug(&processor.r#type) {
            return Err(ApiError::bad_request(
                "processor type must be a lowercase slug",
            ));
        }
        if processor.capability.trim().is_empty() {
            return Err(ApiError::bad_request(
                "processor type capability is required",
            ));
        }
        if processor
            .processor_names
            .iter()
            .any(|name| name.trim().is_empty())
        {
            return Err(ApiError::bad_request(
                "processor type processorNames must not contain blank values",
            ));
        }
        if processor.r#type == "sdk" || processor.r#type == "script" {
            return Err(ApiError::bad_request(
                "sdk/script processor types are built in",
            ));
        }
        if processor.r#type == "external_jar" {
            if processor
                .artifact_ref
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
            {
                return Err(ApiError::bad_request(
                    "external_jar processor type requires artifactRef",
                ));
            }
            if processor
                .container_image
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
            {
                return Err(ApiError::bad_request(
                    "external_jar processor type requires containerImage",
                ));
            }
        }
    }
    Ok(())
}

fn validate_alert_channel_types(
    channels: &[tikee_storage::PluginAlertChannelTypeSummary],
) -> Result<(), ApiError> {
    for channel in channels {
        if !valid_slug(&channel.r#type) {
            return Err(ApiError::bad_request(
                "alert channel type must be a lowercase slug",
            ));
        }
        if channel.target_kind != "webhook" {
            return Err(ApiError::bad_request(
                "custom alert channel targetKind currently supports webhook only",
            ));
        }
    }
    Ok(())
}

fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}
