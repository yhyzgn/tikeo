//! `SeaORM` entity definition for persisted OIDC authorization states.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Persisted OIDC authorization state used for CSRF and one-time callback validation.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "oidc_auth_states")]
pub struct Model {
    /// Unique state row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// SHA-256 hash of the opaque state value.
    #[sea_orm(unique)]
    pub state_hash: String,
    /// Redirect URI associated with this authorization request.
    pub redirect_uri: String,
    /// Expiration timestamp in RFC3339.
    pub expires_at: String,
    /// Consumption timestamp in RFC3339, if the callback already used this state.
    pub consumed_at: Option<String>,
    /// Creation timestamp in RFC3339.
    pub created_at: String,
}

/// Database-level foreign keys are forbidden; OIDC states stand alone.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
