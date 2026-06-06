use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::entities::{auth_session, permission, role, role_permission, user};

use super::util::{new_id, now_rfc3339};
/// Persisted session creation input.
#[derive(Debug, Clone)]
pub struct CreateAuthSession {
    /// Related user id.
    pub user_id: String,
    /// SHA-256 hash of the opaque access token.
    pub token_hash: String,
    /// Optional device identifier.
    pub device_id: Option<String>,
    /// Optional device display name.
    pub device_name: Option<String>,
    /// RFC3339 expiration timestamp.
    pub expires_at: String,
}

/// Persisted auth session plus principal snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthSessionSummary {
    /// Session id.
    pub id: String,
    /// User id.
    pub user_id: String,
    /// Username.
    pub username: String,
    /// Role.
    pub role: String,
    /// Token hash.
    pub token_hash: String,
    /// Optional device id.
    pub device_id: Option<String>,
    /// Optional device name.
    pub device_name: Option<String>,
    /// Expiration timestamp.
    pub expires_at: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Auth session repository backed by the metadata database.
#[derive(Debug, Clone)]
pub struct AuthSessionRepository {
    db: DatabaseConnection,
}

impl AuthSessionRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Persist a new auth session.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn create_session(
        &self,
        input: CreateAuthSession,
    ) -> Result<AuthSessionSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let model = auth_session::ActiveModel {
            id: Set(new_id("sess")),
            user_id: Set(input.user_id),
            token_hash: Set(input.token_hash),
            device_id: Set(input.device_id),
            device_name: Set(input.device_name),
            expires_at: Set(input.expires_at),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;

        self.get_by_token_hash(&model.token_hash)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(model.id))
    }

    /// Lookup a valid session by token hash. Expired sessions are removed lazily.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<AuthSessionSummary>, sea_orm::DbErr> {
        let Some(session) = auth_session::Entity::find()
            .filter(auth_session::Column::TokenHash.eq(token_hash))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        if is_expired_rfc3339(&session.expires_at) {
            let _ = self.delete_by_token_hash(token_hash).await?;
            return Ok(None);
        }

        let Some(user) = user::Entity::find_by_id(session.user_id.clone())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        Ok(Some(AuthSessionSummary::from_models(session, user)))
    }

    /// List valid sessions owned by a username.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_by_username(
        &self,
        username: &str,
    ) -> Result<Vec<AuthSessionSummary>, sea_orm::DbErr> {
        let Some(user) = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.db)
            .await?
        else {
            return Ok(Vec::new());
        };
        let rows = auth_session::Entity::find()
            .filter(auth_session::Column::UserId.eq(user.id.clone()))
            .all(&self.db)
            .await?;
        let mut sessions = Vec::new();
        for session in rows {
            if is_expired_rfc3339(&session.expires_at) {
                let _ = self.delete_by_token_hash(&session.token_hash).await?;
                continue;
            }
            sessions.push(AuthSessionSummary::from_models(session, user.clone()));
        }
        Ok(sessions)
    }

    /// Physically delete expired sessions.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_expired(&self) -> Result<u64, sea_orm::DbErr> {
        let now = now_rfc3339();
        let result = auth_session::Entity::delete_many()
            .filter(auth_session::Column::ExpiresAt.lte(now))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected)
    }

    /// Extend one valid session's expiration timestamp by token hash.
    ///
    /// Expired rows are removed lazily and are not renewed.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn renew_expires_at(
        &self,
        token_hash: &str,
        expires_at: String,
    ) -> Result<bool, sea_orm::DbErr> {
        let Some(session) = auth_session::Entity::find()
            .filter(auth_session::Column::TokenHash.eq(token_hash))
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };

        if is_expired_rfc3339(&session.expires_at) {
            let _ = self.delete_by_token_hash(token_hash).await?;
            return Ok(false);
        }

        let mut active = session.into_active_model();
        active.expires_at = Set(expires_at);
        active.updated_at = Set(now_rfc3339());
        active.update(&self.db).await?;
        Ok(true)
    }

    /// Delete one session by token hash.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_by_token_hash(&self, token_hash: &str) -> Result<bool, sea_orm::DbErr> {
        let result = auth_session::Entity::delete_many()
            .filter(auth_session::Column::TokenHash.eq(token_hash))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Delete one session id owned by a username.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_by_id_for_username(
        &self,
        session_id: &str,
        username: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let Some(user) = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let result = auth_session::Entity::delete_many()
            .filter(auth_session::Column::Id.eq(session_id))
            .filter(auth_session::Column::UserId.eq(user.id))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Delete all sessions belonging to a user.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_by_user_id(&self, user_id: &str) -> Result<u64, sea_orm::DbErr> {
        let result = auth_session::Entity::delete_many()
            .filter(auth_session::Column::UserId.eq(user_id))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected)
    }

    /// Delete all sessions belonging to a username.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_by_username(&self, username: &str) -> Result<u64, sea_orm::DbErr> {
        let Some(user) = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.db)
            .await?
        else {
            return Ok(0);
        };
        self.delete_by_user_id(&user.id).await
    }
}

impl AuthSessionSummary {
    fn from_models(session: auth_session::Model, user: user::Model) -> Self {
        Self {
            id: session.id,
            user_id: user.id,
            username: user.username,
            role: user.role,
            token_hash: session.token_hash,
            device_id: session.device_id,
            device_name: session.device_name,
            expires_at: session.expires_at,
            created_at: session.created_at,
        }
    }
}

fn is_expired_rfc3339(value: &str) -> bool {
    OffsetDateTime::parse(value, &Rfc3339)
        .map_or(true, |expires_at| expires_at <= OffsetDateTime::now_utc())
}

/// Permission granted to a principal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PermissionSummary {
    /// Resource name, for example `users`.
    pub resource: String,
    /// Action name, for example `manage`.
    pub action: String,
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

    /// List permissions granted to a role name.
    pub async fn permissions_for_role(
        &self,
        role_name: &str,
    ) -> Result<Vec<PermissionSummary>, sea_orm::DbErr> {
        let Some(role_model) = role::Entity::find()
            .filter(role::Column::Name.eq(role_name.to_owned()))
            .one(&self.db)
            .await?
        else {
            return Ok(Vec::new());
        };

        let bindings = role_permission::Entity::find()
            .filter(role_permission::Column::RoleId.eq(role_model.id))
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
        permissions.sort_by(|left, right| {
            left.resource
                .cmp(&right.resource)
                .then(left.action.cmp(&right.action))
        });
        permissions.dedup();
        Ok(permissions)
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
        permissions.sort_by(|left, right| {
            left.resource
                .cmp(&right.resource)
                .then(left.action.cmp(&right.action))
        });
        permissions.dedup();
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
}
