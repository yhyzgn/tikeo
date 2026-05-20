use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::util::now_rfc3339;
/// DTO for creating a new user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    /// Unique username.
    pub username: String,
    /// `BCrypt` password hash stored in the `password` column.
    pub password: String,
    /// System role (e.g. "admin", "operator", "viewer").
    pub role: String,
}

/// DTO for user updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUser {
    /// `BCrypt` password hash to update, if provided.
    pub password: Option<String>,
    /// Role to update, if provided.
    pub role: Option<String>,
}

/// Lightweight platform user summary representation.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserSummary {
    /// Unique user identifier.
    pub id: String,
    /// Unique username.
    pub username: String,
    /// System role.
    pub role: String,
    /// RFC3339 formatted creation timestamp.
    pub created_at: String,
}

/// User repository.
#[derive(Debug, Clone)]
pub struct UserRepository {
    db: DatabaseConnection,
}

impl UserRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Clone the underlying database connection for sibling repositories.
    #[must_use]
    pub fn db(&self) -> DatabaseConnection {
        self.db.clone()
    }

    /// Create a new user.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails or username is unique violation.
    pub async fn create_user(&self, params: CreateUser) -> Result<UserSummary, sea_orm::DbErr> {
        use crate::entities::user;

        let active = user::ActiveModel {
            id: Set(format!("usr-{}", Uuid::now_v7())),
            username: Set(params.username),
            password: Set(params.password),
            role: Set(params.role),
            created_at: Set(now_rfc3339()),
        };

        let inserted = active.insert(&self.db).await?;
        Ok(UserSummary {
            id: inserted.id,
            username: inserted.username,
            role: inserted.role,
            created_at: inserted.created_at,
        })
    }

    /// List all platform users.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_users(&self) -> Result<Vec<UserSummary>, sea_orm::DbErr> {
        use crate::entities::user;

        let rows = user::Entity::find().all(&self.db).await?;
        Ok(rows
            .into_iter()
            .map(|r| UserSummary {
                id: r.id,
                username: r.username,
                role: r.role,
                created_at: r.created_at,
            })
            .collect())
    }

    /// Get user by username.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get_by_username(
        &self,
        username: &str,
    ) -> Result<Option<crate::entities::user::Model>, sea_orm::DbErr> {
        use crate::entities::user;

        user::Entity::find()
            .filter(user::Column::Username.eq(username.to_owned()))
            .one(&self.db)
            .await
    }

    /// Delete user by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_user(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        use crate::entities::user;

        let res = user::Entity::delete_by_id(id.to_owned())
            .exec(&self.db)
            .await?;
        Ok(res.rows_affected > 0)
    }

    /// Get user by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get_user(
        &self,
        id: &str,
    ) -> Result<Option<crate::entities::user::Model>, sea_orm::DbErr> {
        use crate::entities::user;

        user::Entity::find_by_id(id.to_owned()).one(&self.db).await
    }

    /// Update user details.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn update_user(
        &self,
        id: &str,
        params: UpdateUser,
    ) -> Result<Option<UserSummary>, sea_orm::DbErr> {
        use crate::entities::user;

        let Some(existing) = user::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        let mut active: user::ActiveModel = existing.into();
        if let Some(hash) = params.password {
            active.password = Set(hash);
        }
        if let Some(role) = params.role {
            active.role = Set(role);
        }

        let updated = active.update(&self.db).await?;
        Ok(Some(UserSummary {
            id: updated.id,
            username: updated.username,
            role: updated.role,
            created_at: updated.created_at,
        }))
    }
}
