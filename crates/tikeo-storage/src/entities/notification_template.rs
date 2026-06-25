//! Reusable notification template entity.

use sea_orm::entity::prelude::*;

/// Reusable provider-specific notification template.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "notification_templates")]
pub struct Model {
    /// Stable template identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Operator-facing stable key, for example `billing-slack-failure`.
    pub template_key: String,
    /// Human-readable template name.
    pub name: String,
    /// Optional template description.
    pub description: Option<String>,
    /// Provider this template renders for.
    pub provider: String,
    /// Provider message type.
    pub message_type: String,
    /// Disabled templates cannot be selected by new policies.
    pub enabled: bool,
    /// Provider-specific template body JSON.
    pub body_json: String,
    /// Optional documented/default variables JSON.
    pub variables_json: String,
    /// Actor who created the template.
    pub created_by: Option<String>,
    /// Actor who last updated the template.
    pub updated_by: Option<String>,
    /// RFC3339 creation timestamp.
    pub created_at: String,
    /// RFC3339 update timestamp.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; policies soft-link by template id/key.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
