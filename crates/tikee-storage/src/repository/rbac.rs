use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use crate::entities::{
    permission, role, role_menu_permission, role_permission, role_ui_action_permission,
};

use super::{
    auth::PermissionSummary,
    util::{new_id, now_rfc3339},
};

/// Role catalog entry returned by storage repositories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleSummary {
    /// Stable role id.
    pub id: String,
    /// Role key used by users and API clients.
    pub name: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Human-readable description.
    pub description: String,
    /// Whether this role is protected by the platform.
    pub builtin: bool,
    /// Whether this role can grant permissions to users.
    pub enabled: bool,
    /// Backend API permissions granted by this role.
    pub permissions: Vec<PermissionSummary>,
    /// Menu keys visible to this role.
    pub menu_keys: Vec<String>,
    /// UI operation element keys granted to this role.
    pub ui_action_keys: Vec<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Permission catalog entry persisted in storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PermissionCatalogItem {
    /// Stable permission id.
    pub id: String,
    /// Resource name.
    pub resource: String,
    /// Action name.
    pub action: String,
    /// Human-readable description.
    pub description: String,
}

/// Role creation input.
#[derive(Debug, Clone)]
pub struct CreateRole {
    /// Unique role key.
    pub name: String,
    /// Display name.
    pub display_name: String,
    /// Optional description.
    pub description: String,
    /// Initial enabled flag.
    pub enabled: bool,
    /// Initial backend permission ids.
    pub permission_ids: Vec<String>,
    /// Initial menu keys.
    pub menu_keys: Vec<String>,
    /// Initial UI action keys.
    pub ui_action_keys: Vec<String>,
}

/// Role update input.
#[derive(Debug, Clone)]
pub struct UpdateRole {
    /// Display name.
    pub display_name: String,
    /// Optional description.
    pub description: String,
    /// Enabled flag.
    pub enabled: bool,
    /// Replacement backend permission ids.
    pub permission_ids: Vec<String>,
    /// Replacement menu keys.
    pub menu_keys: Vec<String>,
    /// Replacement UI action keys.
    pub ui_action_keys: Vec<String>,
}

/// RBAC repository using soft relations between users, roles, and permissions.
#[derive(Debug, Clone)]
pub struct RbacRepository {
    db: DatabaseConnection,
}

impl RbacRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// List all roles with their permission matrices.
    pub async fn list_roles(&self) -> Result<Vec<RoleSummary>, sea_orm::DbErr> {
        let roles = role::Entity::find().all(&self.db).await?;
        let mut summaries = Vec::with_capacity(roles.len());
        for role_model in roles {
            summaries.push(self.role_summary(role_model).await?);
        }
        summaries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(summaries)
    }

    /// Get one role by id.
    pub async fn get_role(&self, id: &str) -> Result<Option<RoleSummary>, sea_orm::DbErr> {
        let Some(role_model) = role::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(self.role_summary(role_model).await?))
    }

    /// Find a role by name.
    pub async fn role_exists_by_name(&self, name: &str) -> Result<bool, sea_orm::DbErr> {
        Ok(role::Entity::find()
            .filter(role::Column::Name.eq(name.to_owned()))
            .one(&self.db)
            .await?
            .is_some())
    }

    /// Create a custom role.
    pub async fn create_role(&self, input: CreateRole) -> Result<RoleSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let role_id = new_id("role");
        role::ActiveModel {
            id: Set(role_id.clone()),
            name: Set(input.name),
            description: Set(input.description),
            display_name: Set(input.display_name),
            builtin: Set(false),
            enabled: Set(input.enabled),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        self.replace_role_permissions(&role_id, &input.permission_ids)
            .await?;
        self.replace_role_menu_permissions(&role_id, &input.menu_keys)
            .await?;
        self.replace_role_ui_action_permissions(&role_id, &input.ui_action_keys)
            .await?;
        self.get_role(&role_id)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(role_id))
    }

    /// Update a role and replace all matrices atomically enough for the single metadata DB boundary.
    pub async fn update_role(
        &self,
        id: &str,
        input: UpdateRole,
    ) -> Result<Option<RoleSummary>, sea_orm::DbErr> {
        let Some(existing) = role::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let mut active: role::ActiveModel = existing.into();
        active.description = Set(input.description);
        active.display_name = Set(input.display_name);
        active.enabled = Set(input.enabled);
        active.updated_at = Set(now_rfc3339());
        active.update(&self.db).await?;
        self.replace_role_permissions(id, &input.permission_ids)
            .await?;
        self.replace_role_menu_permissions(id, &input.menu_keys)
            .await?;
        self.replace_role_ui_action_permissions(id, &input.ui_action_keys)
            .await?;
        self.get_role(id).await
    }

    /// Delete a non-builtin role.
    pub async fn delete_role(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        let Some(existing) = role::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        if existing.builtin {
            return Ok(false);
        }
        role_permission::Entity::delete_many()
            .filter(role_permission::Column::RoleId.eq(id.to_owned()))
            .exec(&self.db)
            .await?;
        role_menu_permission::Entity::delete_many()
            .filter(role_menu_permission::Column::RoleId.eq(id.to_owned()))
            .exec(&self.db)
            .await?;
        role_ui_action_permission::Entity::delete_many()
            .filter(role_ui_action_permission::Column::RoleId.eq(id.to_owned()))
            .exec(&self.db)
            .await?;
        let result = role::Entity::delete_by_id(id.to_owned())
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// List backend permission catalog items.
    pub async fn list_permission_catalog(
        &self,
    ) -> Result<Vec<PermissionCatalogItem>, sea_orm::DbErr> {
        let mut rows = permission::Entity::find()
            .all(&self.db)
            .await?
            .into_iter()
            .map(|item| PermissionCatalogItem {
                id: item.id,
                resource: item.resource,
                action: item.action,
                description: item.description,
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            left.resource
                .cmp(&right.resource)
                .then(left.action.cmp(&right.action))
        });
        Ok(rows)
    }

    /// List permissions granted to a role name.
    pub async fn permissions_for_role(
        &self,
        role_name: &str,
    ) -> Result<Vec<PermissionSummary>, sea_orm::DbErr> {
        let Some(role_model) = role::Entity::find()
            .filter(role::Column::Name.eq(role_name.to_owned()))
            .filter(role::Column::Enabled.eq(true))
            .one(&self.db)
            .await?
        else {
            return Ok(Vec::new());
        };
        self.permissions_for_role_id(&role_model.id).await
    }

    /// List permissions granted across multiple roles.
    pub async fn permissions_for_roles(
        &self,
        roles: &[String],
    ) -> Result<Vec<PermissionSummary>, sea_orm::DbErr> {
        let mut permissions = Vec::new();
        for role_name in roles {
            permissions.extend(self.permissions_for_role(role_name).await?);
        }
        sort_permissions(&mut permissions);
        Ok(permissions)
    }

    /// List every backend permission in the catalog.
    pub async fn all_permissions(&self) -> Result<Vec<PermissionSummary>, sea_orm::DbErr> {
        let mut permissions = permission::Entity::find()
            .all(&self.db)
            .await?
            .into_iter()
            .map(|permission| PermissionSummary {
                resource: permission.resource,
                action: permission.action,
            })
            .collect::<Vec<_>>();
        sort_permissions(&mut permissions);
        Ok(permissions)
    }

    /// Check whether any role grants a resource/action permission.
    pub async fn has_permission(
        &self,
        roles: &[String],
        resource: &str,
        action: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let permissions = self.permissions_for_roles(roles).await?;
        Ok(permissions.iter().any(|permission| {
            permission.resource == resource
                && (permission.action == action || permission.action == "manage")
        }))
    }

    /// List menu keys across role names.
    pub async fn menu_keys_for_roles(
        &self,
        roles: &[String],
    ) -> Result<Vec<String>, sea_orm::DbErr> {
        let mut keys = Vec::new();
        for role_name in roles {
            let Some(role_model) = role::Entity::find()
                .filter(role::Column::Name.eq(role_name.to_owned()))
                .filter(role::Column::Enabled.eq(true))
                .one(&self.db)
                .await?
            else {
                continue;
            };
            keys.extend(self.menu_keys_for_role_id(&role_model.id).await?);
        }
        keys.sort();
        keys.dedup();
        Ok(keys)
    }

    /// List every menu key known to persisted role-menu bindings.
    pub async fn all_menu_keys(&self) -> Result<Vec<String>, sea_orm::DbErr> {
        let mut keys = role_menu_permission::Entity::find()
            .all(&self.db)
            .await?
            .into_iter()
            .map(|binding| binding.menu_key)
            .collect::<Vec<_>>();
        keys.sort();
        keys.dedup();
        Ok(keys)
    }

    /// List UI action keys across role names.
    pub async fn ui_action_keys_for_roles(
        &self,
        roles: &[String],
    ) -> Result<Vec<String>, sea_orm::DbErr> {
        let mut keys = Vec::new();
        for role_name in roles {
            let Some(role_model) = role::Entity::find()
                .filter(role::Column::Name.eq(role_name.to_owned()))
                .filter(role::Column::Enabled.eq(true))
                .one(&self.db)
                .await?
            else {
                continue;
            };
            keys.extend(self.ui_action_keys_for_role_id(&role_model.id).await?);
        }
        keys.sort();
        keys.dedup();
        Ok(keys)
    }

    /// List every UI action key known to persisted role-action bindings.
    pub async fn all_ui_action_keys(&self) -> Result<Vec<String>, sea_orm::DbErr> {
        let mut keys = role_ui_action_permission::Entity::find()
            .all(&self.db)
            .await?
            .into_iter()
            .map(|binding| binding.ui_action_key)
            .collect::<Vec<_>>();
        keys.sort();
        keys.dedup();
        Ok(keys)
    }

    async fn role_summary(&self, role_model: role::Model) -> Result<RoleSummary, sea_orm::DbErr> {
        Ok(RoleSummary {
            permissions: self.permissions_for_role_id(&role_model.id).await?,
            menu_keys: self.menu_keys_for_role_id(&role_model.id).await?,
            ui_action_keys: self.ui_action_keys_for_role_id(&role_model.id).await?,
            id: role_model.id,
            name: role_model.name,
            display_name: role_model.display_name,
            description: role_model.description,
            builtin: role_model.builtin,
            enabled: role_model.enabled,
            created_at: role_model.created_at,
            updated_at: role_model.updated_at,
        })
    }

    async fn permissions_for_role_id(
        &self,
        role_id: &str,
    ) -> Result<Vec<PermissionSummary>, sea_orm::DbErr> {
        let bindings = role_permission::Entity::find()
            .filter(role_permission::Column::RoleId.eq(role_id.to_owned()))
            .all(&self.db)
            .await?;
        let mut permissions = Vec::new();
        for binding in bindings {
            if let Some(permission_model) = permission::Entity::find_by_id(binding.permission_id)
                .one(&self.db)
                .await?
            {
                permissions.push(PermissionSummary {
                    resource: permission_model.resource,
                    action: permission_model.action,
                });
            }
        }
        sort_permissions(&mut permissions);
        Ok(permissions)
    }

    async fn menu_keys_for_role_id(&self, role_id: &str) -> Result<Vec<String>, sea_orm::DbErr> {
        let mut keys = role_menu_permission::Entity::find()
            .filter(role_menu_permission::Column::RoleId.eq(role_id.to_owned()))
            .all(&self.db)
            .await?
            .into_iter()
            .map(|binding| binding.menu_key)
            .collect::<Vec<_>>();
        keys.sort();
        keys.dedup();
        Ok(keys)
    }

    async fn ui_action_keys_for_role_id(
        &self,
        role_id: &str,
    ) -> Result<Vec<String>, sea_orm::DbErr> {
        let mut keys = role_ui_action_permission::Entity::find()
            .filter(role_ui_action_permission::Column::RoleId.eq(role_id.to_owned()))
            .all(&self.db)
            .await?
            .into_iter()
            .map(|binding| binding.ui_action_key)
            .collect::<Vec<_>>();
        keys.sort();
        keys.dedup();
        Ok(keys)
    }

    async fn replace_role_permissions(
        &self,
        role_id: &str,
        permission_ids: &[String],
    ) -> Result<(), sea_orm::DbErr> {
        role_permission::Entity::delete_many()
            .filter(role_permission::Column::RoleId.eq(role_id.to_owned()))
            .exec(&self.db)
            .await?;
        for permission_id in normalized(permission_ids) {
            role_permission::ActiveModel {
                id: Set(format!("rp-{role_id}-{permission_id}")),
                role_id: Set(role_id.to_owned()),
                permission_id: Set(permission_id),
                created_at: Set(now_rfc3339()),
            }
            .insert(&self.db)
            .await?;
        }
        Ok(())
    }

    async fn replace_role_menu_permissions(
        &self,
        role_id: &str,
        menu_keys: &[String],
    ) -> Result<(), sea_orm::DbErr> {
        role_menu_permission::Entity::delete_many()
            .filter(role_menu_permission::Column::RoleId.eq(role_id.to_owned()))
            .exec(&self.db)
            .await?;
        for menu_key in normalized(menu_keys) {
            role_menu_permission::ActiveModel {
                id: Set(format!("rmp-{role_id}-{menu_key}").replace('/', "_")),
                role_id: Set(role_id.to_owned()),
                menu_key: Set(menu_key),
                created_at: Set(now_rfc3339()),
            }
            .insert(&self.db)
            .await?;
        }
        Ok(())
    }

    async fn replace_role_ui_action_permissions(
        &self,
        role_id: &str,
        ui_action_keys: &[String],
    ) -> Result<(), sea_orm::DbErr> {
        role_ui_action_permission::Entity::delete_many()
            .filter(role_ui_action_permission::Column::RoleId.eq(role_id.to_owned()))
            .exec(&self.db)
            .await?;
        for ui_action_key in normalized(ui_action_keys) {
            role_ui_action_permission::ActiveModel {
                id: Set(format!("rua-{role_id}-{ui_action_key}").replace(['/', '.'], "_")),
                role_id: Set(role_id.to_owned()),
                ui_action_key: Set(ui_action_key),
                created_at: Set(now_rfc3339()),
            }
            .insert(&self.db)
            .await?;
        }
        Ok(())
    }
}

fn sort_permissions(permissions: &mut Vec<PermissionSummary>) {
    permissions.sort_by(|left, right| {
        left.resource
            .cmp(&right.resource)
            .then(left.action.cmp(&right.action))
    });
    permissions.dedup();
}

fn normalized(values: &[String]) -> Vec<String> {
    let mut values = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}
