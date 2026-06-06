//! Worker pool entity.

use sea_orm::entity::prelude::*;

/// Worker pool row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "worker_pools")]
pub struct Model {
    /// Worker pool identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Owning namespace identifier, soft-linked to `namespaces.id`.
    pub namespace_id: String,
    /// Owning application identifier, soft-linked to `apps.id`.
    pub app_id: String,
    /// Worker pool name unique within an app.
    pub name: String,
    /// Maximum queued/running dispatch items allowed for this pool; 0 means unlimited.
    pub max_queue_depth: i32,
    /// Maximum concurrently running dispatch items allowed for this pool; 0 means unlimited.
    pub max_concurrency: i32,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
