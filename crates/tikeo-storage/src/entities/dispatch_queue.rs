//! `SeaORM` entity definition for persistent dispatch queue rows.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dispatch_queue")]
/// Dispatch queue persistence model.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// Stable row identifier.
    pub id: String,
    /// Related job instance identifier.
    pub job_instance_id: Option<String>,
    /// Related workflow node instance identifier.
    pub workflow_node_instance_id: Option<String>,
    /// Scheduler shard identifier.
    pub shard_id: Option<i32>,
    /// Shard map version.
    pub shard_map_version: Option<i64>,
    /// Total shard count.
    pub shard_count: Option<i32>,
    /// Shard owner epoch.
    pub owner_epoch: Option<i64>,
    /// Shard owner fencing token.
    pub owner_fencing_token: Option<String>,
    /// Queue priority.
    pub priority: i32,
    /// Earliest dispatch timestamp.
    pub run_after: String,
    /// Current status.
    pub status: String,
    /// Dispatch attempt number.
    pub attempt: i32,
    /// Current lease owner.
    pub lease_owner: Option<String>,
    /// Lease expiration timestamp.
    pub lease_until: Option<String>,
    /// Dispatch fencing token.
    pub fencing_token: Option<String>,
    /// Serialized worker selector.
    pub worker_selector: Option<String>,
    /// Tenant namespace.
    pub namespace: Option<String>,
    /// Tenant application.
    pub app: Option<String>,
    /// Worker pool name.
    pub worker_pool: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// `SeaORM` relation marker for this entity.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
