//! `SeaORM` entity definition for persisted auth sessions.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Persisted opaque-token auth session.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "auth_sessions")]
pub struct Model {
    /// Unique session identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Related user id, soft-linked to `users.id`.
    pub user_id: String,
    /// SHA-256 hash of the opaque access token.
    #[sea_orm(unique)]
    /// Token hash value.
    pub token_hash: String,
    /// Optional device identifier.
    pub device_id: Option<String>,
    /// Optional human-readable device name.
    pub device_name: Option<String>,
    /// Expiration timestamp in RFC3339.
    pub expires_at: String,
    /// Creation timestamp in RFC3339.
    pub created_at: String,
    /// Last update timestamp in RFC3339.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
