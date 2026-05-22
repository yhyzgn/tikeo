//! `SeaORM` entity definition for RBAC permissions.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Permission catalog entry expressed as resource/action.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "permissions")]
pub struct Model {
    /// Stable permission identifier, for example `perm-users-read`.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Resource name, for example `users`.
    pub resource: String,
    /// Action name, for example `read` or `manage`.
    pub action: String,
    /// Human-readable description.
    pub description: String,
    /// Timestamp when the permission was created.
    pub created_at: String,
}

/// Relations are intentionally empty; tikee uses soft relations only.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
