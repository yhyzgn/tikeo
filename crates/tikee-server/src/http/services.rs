//! Application services for HTTP auth, users, and RBAC.

use tikee_storage::{PermissionSummary, RbacRepository};

use super::{dto::MeResponse, error::ApiError};

/// RBAC service around repository-backed permission checks.
#[derive(Debug, Clone)]
pub struct RbacService {
    repo: RbacRepository,
}

impl RbacService {
    /// Build service from repository.
    #[must_use]
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

    /// Check whether a principal has a resource/action permission.
    #[must_use]
    pub fn principal_has_permission(
        &self,
        principal: &MeResponse,
        resource: &str,
        action: &str,
    ) -> bool {
        principal.roles.iter().any(|role| role == "admin")
            || principal.permissions.iter().any(|permission| {
                permission.resource == resource
                    && (permission.action == action || permission.action == "manage")
            })
    }
}
