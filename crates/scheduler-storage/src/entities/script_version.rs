//! Script version history entity.

use sea_orm::entity::prelude::*;

/// Immutable snapshot of a script definition at a point in time.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "script_versions")]
pub struct Model {
    /// Version record identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Soft-linked to `scripts.id`.
    pub script_id: String,
    /// Monotonically increasing version number.
    pub version_number: i64,
    /// Snapshot of script content.
    pub content: String,
    /// Lowercase hex SHA-256 digest of the content snapshot.
    pub content_sha256: String,
    /// Snapshot of language.
    pub language: String,
    /// Snapshot of status.
    pub status: String,
    /// Snapshot of `timeout_seconds`.
    pub timeout_seconds: Option<i64>,
    /// Snapshot of `max_memory_bytes`.
    pub max_memory_bytes: Option<i64>,
    /// Snapshot of `allow_network`.
    pub allow_network: bool,
    /// Snapshot of `allowed_env_vars`.
    pub allowed_env_vars: Option<String>,
    /// User who triggered this version.
    pub created_by: String,
    /// Timestamp in RFC3339 format.
    pub created_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
