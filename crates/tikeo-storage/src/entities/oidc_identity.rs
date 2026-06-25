//! `SeaORM` entity definition for mapped OIDC external identities.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Soft-link mapping from an external OIDC subject to a local tikeo user and optional scopes.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "oidc_identities")]
pub struct Model {
    /// Unique mapping row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// External issuer URL.
    pub issuer: String,
    /// External subject (`sub`) from `UserInfo`.
    pub subject: String,
    /// Local tikeo username. This is a soft link; database foreign keys are forbidden.
    pub username: String,
    /// Optional namespace binding applied to issued local sessions.
    pub namespace: Option<String>,
    /// Optional app binding applied to issued local sessions.
    pub app: Option<String>,
    /// Optional worker-pool binding applied to issued local sessions.
    pub worker_pool: Option<String>,
    /// Creation timestamp in RFC3339.
    pub created_at: String,
    /// Last update timestamp in RFC3339.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; OIDC identities use soft links only.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
