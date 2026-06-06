//! `SeaORM` entity definition for role UI action-element permission bindings.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Soft relation between a role and a UI action element key.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "role_ui_action_permissions")]
pub struct Model {
    /// Binding identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Related role id. No database foreign key is created.
    pub role_id: String,
    /// Structured UI action key from the server-owned UI action catalog.
    pub ui_action_key: String,
    /// Timestamp when the binding was created.
    pub created_at: String,
}

/// Relations are intentionally empty; tikeo uses soft relations only.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
