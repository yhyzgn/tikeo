//! `SeaORM` entity definition for app-scoped service accounts.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Long-lived machine identity used by SDK/API keys.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "service_accounts")]
pub struct Model {
    /// Stable service account id.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Human-readable service account name unique within namespace/app.
    pub name: String,
    /// Optional description for operators.
    pub description: Option<String>,
    /// Namespace scope.
    pub namespace: String,
    /// App scope.
    pub app: String,
    /// Optional worker pool binding.
    pub worker_pool: Option<String>,
    /// Service account status: active or disabled.
    pub status: String,
    /// Human admin actor that created it.
    pub created_by: String,
    /// Human admin actor that last updated it.
    pub updated_by: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by scope/name.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
