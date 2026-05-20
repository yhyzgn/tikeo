//! `SeaORM` entity definition for RBAC roles.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Platform role catalog entry.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "roles")]
pub struct Model {
    /// Stable role identifier, for example `role-admin`.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Role key used by users and API clients, for example `admin`.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Timestamp when the role was created.
    pub created_at: String,
}

/// Relations are intentionally empty; scheduler uses soft relations only.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
