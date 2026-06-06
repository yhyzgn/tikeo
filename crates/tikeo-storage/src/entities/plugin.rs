//! Plugin registry entity.

use sea_orm::entity::prelude::*;

/// Plugin declaration row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "plugins")]
pub struct Model {
    /// Plugin identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Display name unique within the registry.
    pub name: String,
    /// Plugin category, for example `processor`, `alert_channel`, or `mixed`.
    pub kind: String,
    /// JSON array of custom processor type declarations.
    pub processor_types_json: String,
    /// JSON array of custom alert channel type declarations.
    pub alert_channel_types_json: String,
    /// Enabled flag. Disabled plugins stay visible but are ignored by resolution.
    pub enabled: bool,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
