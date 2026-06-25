//! `SeaORM` entity definition for RBAC roles.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Platform role catalog entry.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "roles")]
pub struct Model {
    /// Stable role identifier, for example `role-owner`.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Role key used by users and API clients, for example `owner`.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Whether the role is a protected built-in role.
    pub builtin: bool,
    /// Whether the role can grant permissions to users.
    pub enabled: bool,
    /// Timestamp when the role was created.
    pub created_at: String,
    /// Timestamp when the role was last updated.
    pub updated_at: String,
}

/// Relations are intentionally empty; tikeo uses soft relations only.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
