use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::http::{
    AppState, auth,
    dto::{ApiResponse, EmptyData},
    error::ApiError,
};

use super::common::audit;

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoleRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub permission_ids: Vec<String>,
    pub menu_keys: Vec<String>,
    pub ui_action_keys: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleRequest {
    pub display_name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub permission_ids: Vec<String>,
    pub menu_keys: Vec<String>,
    pub ui_action_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MenuPermissionCatalogItem {
    pub key: String,
    pub label: String,
    pub group: String,
    pub route_path: String,
    pub required_permission: Option<tikeo_storage::PermissionSummary>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UiActionPermissionCatalogItem {
    pub key: String,
    pub label: String,
    pub page_key: String,
    pub operation: String,
    pub dangerous: bool,
    pub required_permission: Option<tikeo_storage::PermissionSummary>,
}

#[utoipa::path(get, path = "/api/v1/roles", tag = "roles")]
/// List managed RBAC roles.
///
/// # Errors
///
/// Returns an API error when authentication fails or storage cannot list roles.
pub async fn list_roles(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::RoleSummary>>>, ApiError> {
    auth::require_permission(&headers, &state, "roles", "read").await?;
    let roles = state.rbac.list_roles().await?;
    Ok(Json(ApiResponse::success(roles)))
}

#[utoipa::path(post, path = "/api/v1/roles", tag = "roles", request_body = CreateRoleRequest)]
/// Create a managed RBAC role.
///
/// # Errors
///
/// Returns an API error when permission checks, validation, uniqueness, audit, or storage fails.
pub async fn create_role(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateRoleRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::RoleSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "roles", "manage").await?;
    validate_role_name(&request.name)?;
    validate_role_shape(&request.display_name)?;
    if matches!(request.name.as_str(), "owner") {
        return Err(ApiError::bad_request("owner role is reserved"));
    }
    if state.rbac.role_exists_by_name(&request.name).await? {
        return Err(ApiError::bad_request(format!(
            "role already exists: {}",
            request.name
        )));
    }
    let created = state
        .rbac
        .create_role(tikeo_storage::CreateRole {
            name: request.name.trim().to_owned(),
            display_name: request.display_name.trim().to_owned(),
            description: request.description.unwrap_or_default().trim().to_owned(),
            enabled: request.enabled,
            permission_ids: request.permission_ids,
            menu_keys: request.menu_keys,
            ui_action_keys: request.ui_action_keys,
        })
        .await?;
    audit(
        &state,
        &principal.username,
        "create",
        "role",
        &created.id,
        Some(format!("name={}", created.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(created)))
}

#[utoipa::path(patch, path = "/api/v1/roles/{id}", tag = "roles", request_body = UpdateRoleRequest)]
/// Update a managed RBAC role.
///
/// # Errors
///
/// Returns an API error when permission checks, validation, built-in protection, or storage fails.
pub async fn update_role(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateRoleRequest>,
) -> Result<Json<ApiResponse<tikeo_storage::RoleSummary>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "roles", "manage").await?;
    let existing = state
        .rbac
        .get_role(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("role not found: {id}")))?;
    if existing.builtin {
        return Err(ApiError::bad_request(
            "built-in owner role cannot be modified",
        ));
    }
    validate_role_shape(&request.display_name)?;
    let updated = state
        .rbac
        .update_role(
            &id,
            tikeo_storage::UpdateRole {
                display_name: request.display_name.trim().to_owned(),
                description: request.description.unwrap_or_default().trim().to_owned(),
                enabled: request.enabled,
                permission_ids: request.permission_ids,
                menu_keys: request.menu_keys,
                ui_action_keys: request.ui_action_keys,
            },
        )
        .await?
        .ok_or_else(|| ApiError::not_found(format!("role not found: {id}")))?;
    audit(
        &state,
        &principal.username,
        "update",
        "role",
        &id,
        Some(format!("name={}", updated.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(updated)))
}

#[utoipa::path(delete, path = "/api/v1/roles/{id}", tag = "roles")]
/// Delete a non-builtin RBAC role.
///
/// # Errors
///
/// Returns an API error when permission checks fail, the role is protected or assigned, or storage fails.
pub async fn delete_role(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EmptyData>>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "roles", "manage").await?;
    let existing = state
        .rbac
        .get_role(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("role not found: {id}")))?;
    if existing.builtin {
        return Err(ApiError::bad_request(
            "built-in owner role cannot be deleted",
        ));
    }
    let users = state
        .users
        .count_by_role(&existing.name)
        .await
        .map_err(|error| ApiError::storage(&error))?;
    if users > 0 {
        return Err(ApiError::bad_request(format!(
            "role is still assigned to {users} user(s)"
        )));
    }
    let deleted = state.rbac.delete_role(&id).await?;
    if !deleted {
        return Err(ApiError::not_found(format!("role not found: {id}")));
    }
    audit(
        &state,
        &principal.username,
        "delete",
        "role",
        &id,
        Some(format!("name={}", existing.name)),
        &headers,
    )
    .await;
    Ok(Json(ApiResponse::success(EmptyData {})))
}

#[utoipa::path(get, path = "/api/v1/permissions/catalog", tag = "roles")]
/// Return the backend permission catalog used by role matrices.
///
/// # Errors
///
/// Returns an API error when authentication fails or storage cannot list the catalog.
pub async fn permission_catalog(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<tikeo_storage::PermissionCatalogItem>>>, ApiError> {
    auth::require_permission(&headers, &state, "roles", "read").await?;
    Ok(Json(ApiResponse::success(
        state.rbac.list_permission_catalog().await?,
    )))
}

#[utoipa::path(get, path = "/api/v1/menu-permissions/catalog", tag = "roles")]
/// Return the Web menu permission catalog used by role matrices.
///
/// # Errors
///
/// Returns an API error when authentication fails.
pub async fn menu_permission_catalog(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<MenuPermissionCatalogItem>>>, ApiError> {
    auth::require_permission(&headers, &state, "roles", "read").await?;
    Ok(Json(ApiResponse::success(
        MENU_PERMISSION_CATALOG.iter().map(menu_item).collect(),
    )))
}

#[utoipa::path(get, path = "/api/v1/ui-action-permissions/catalog", tag = "roles")]
/// Return the UI action-element permission catalog used by role matrices.
///
/// # Errors
///
/// Returns an API error when authentication fails.
pub async fn ui_action_permission_catalog(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<UiActionPermissionCatalogItem>>>, ApiError> {
    auth::require_permission(&headers, &state, "roles", "read").await?;
    Ok(Json(ApiResponse::success(
        UI_ACTION_PERMISSION_CATALOG
            .iter()
            .map(ui_action_item)
            .collect(),
    )))
}

fn validate_role_name(name: &str) -> Result<(), ApiError> {
    let name = name.trim();
    if name.len() < 2
        || !name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
    {
        return Err(ApiError::bad_request(
            "role name must use lowercase letters, digits, '-' or '_'",
        ));
    }
    Ok(())
}

fn validate_role_shape(display_name: &str) -> Result<(), ApiError> {
    if display_name.trim().is_empty() {
        return Err(ApiError::bad_request("display_name is required"));
    }
    Ok(())
}

fn menu_item(
    (key, label, group, path, permission): &MenuCatalogEntry,
) -> MenuPermissionCatalogItem {
    MenuPermissionCatalogItem {
        key: (*key).to_owned(),
        label: (*label).to_owned(),
        group: (*group).to_owned(),
        route_path: (*path).to_owned(),
        required_permission: permission.map(permission_summary),
    }
}

fn ui_action_item(
    (key, label, page, operation, dangerous, permission): &UiActionCatalogEntry,
) -> UiActionPermissionCatalogItem {
    UiActionPermissionCatalogItem {
        key: (*key).to_owned(),
        label: (*label).to_owned(),
        page_key: (*page).to_owned(),
        operation: (*operation).to_owned(),
        dangerous: *dangerous,
        required_permission: permission.map(permission_summary),
    }
}

fn permission_summary((resource, action): (&str, &str)) -> tikeo_storage::PermissionSummary {
    tikeo_storage::PermissionSummary {
        resource: resource.to_owned(),
        action: action.to_owned(),
    }
}

type MenuCatalogEntry = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Option<(&'static str, &'static str)>,
);
type UiActionCatalogEntry = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    bool,
    Option<(&'static str, &'static str)>,
);

const MENU_PERMISSION_CATALOG: &[MenuCatalogEntry] = &[
    ("/dashboard", "总览", "overview", "/dashboard", None),
    (
        "/jobs",
        "任务",
        "orchestration",
        "/jobs",
        Some(("jobs", "read")),
    ),
    (
        "/workflows",
        "工作流",
        "orchestration",
        "/workflows",
        Some(("workflows", "read")),
    ),
    (
        "/instances",
        "实例",
        "orchestration",
        "/instances",
        Some(("instances", "read")),
    ),
    (
        "/workers",
        "Worker 集群",
        "runtime",
        "/workers",
        Some(("workers", "read")),
    ),
    (
        "/workers/dispatch-queue",
        "调度队列",
        "runtime",
        "/workers/dispatch-queue",
        Some(("workers", "read")),
    ),
    (
        "/scripts",
        "脚本管理",
        "runtime",
        "/scripts",
        Some(("scripts", "read")),
    ),
    (
        "/plugins",
        "插件系统",
        "runtime",
        "/plugins",
        Some(("tenants", "read")),
    ),
    (
        "/scopes",
        "租户范围",
        "governance",
        "/scopes",
        Some(("tenants", "read")),
    ),
    (
        "/users",
        "用户管理",
        "governance",
        "/users",
        Some(("users", "read")),
    ),
    (
        "/roles",
        "角色管理",
        "governance",
        "/roles",
        Some(("roles", "read")),
    ),
    (
        "/calendars",
        "调度日历",
        "governance",
        "/calendars",
        Some(("tenants", "read")),
    ),
    (
        "/api-keys",
        "API-Key",
        "governance",
        "/api-keys",
        Some(("tenants", "manage")),
    ),
    (
        "/gitops",
        "GitOps/IaC",
        "governance",
        "/gitops",
        Some(("tenants", "read")),
    ),
    (
        "/alerts",
        "告警投递",
        "observability",
        "/alerts",
        Some(("audit", "read")),
    ),
    (
        "/audit",
        "审计日志",
        "observability",
        "/audit",
        Some(("audit", "read")),
    ),
];

const UI_ACTION_PERMISSION_CATALOG: &[UiActionCatalogEntry] = &[
    (
        "users.create",
        "创建用户",
        "/users",
        "create",
        false,
        Some(("users", "manage")),
    ),
    (
        "users.edit",
        "编辑用户",
        "/users",
        "edit",
        false,
        Some(("users", "manage")),
    ),
    (
        "users.delete",
        "删除用户",
        "/users",
        "delete",
        true,
        Some(("users", "manage")),
    ),
    (
        "roles.create",
        "创建角色",
        "/roles",
        "create",
        false,
        Some(("roles", "manage")),
    ),
    (
        "roles.edit",
        "编辑角色",
        "/roles",
        "edit",
        false,
        Some(("roles", "manage")),
    ),
    (
        "roles.delete",
        "删除角色",
        "/roles",
        "delete",
        true,
        Some(("roles", "manage")),
    ),
    (
        "roles.permissions.edit",
        "编辑权限矩阵",
        "/roles",
        "edit",
        false,
        Some(("roles", "manage")),
    ),
    (
        "jobs.create",
        "创建任务",
        "/jobs",
        "create",
        false,
        Some(("jobs", "write")),
    ),
    (
        "jobs.edit",
        "编辑任务",
        "/jobs",
        "edit",
        false,
        Some(("jobs", "write")),
    ),
    (
        "jobs.delete",
        "删除任务",
        "/jobs",
        "delete",
        true,
        Some(("jobs", "write")),
    ),
    (
        "jobs.trigger",
        "触发任务",
        "/jobs",
        "execute",
        false,
        Some(("instances", "execute")),
    ),
    (
        "scripts.create",
        "创建脚本",
        "/scripts",
        "create",
        false,
        Some(("scripts", "manage")),
    ),
    (
        "scripts.edit",
        "编辑脚本",
        "/scripts",
        "edit",
        false,
        Some(("scripts", "manage")),
    ),
    (
        "scripts.delete",
        "删除脚本",
        "/scripts",
        "delete",
        true,
        Some(("scripts", "manage")),
    ),
    (
        "scripts.publish",
        "发布脚本",
        "/scripts",
        "publish",
        false,
        Some(("scripts", "manage")),
    ),
    (
        "workflows.create",
        "创建工作流",
        "/workflows",
        "create",
        false,
        Some(("workflows", "manage")),
    ),
    (
        "workflows.edit",
        "编辑工作流",
        "/workflows",
        "edit",
        false,
        Some(("workflows", "manage")),
    ),
    (
        "workflows.run",
        "运行工作流",
        "/workflows",
        "execute",
        false,
        Some(("workflows", "execute")),
    ),
    (
        "apiKeys.create",
        "创建 API-Key",
        "/api-keys",
        "create",
        false,
        Some(("tenants", "manage")),
    ),
    (
        "apiKeys.edit",
        "编辑 API-Key",
        "/api-keys",
        "edit",
        false,
        Some(("tenants", "manage")),
    ),
    (
        "apiKeys.delete",
        "吊销 API-Key",
        "/api-keys",
        "delete",
        true,
        Some(("tenants", "manage")),
    ),
];
