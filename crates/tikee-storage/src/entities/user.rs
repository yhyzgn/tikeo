//! `SeaORM` entity definition for platform users.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// The database model for a user.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    /// Unique identifier for the user.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Unique username.
    pub username: String,
    /// Contact email address.
    pub email: String,
    /// `BCrypt` password hash stored in the `password` column.
    pub password: String,
    /// System role (e.g. "admin", "operator", "viewer").
    pub role: String,
    /// Whether this account was created by the one-time deployment bootstrap flow.
    pub bootstrap_admin: bool,
    /// Timestamp when the user was created.
    pub created_at: String,
}

/// Relations for the user entity.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
