//! Job entity.

use sea_orm::entity::prelude::*;

/// Job row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "jobs")]
pub struct Model {
    /// Job identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Owning namespace identifier, soft-linked to `namespaces.id`.
    pub namespace_id: String,
    /// Owning application identifier, soft-linked to `apps.id`.
    pub app_id: String,
    /// Display name unique within app.
    pub name: String,
    /// Schedule type, for example `api`, `cron`, `fixed_rate`.
    pub schedule_type: String,
    /// Optional schedule expression.
    pub schedule_expr: Option<String>,
    /// Optional worker processor binding.
    pub processor_name: Option<String>,
    /// Enabled flag.
    pub enabled: bool,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
