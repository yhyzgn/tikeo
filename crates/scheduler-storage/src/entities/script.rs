//! Script definition entity.

use sea_orm::entity::prelude::*;

/// Script definition row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "scripts")]
pub struct Model {
    /// Script identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Display name unique within namespace/app scope.
    pub name: String,
    /// Script language, for example `shell`, `python`, `node`, `rhai`, `wasm`.
    pub language: String,
    /// Semantic version string.
    pub version: String,
    /// Script source content.
    pub content: String,
    /// Approval status: `draft`, `approved`, `disabled`.
    pub status: String,
    /// Optional timeout seconds for execution.
    pub timeout_seconds: Option<i64>,
    /// Optional max memory bytes for sandbox.
    pub max_memory_bytes: Option<i64>,
    /// Whether network access is allowed during execution.
    pub allow_network: bool,
    /// Allowed environment variable names as JSON array string.
    pub allowed_env_vars: Option<String>,
    /// Creator user id, soft-linked to `users.id`.
    pub created_by: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
