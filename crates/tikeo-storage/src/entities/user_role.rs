//! `SeaORM` entity definition for user-role bindings.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Soft relation between a user and a role.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_roles")]
pub struct Model {
    /// Binding identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Related user id. No database foreign key is created.
    pub user_id: String,
    /// Related role id. No database foreign key is created.
    pub role_id: String,
    /// Timestamp when the binding was created.
    pub created_at: String,
}

/// Relations are intentionally empty; tikeo uses soft relations only.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
