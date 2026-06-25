//! `SeaORM` entity definition for instance event stream records.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "instance_events")]
/// Instance event persistence model.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// Stable row identifier.
    pub id: String,
    /// Related instance identifier.
    pub instance_id: String,
    /// Related instance type.
    pub instance_type: String,
    /// Event type.
    pub event_type: String,
    /// Human-readable message.
    pub message: String,
    /// Serialized event payload.
    pub payload: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// `SeaORM` relation marker for this entity.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
