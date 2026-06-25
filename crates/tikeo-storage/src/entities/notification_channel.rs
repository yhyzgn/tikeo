//! Reusable outbound notification channel entity.

use sea_orm::entity::prelude::*;

/// Reusable outbound notification destination.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "notification_channels")]
pub struct Model {
    /// Stable channel identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Visibility scope: `global`, `namespace`, `app`, or `worker_pool`.
    pub scope_type: String,
    /// Optional namespace soft link/name.
    pub namespace: Option<String>,
    /// Optional app soft link/name.
    pub app: Option<String>,
    /// Optional worker pool soft link/name.
    pub worker_pool: Option<String>,
    /// Operator-facing channel name.
    pub name: String,
    /// Provider key such as webhook, slack, feishu, pagerduty, or email.
    pub provider: String,
    /// Disabled channels stay configured but do not deliver.
    pub enabled: bool,
    /// Redacted provider configuration JSON returned by repository APIs.
    pub config_json: String,
    /// Secret references JSON; raw secret values must not be exposed by API handlers.
    pub secret_refs_json: String,
    /// Cached target display value without credentials/tokens.
    pub target_redacted: String,
    /// Optional transport/local-smoke safety overrides.
    pub safety_policy_json: Option<String>,
    /// Actor who created the channel.
    pub created_by: Option<String>,
    /// Actor who last updated the channel.
    pub updated_by: Option<String>,
    /// RFC3339 creation timestamp.
    pub created_at: String,
    /// RFC3339 update timestamp.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
