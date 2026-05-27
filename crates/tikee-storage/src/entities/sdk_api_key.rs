//! `SeaORM` entity definition for app-scoped SDK management API keys.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Persisted SDK management API key metadata. Plaintext keys are never stored.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sdk_api_keys")]
pub struct Model {
    /// Unique API key identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Human-readable key name.
    pub name: String,
    /// SHA-256 hash of the plaintext API key.
    #[sea_orm(unique)]
    pub key_hash: String,
    /// Redacted display prefix such as `tk-AbCd...`.
    pub key_prefix: String,
    /// Namespace scope.
    pub namespace: String,
    /// App scope.
    pub app: String,
    /// Comma-separated scope allow-list.
    pub scopes: String,
    /// Key status: active or revoked.
    pub status: String,
    /// RFC3339 expiration timestamp, if any.
    pub expires_at: Option<String>,
    /// RFC3339 last-used timestamp, if any.
    pub last_used_at: Option<String>,
    /// Human admin actor that created the key.
    pub created_by: String,
    /// Human admin actor that revoked the key, if revoked.
    pub revoked_by: Option<String>,
    /// Previous key id when created by rotation, if any.
    pub rotated_from: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by ids/names.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
