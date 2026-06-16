//! Worker dispatch outbox entity.

use sea_orm::entity::prelude::*;

/// Durable dispatch intent handed from a scheduler owner to a Worker gateway.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "worker_dispatch_outbox")]
pub struct Model {
    /// Outbox row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Target job instance.
    pub instance_id: String,
    /// Target attempt.
    pub attempt_id: String,
    /// Current Worker session id.
    pub worker_id: String,
    /// Stable logical Worker id for reroute after reconnect.
    pub logical_instance_id: String,
    /// Server node that should deliver this row.
    pub gateway_node_id: String,
    /// Worker session generation observed when the row was created/rerouted.
    pub gateway_generation: i64,
    /// Persisted assignment token carried by `DispatchTask` and Worker progress.
    pub assignment_token: String,
    /// Serialized `DispatchTask` payload.
    pub dispatch_payload: String,
    /// Scheduler shard id.
    pub shard_id: i64,
    /// Scheduler owner node that created this intent.
    pub owner_node_id: String,
    /// Scheduler owner epoch.
    pub owner_epoch: i64,
    /// Scheduler owner fencing token.
    pub owner_fencing_token: String,
    /// Outbox lifecycle status.
    pub status: String,
    /// Delivery attempts made by gateways.
    pub delivery_attempts: i32,
    /// Earliest next delivery time.
    pub next_delivery_at: String,
    /// Deadline for Worker ack/progress after stream send.
    pub visibility_deadline: Option<String>,
    /// Last delivery/reroute error.
    pub last_error: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Database-level foreign keys are intentionally avoided; ids are soft links.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
