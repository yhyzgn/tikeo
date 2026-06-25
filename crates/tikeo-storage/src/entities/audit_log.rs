//! Audit log entity for tracking platform write operations.

use sea_orm::entity::prelude::*;

/// Audit log row recording a platform write operation.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "audit_logs")]
pub struct Model {
    /// Audit log identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    /// Identifier value.
    pub id: String,
    /// Actor who performed the action (username or system identifier).
    pub actor: String,
    /// Action performed, for example `create`, `update`, `delete`, `login`, `logout`.
    pub action: String,
    /// Resource type, for example `job`, `script`, `user`.
    pub resource_type: String,
    /// Resource identifier affected by the action.
    pub resource_id: String,
    /// Optional JSON detail about the action.
    pub detail: Option<String>,
    /// Optional JSON snapshot before the action.
    pub before: Option<String>,
    /// Optional JSON snapshot after the action.
    pub after: Option<String>,
    /// Request trace id for correlating logs across layers.
    pub trace_id: Option<String>,
    /// Audit result status, e.g. `success` or `failed`.
    pub result: String,
    /// Optional failure reason when `result=failed`.
    pub failure_reason: Option<String>,
    /// Client IP address at the time of the action.
    pub ip_address: Option<String>,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
