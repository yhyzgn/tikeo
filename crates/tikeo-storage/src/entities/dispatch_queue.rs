#![allow(missing_docs)]

//! `SeaORM` entity definition for persistent dispatch queue rows.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dispatch_queue")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub job_instance_id: Option<String>,
    pub workflow_node_instance_id: Option<String>,
    pub priority: i32,
    pub run_after: String,
    pub status: String,
    pub attempt: i32,
    pub lease_owner: Option<String>,
    pub lease_until: Option<String>,
    pub fencing_token: Option<String>,
    pub worker_selector: Option<String>,
    pub namespace: Option<String>,
    pub app: Option<String>,
    pub worker_pool: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
