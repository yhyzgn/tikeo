//! Application services for HTTP auth, users, and RBAC.

use tikeo_storage::{
    CreateRole, PermissionCatalogItem, PermissionSummary, RbacRepository, RoleSummary, UpdateRole,
};

use super::{dto::MeResponse, error::ApiError};

/// RBAC service around repository-backed permission checks.
#[derive(Debug, Clone)]
pub struct RbacService {
    repo: RbacRepository,
}

impl RbacService {
    /// Build service from repository.
    #[must_use]
    /// New.
    pub const fn new(repo: RbacRepository) -> Self {
        Self { repo }
    }

    /// Return permissions granted by all roles.
    ///
    /// # Errors
    ///
    /// Returns an API storage error when repository access fails.
    pub async fn permissions_for_roles(
        &self,
        roles: &[String],
    ) -> Result<Vec<PermissionSummary>, ApiError> {
        self.repo
            .permissions_for_roles(roles)
            .await
            .map_err(|error| ApiError::storage(&error))
    }

    /// List managed roles.
    ///
    /// # Errors
    ///
    /// Returns an API storage error when repository access fails.
    pub async fn list_roles(&self) -> Result<Vec<RoleSummary>, ApiError> {
        self.repo
            .list_roles()
            .await
            .map_err(|error| ApiError::storage(&error))
    }

    /// Get a managed role by id.
    ///
    /// # Errors
    ///
    /// Returns an API storage error when repository access fails.
    pub async fn get_role(&self, id: &str) -> Result<Option<RoleSummary>, ApiError> {
        self.repo
            .get_role(id)
            .await
            .map_err(|error| ApiError::storage(&error))
    }

    /// Check whether a role name already exists.
    ///
    /// # Errors
    ///
    /// Returns an API storage error when repository access fails.
    pub async fn role_exists_by_name(&self, name: &str) -> Result<bool, ApiError> {
        self.repo
            .role_exists_by_name(name)
            .await
            .map_err(|error| ApiError::storage(&error))
    }

    /// Create a managed role.
    ///
    /// # Errors
    ///
    /// Returns an API storage error when repository access fails.
    pub async fn create_role(&self, input: CreateRole) -> Result<RoleSummary, ApiError> {
        self.repo
            .create_role(input)
            .await
            .map_err(|error| ApiError::storage(&error))
    }

    /// Update a managed role.
    ///
    /// # Errors
    ///
    /// Returns an API storage error when repository access fails.
    pub async fn update_role(
        &self,
        id: &str,
        input: UpdateRole,
    ) -> Result<Option<RoleSummary>, ApiError> {
        self.repo
            .update_role(id, input)
            .await
            .map_err(|error| ApiError::storage(&error))
    }

    /// Delete a managed role.
    ///
    /// # Errors
    ///
    /// Returns an API storage error when repository access fails.
    pub async fn delete_role(&self, id: &str) -> Result<bool, ApiError> {
        self.repo
            .delete_role(id)
            .await
            .map_err(|error| ApiError::storage(&error))
    }

    /// List backend permission catalog items.
    ///
    /// # Errors
    ///
    /// Returns an API storage error when repository access fails.
    pub async fn list_permission_catalog(&self) -> Result<Vec<PermissionCatalogItem>, ApiError> {
        self.repo
            .list_permission_catalog()
            .await
            .map_err(|error| ApiError::storage(&error))
    }

    /// Check whether a principal has a resource/action permission.
    #[must_use]
    /// Principal has permission.
    pub fn principal_has_permission(
        &self,
        principal: &MeResponse,
        resource: &str,
        action: &str,
    ) -> bool {
        principal.bootstrap_admin
            || principal.permissions.iter().any(|permission| {
                permission.resource == resource
                    && (permission.action == action || permission.action == "manage")
            })
    }
}
