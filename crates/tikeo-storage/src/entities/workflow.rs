//! `SeaORM` entity definition for workflow definitions.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Workflow definition row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflows")]
/// Workflow definition persistence model.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// Stable row identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Serialized workflow definition.
    pub definition: String,
    /// Current status.
    pub status: String,
    /// Creator principal identifier.
    pub created_by: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// `SeaORM` relation marker for this entity.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
