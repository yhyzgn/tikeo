//! Worker session entity.

use sea_orm::entity::prelude::*;

/// Ephemeral Worker Tunnel session row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "worker_sessions")]
pub struct Model {
    /// Server-assigned ephemeral worker session id.
    #[sea_orm(primary_key, auto_increment = false)]
    pub worker_id: String,
    /// Soft link to `worker_logical_instances.id`.
    pub logical_instance_id: String,
    /// Server-local connection id for routing diagnostics.
    pub connection_id: String,
    /// Monotonic generation within the logical instance.
    pub generation: i64,
    /// SHA-256 hash of the session fencing token.
    pub fencing_token_hash: String,
    /// Session lifecycle status.
    pub status: String,
    /// Machine-readable status reason.
    pub status_reason: Option<String>,
    /// Human-readable status evidence.
    pub status_evidence: Option<String>,
    /// Lease expiry timestamp.
    pub lease_expires_at: String,
    /// Last heartbeat timestamp.
    pub last_heartbeat_at: String,
    /// Last accepted heartbeat sequence.
    pub last_sequence: i64,
    /// Connection establishment timestamp.
    pub connected_at: String,
    /// Disconnect timestamp, when known.
    pub disconnected_at: Option<String>,
    /// Replacement worker id, when status is `replaced`.
    pub replaced_by_worker_id: Option<String>,
    /// Drain request timestamp, when any.
    pub drain_requested_at: Option<String>,
    /// Legacy free-form capabilities snapshot.
    pub capabilities_json: String,
    /// Structured capabilities snapshot.
    pub structured_capabilities_json: String,
    /// Worker label snapshot.
    pub labels_json: String,
    /// Worker master/election state snapshot.
    pub master_json: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Database-level foreign keys are forbidden; relationships are soft-linked by id fields.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
