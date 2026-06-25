//! Notification policy/subscription entity.

use sea_orm::entity::prelude::*;

/// Event subscription that maps source filters to channels/templates.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "notification_policies")]
pub struct Model {
    /// Stable policy identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Operator-facing policy name.
    pub name: String,
    /// Disabled policies do not create outbound messages.
    pub enabled: bool,
    /// Owner dimension such as `global`, `app`, `job`, `workflow`, or `alert_rule`.
    pub owner_type: String,
    /// Optional owner soft link.
    pub owner_id: Option<String>,
    /// Event family consumed by this policy.
    pub event_family: String,
    /// Structured event filter JSON.
    pub event_filter_json: String,
    /// Ordered channel references JSON.
    pub channel_refs_json: String,
    /// Optional template id soft link.
    pub template_ref: Option<String>,
    /// Notification severity.
    pub severity: String,
    /// Dedupe window in seconds.
    pub dedupe_seconds: i64,
    /// Optional throttling configuration JSON.
    pub throttle_json: Option<String>,
    /// Optional quiet-hours configuration JSON.
    pub quiet_hours_json: Option<String>,
    /// Optional escalation configuration JSON.
    pub escalation_json: Option<String>,
    /// Actor who created the policy.
    pub created_by: Option<String>,
    /// Actor who last updated the policy.
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
