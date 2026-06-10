//! Normalized outbound notification message entity.

use sea_orm::entity::prelude::*;

/// Message produced from a source event before provider delivery.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "notification_messages")]
pub struct Model {
    /// Stable message identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Source type such as `alert_event`, `job_instance`, or `workflow_node_instance`.
    pub source_type: String,
    /// Source soft-link id.
    pub source_id: String,
    /// Policy that produced this message.
    pub policy_id: String,
    /// Stable event type, e.g. `job_instance.failed`.
    pub event_type: String,
    /// Resource type shown to operators.
    pub resource_type: String,
    /// Resource soft-link id shown to operators.
    pub resource_id: String,
    /// Notification severity.
    pub severity: String,
    /// Rendered subject.
    pub subject: String,
    /// Rendered body.
    pub body: String,
    /// Provider-neutral payload JSON.
    pub payload_json: String,
    /// Idempotency/noise-control key.
    pub dedupe_key: String,
    /// Optional distributed trace id.
    pub trace_id: Option<String>,
    /// `pending`, `delivering`, `delivered`, `partial_failed`, `failed`, `suppressed`, or `dead_letter`.
    pub status: String,
    /// RFC3339 creation timestamp.
    pub created_at: String,
    /// RFC3339 update timestamp.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
