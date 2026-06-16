use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    sea_query::Expr,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::entities::cluster_shard_ownership;

use super::util::{now_rfc3339, rfc3339_after_seconds};

/// Raft-applied shard ownership update.
#[derive(Debug, Clone)]
pub struct UpsertClusterShardOwnership {
    /// Scheduler shard id.
    pub shard_id: i32,
    /// Owner node id for this shard.
    pub owner_node_id: String,
    /// Monotonic ownership epoch.
    pub epoch: i64,
    /// Raft term that produced this projection.
    pub raft_term: i64,
    /// Optional lease hint for diagnostics.
    pub lease_seconds: Option<i64>,
}

/// Persisted scheduler shard ownership row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClusterShardOwnershipSummary {
    /// Scheduler shard id.
    pub shard_id: i32,
    /// Current owner node id.
    pub owner_node_id: String,
    /// Monotonic ownership epoch.
    pub epoch: i64,
    /// Raft term that produced this projection.
    pub raft_term: i64,
    /// Epoch-scoped fencing token.
    pub fencing_token: String,
    /// Ownership status.
    pub status: String,
    /// Optional lease hint.
    pub lease_expires_at: Option<String>,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Cluster shard ownership diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClusterShardOwnershipSloSummary {
    /// Total projected shards.
    pub total: u64,
    /// Active shard rows.
    pub active: u64,
    /// Highest projected epoch.
    pub max_epoch: i64,
    /// Count of active shards owned by each node.
    pub active_by_owner: std::collections::BTreeMap<String, u64>,
}

/// Repository for Raft-applied scheduler shard ownership projections.
#[derive(Debug, Clone)]
pub struct ClusterShardOwnershipRepository {
    db: DatabaseConnection,
}

impl ClusterShardOwnershipRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Upsert one shard ownership row only when the supplied epoch is newer.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn upsert_newer(
        &self,
        input: UpsertClusterShardOwnership,
    ) -> Result<Option<ClusterShardOwnershipSummary>, sea_orm::DbErr> {
        if input.shard_id < 0 {
            return Err(sea_orm::DbErr::Custom(
                "shard_id must be non-negative".to_owned(),
            ));
        }
        if input.owner_node_id.trim().is_empty() {
            return Err(sea_orm::DbErr::Custom(
                "owner_node_id is required".to_owned(),
            ));
        }
        if input.epoch <= 0 {
            return Err(sea_orm::DbErr::Custom(
                "ownership epoch must be positive".to_owned(),
            ));
        }
        let token = fencing_token(input.epoch, input.shard_id, &input.owner_node_id);
        let now = now_rfc3339();
        let lease_expires_at = input
            .lease_seconds
            .filter(|seconds| *seconds > 0)
            .map(rfc3339_after_seconds);

        if cluster_shard_ownership::Entity::find_by_id(input.shard_id)
            .one(&self.db)
            .await?
            .is_none()
        {
            let model = cluster_shard_ownership::ActiveModel {
                shard_id: Set(input.shard_id),
                owner_node_id: Set(input.owner_node_id),
                epoch: Set(input.epoch),
                raft_term: Set(input.raft_term),
                fencing_token: Set(token),
                status: Set("active".to_owned()),
                lease_expires_at: Set(lease_expires_at),
                updated_at: Set(now),
            }
            .insert(&self.db)
            .await?;
            return Ok(Some(model.into()));
        }

        let result = cluster_shard_ownership::Entity::update_many()
            .col_expr(
                cluster_shard_ownership::Column::OwnerNodeId,
                Expr::value(input.owner_node_id),
            )
            .col_expr(
                cluster_shard_ownership::Column::Epoch,
                Expr::value(input.epoch),
            )
            .col_expr(
                cluster_shard_ownership::Column::RaftTerm,
                Expr::value(input.raft_term),
            )
            .col_expr(
                cluster_shard_ownership::Column::FencingToken,
                Expr::value(token),
            )
            .col_expr(
                cluster_shard_ownership::Column::Status,
                Expr::value("active"),
            )
            .col_expr(
                cluster_shard_ownership::Column::LeaseExpiresAt,
                Expr::value(lease_expires_at),
            )
            .col_expr(cluster_shard_ownership::Column::UpdatedAt, Expr::value(now))
            .filter(cluster_shard_ownership::Column::ShardId.eq(input.shard_id))
            .filter(cluster_shard_ownership::Column::Epoch.lt(input.epoch))
            .exec(&self.db)
            .await?;
        if result.rows_affected == 0 {
            return Ok(None);
        }
        self.get(input.shard_id).await
    }

    /// Load one shard ownership row.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get(
        &self,
        shard_id: i32,
    ) -> Result<Option<ClusterShardOwnershipSummary>, sea_orm::DbErr> {
        cluster_shard_ownership::Entity::find_by_id(shard_id)
            .one(&self.db)
            .await
            .map(|row| row.map(ClusterShardOwnershipSummary::from))
    }

    /// Return whether the presented owner fencing token is current and active.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn accepts_fencing_token(
        &self,
        shard_id: i32,
        owner_node_id: &str,
        epoch: i64,
        fencing_token: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        if fencing_token.trim().is_empty() {
            return Ok(false);
        }
        Ok(cluster_shard_ownership::Entity::find()
            .filter(cluster_shard_ownership::Column::ShardId.eq(shard_id))
            .filter(cluster_shard_ownership::Column::OwnerNodeId.eq(owner_node_id.to_owned()))
            .filter(cluster_shard_ownership::Column::Epoch.eq(epoch))
            .filter(cluster_shard_ownership::Column::FencingToken.eq(fencing_token.to_owned()))
            .filter(cluster_shard_ownership::Column::Status.eq("active"))
            .one(&self.db)
            .await?
            .is_some())
    }

    /// List ownership rows ordered by shard id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list(&self) -> Result<Vec<ClusterShardOwnershipSummary>, sea_orm::DbErr> {
        cluster_shard_ownership::Entity::find()
            .order_by_asc(cluster_shard_ownership::Column::ShardId)
            .all(&self.db)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(ClusterShardOwnershipSummary::from)
                    .collect()
            })
    }

    /// Summarize ownership health for diagnostics.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn summary(&self) -> Result<ClusterShardOwnershipSloSummary, sea_orm::DbErr> {
        let mut summary = ClusterShardOwnershipSloSummary::default();
        for row in self.list().await? {
            summary.total = summary.total.saturating_add(1);
            summary.max_epoch = summary.max_epoch.max(row.epoch);
            if row.status == "active" {
                summary.active = summary.active.saturating_add(1);
                *summary
                    .active_by_owner
                    .entry(row.owner_node_id)
                    .or_insert(0) += 1;
            }
        }
        Ok(summary)
    }
}

fn fencing_token(epoch: i64, shard_id: i32, owner_node_id: &str) -> String {
    format!("raft-shard:epoch:{epoch}:shard:{shard_id}:node:{owner_node_id}")
}

impl From<cluster_shard_ownership::Model> for ClusterShardOwnershipSummary {
    fn from(value: cluster_shard_ownership::Model) -> Self {
        Self {
            shard_id: value.shard_id,
            owner_node_id: value.owner_node_id,
            epoch: value.epoch,
            raft_term: value.raft_term,
            fencing_token: value.fencing_token,
            status: value.status,
            lease_expires_at: value.lease_expires_at,
            updated_at: value.updated_at,
        }
    }
}
