use std::collections::BTreeMap;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait, sea_query::Expr,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::entities::{job_instance_attempt, worker_dispatch_outbox};

use super::util::{new_id, now_rfc3339, rfc3339_after_seconds};

/// Input for a durable Worker dispatch outbox row.
#[derive(Debug, Clone)]
pub struct CreateWorkerDispatchOutbox {
    /// Target job instance.
    pub instance_id: String,
    /// Target attempt.
    pub attempt_id: String,
    /// Current Worker session id.
    pub worker_id: String,
    /// Stable logical Worker id.
    pub logical_instance_id: String,
    /// Gateway node that owns the Worker stream.
    pub gateway_node_id: String,
    /// Worker session generation at selection time.
    pub gateway_generation: i64,
    /// Assignment token already persisted on the attempt.
    pub assignment_token: String,
    /// Serialized dispatch payload.
    pub dispatch_payload: String,
    /// Scheduler shard id.
    pub shard_id: i64,
    /// Owner node that created this intent.
    pub owner_node_id: String,
    /// Owner epoch.
    pub owner_epoch: i64,
    /// Owner fencing token.
    pub owner_fencing_token: String,
    /// Optional first delivery time.
    pub next_delivery_at: Option<String>,
}

/// Durable Worker dispatch outbox row summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerDispatchOutboxSummary {
    /// Outbox id.
    pub id: String,
    /// Target job instance.
    pub instance_id: String,
    /// Target attempt.
    pub attempt_id: String,
    /// Current Worker session id.
    pub worker_id: String,
    /// Stable logical Worker id.
    pub logical_instance_id: String,
    /// Gateway node that should deliver the row.
    pub gateway_node_id: String,
    /// Worker session generation.
    pub gateway_generation: i64,
    /// Assignment token.
    pub assignment_token: String,
    /// Serialized dispatch payload.
    pub dispatch_payload: String,
    /// Scheduler shard id.
    pub shard_id: i64,
    /// Owner node id.
    pub owner_node_id: String,
    /// Owner epoch.
    pub owner_epoch: i64,
    /// Owner fencing token.
    pub owner_fencing_token: String,
    /// Outbox status.
    pub status: String,
    /// Delivery attempts.
    pub delivery_attempts: i32,
    /// Next delivery timestamp.
    pub next_delivery_at: String,
    /// Visibility deadline.
    pub visibility_deadline: Option<String>,
    /// Last error.
    pub last_error: Option<String>,
    /// Created timestamp.
    pub created_at: String,
    /// Updated timestamp.
    pub updated_at: String,
}

/// Durable Worker dispatch outbox aggregate summary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerDispatchOutboxSloSummary {
    /// Total outbox rows.
    pub total: u64,
    /// Count by outbox status.
    pub by_status: BTreeMap<String, u64>,
    /// Age in seconds of the oldest queued/reroute-pending row.
    pub oldest_queued_age_seconds: u64,
}

/// Repository for durable Worker dispatch handoff rows.
#[derive(Debug, Clone)]
pub struct WorkerDispatchOutboxRepository {
    db: DatabaseConnection,
}

impl WorkerDispatchOutboxRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Summarize outbox health for metrics and diagnostics.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn summary(&self) -> Result<WorkerDispatchOutboxSloSummary, sea_orm::DbErr> {
        let rows = worker_dispatch_outbox::Entity::find().all(&self.db).await?;
        let now = time::OffsetDateTime::now_utc();
        let mut summary = WorkerDispatchOutboxSloSummary::default();
        for row in rows {
            summary.total = summary.total.saturating_add(1);
            *summary.by_status.entry(row.status.clone()).or_insert(0) += 1;
            if matches!(row.status.as_str(), "queued" | "reroute_pending") {
                let age = time::OffsetDateTime::parse(
                    &row.created_at,
                    &time::format_description::well_known::Rfc3339,
                )
                .ok()
                .and_then(|created| (now - created).whole_seconds().try_into().ok())
                .unwrap_or(0);
                summary.oldest_queued_age_seconds = summary.oldest_queued_age_seconds.max(age);
            }
        }
        Ok(summary)
    }

    /// Create a queued outbox row.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn create(
        &self,
        input: CreateWorkerDispatchOutbox,
    ) -> Result<WorkerDispatchOutboxSummary, sea_orm::DbErr> {
        if input.assignment_token.trim().is_empty() {
            return Err(sea_orm::DbErr::Custom(
                "assignment_token is required for worker dispatch outbox".to_owned(),
            ));
        }
        let now = now_rfc3339();
        let model = worker_dispatch_outbox::ActiveModel {
            id: Set(new_id("outbox")),
            instance_id: Set(input.instance_id),
            attempt_id: Set(input.attempt_id),
            worker_id: Set(input.worker_id),
            logical_instance_id: Set(input.logical_instance_id),
            gateway_node_id: Set(input.gateway_node_id),
            gateway_generation: Set(input.gateway_generation),
            assignment_token: Set(input.assignment_token),
            dispatch_payload: Set(input.dispatch_payload),
            shard_id: Set(input.shard_id),
            owner_node_id: Set(input.owner_node_id),
            owner_epoch: Set(input.owner_epoch),
            owner_fencing_token: Set(input.owner_fencing_token),
            status: Set("queued".to_owned()),
            delivery_attempts: Set(0),
            next_delivery_at: Set(input.next_delivery_at.unwrap_or_else(|| now.clone())),
            visibility_deadline: Set(None),
            last_error: Set(None),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(model.into())
    }

    /// Claim the oldest due row for one gateway.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn claim_next_for_gateway(
        &self,
        gateway_node_id: &str,
        limit_window_seconds: i64,
    ) -> Result<Option<WorkerDispatchOutboxSummary>, sea_orm::DbErr> {
        let now = now_rfc3339();
        let Some(row) = worker_dispatch_outbox::Entity::find()
            .filter(worker_dispatch_outbox::Column::GatewayNodeId.eq(gateway_node_id.to_owned()))
            .filter(worker_dispatch_outbox::Column::Status.is_in(["queued", "reroute_pending"]))
            .filter(worker_dispatch_outbox::Column::NextDeliveryAt.lte(now.clone()))
            .order_by_asc(worker_dispatch_outbox::Column::CreatedAt)
            .limit(1)
            .one(&self.db)
            .await?
        else {
            let _ = limit_window_seconds;
            return Ok(None);
        };
        let result = worker_dispatch_outbox::Entity::update_many()
            .col_expr(
                worker_dispatch_outbox::Column::Status,
                Expr::value("delivering"),
            )
            .col_expr(
                worker_dispatch_outbox::Column::DeliveryAttempts,
                Expr::col(worker_dispatch_outbox::Column::DeliveryAttempts).add(1),
            )
            .col_expr(
                worker_dispatch_outbox::Column::UpdatedAt,
                Expr::value(now_rfc3339()),
            )
            .filter(worker_dispatch_outbox::Column::Id.eq(row.id.clone()))
            .filter(worker_dispatch_outbox::Column::Status.is_in(["queued", "reroute_pending"]))
            .exec(&self.db)
            .await?;
        if result.rows_affected == 0 {
            return Ok(None);
        }
        worker_dispatch_outbox::Entity::find_by_id(row.id)
            .one(&self.db)
            .await
            .map(|row| row.map(WorkerDispatchOutboxSummary::from))
    }

    /// Mark one claimed row as delivered to the local stream.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn mark_delivered(
        &self,
        outbox_id: &str,
        visibility_timeout_seconds: i64,
    ) -> Result<Option<WorkerDispatchOutboxSummary>, sea_orm::DbErr> {
        let deadline = rfc3339_after_seconds(visibility_timeout_seconds.max(1));
        let result = worker_dispatch_outbox::Entity::update_many()
            .col_expr(
                worker_dispatch_outbox::Column::Status,
                Expr::value("delivered"),
            )
            .col_expr(
                worker_dispatch_outbox::Column::VisibilityDeadline,
                Expr::value(Some(deadline)),
            )
            .col_expr(
                worker_dispatch_outbox::Column::UpdatedAt,
                Expr::value(now_rfc3339()),
            )
            .filter(worker_dispatch_outbox::Column::Id.eq(outbox_id.to_owned()))
            .filter(worker_dispatch_outbox::Column::Status.eq("delivering"))
            .exec(&self.db)
            .await?;
        if result.rows_affected == 0 {
            return Ok(None);
        }
        self.get(outbox_id).await
    }

    /// Requeue delivered rows that did not receive Worker ack/progress before visibility timeout.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn requeue_expired_delivered(
        &self,
        retry_after_seconds: i64,
    ) -> Result<u64, sea_orm::DbErr> {
        let now = now_rfc3339();
        let result = worker_dispatch_outbox::Entity::update_many()
            .col_expr(
                worker_dispatch_outbox::Column::Status,
                Expr::value("queued"),
            )
            .col_expr(
                worker_dispatch_outbox::Column::NextDeliveryAt,
                Expr::value(rfc3339_after_seconds(retry_after_seconds.max(0))),
            )
            .col_expr(
                worker_dispatch_outbox::Column::VisibilityDeadline,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                worker_dispatch_outbox::Column::LastError,
                Expr::value(Some(
                    "visibility timeout expired before worker ack".to_owned(),
                )),
            )
            .col_expr(
                worker_dispatch_outbox::Column::UpdatedAt,
                Expr::value(now.clone()),
            )
            .filter(worker_dispatch_outbox::Column::Status.eq("delivered"))
            .filter(worker_dispatch_outbox::Column::VisibilityDeadline.is_not_null())
            .filter(worker_dispatch_outbox::Column::VisibilityDeadline.lte(now))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected)
    }

    /// Reroute a queued or failed delivery row to a newer Worker session/gateway.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn reroute(
        &self,
        outbox_id: &str,
        gateway_node_id: &str,
        worker_id: &str,
        gateway_generation: i64,
    ) -> Result<Option<WorkerDispatchOutboxSummary>, sea_orm::DbErr> {
        let Some(row) = worker_dispatch_outbox::Entity::find_by_id(outbox_id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        if row.status == "completed" {
            return Ok(None);
        }

        let txn = self.db.begin().await?;
        let result = worker_dispatch_outbox::Entity::update_many()
            .col_expr(
                worker_dispatch_outbox::Column::GatewayNodeId,
                Expr::value(gateway_node_id.to_owned()),
            )
            .col_expr(
                worker_dispatch_outbox::Column::WorkerId,
                Expr::value(worker_id.to_owned()),
            )
            .col_expr(
                worker_dispatch_outbox::Column::GatewayGeneration,
                Expr::value(gateway_generation),
            )
            .col_expr(
                worker_dispatch_outbox::Column::Status,
                Expr::value("queued"),
            )
            .col_expr(
                worker_dispatch_outbox::Column::NextDeliveryAt,
                Expr::value(now_rfc3339()),
            )
            .col_expr(
                worker_dispatch_outbox::Column::VisibilityDeadline,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                worker_dispatch_outbox::Column::LastError,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                worker_dispatch_outbox::Column::UpdatedAt,
                Expr::value(now_rfc3339()),
            )
            .filter(worker_dispatch_outbox::Column::Id.eq(outbox_id.to_owned()))
            .filter(worker_dispatch_outbox::Column::Status.ne("completed"))
            .exec(&txn)
            .await?;
        if result.rows_affected == 0 {
            return Ok(None);
        }

        // Rerouting transfers the same durable assignment to a newer session of the
        // same logical Worker. Keep the attempt row aligned with the session that
        // will report logs/results; otherwise assignment-token fencing would reject
        // the rerouted Worker even though the outbox was correctly moved.
        job_instance_attempt::Entity::update_many()
            .col_expr(
                job_instance_attempt::Column::WorkerId,
                Expr::value(worker_id.to_owned()),
            )
            .col_expr(
                job_instance_attempt::Column::UpdatedAt,
                Expr::value(now_rfc3339()),
            )
            .filter(job_instance_attempt::Column::Id.eq(row.attempt_id))
            .filter(job_instance_attempt::Column::InstanceId.eq(row.instance_id))
            .filter(job_instance_attempt::Column::WorkerId.eq(row.worker_id))
            .filter(job_instance_attempt::Column::AssignmentToken.eq(row.assignment_token))
            .exec(&txn)
            .await?;

        let updated = worker_dispatch_outbox::Entity::find_by_id(outbox_id.to_owned())
            .one(&txn)
            .await?
            .map(WorkerDispatchOutboxSummary::from);
        txn.commit().await?;
        Ok(updated)
    }

    /// Mark the matching assignment outbox as acked after the Worker sends progress.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn mark_acked_by_assignment(
        &self,
        instance_id: &str,
        worker_id: &str,
        assignment_token: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        if assignment_token.trim().is_empty() {
            return Ok(false);
        }
        let result = worker_dispatch_outbox::Entity::update_many()
            .col_expr(worker_dispatch_outbox::Column::Status, Expr::value("acked"))
            .col_expr(
                worker_dispatch_outbox::Column::VisibilityDeadline,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                worker_dispatch_outbox::Column::LastError,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                worker_dispatch_outbox::Column::UpdatedAt,
                Expr::value(now_rfc3339()),
            )
            .filter(worker_dispatch_outbox::Column::InstanceId.eq(instance_id.to_owned()))
            .filter(worker_dispatch_outbox::Column::WorkerId.eq(worker_id.to_owned()))
            .filter(worker_dispatch_outbox::Column::AssignmentToken.eq(assignment_token.to_owned()))
            .filter(worker_dispatch_outbox::Column::Status.is_in(["delivered", "acked"]))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Requeue a claimed delivery row after a transport failure.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn mark_delivery_failed(
        &self,
        outbox_id: &str,
        error: &str,
        retry_after_seconds: i64,
    ) -> Result<Option<WorkerDispatchOutboxSummary>, sea_orm::DbErr> {
        let result = worker_dispatch_outbox::Entity::update_many()
            .col_expr(
                worker_dispatch_outbox::Column::Status,
                Expr::value("queued"),
            )
            .col_expr(
                worker_dispatch_outbox::Column::NextDeliveryAt,
                Expr::value(rfc3339_after_seconds(retry_after_seconds.max(0))),
            )
            .col_expr(
                worker_dispatch_outbox::Column::VisibilityDeadline,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                worker_dispatch_outbox::Column::LastError,
                Expr::value(Some(error.to_owned())),
            )
            .col_expr(
                worker_dispatch_outbox::Column::UpdatedAt,
                Expr::value(now_rfc3339()),
            )
            .filter(worker_dispatch_outbox::Column::Id.eq(outbox_id.to_owned()))
            .filter(worker_dispatch_outbox::Column::Status.is_in([
                "delivering",
                "delivered",
                "queued",
            ]))
            .exec(&self.db)
            .await?;
        if result.rows_affected == 0 {
            return Ok(None);
        }
        self.get(outbox_id).await
    }

    /// Mark the matching assignment outbox terminal after a fenced Worker result.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn mark_completed_by_assignment(
        &self,
        instance_id: &str,
        worker_id: &str,
        assignment_token: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        if assignment_token.trim().is_empty() {
            return Ok(false);
        }
        let result = worker_dispatch_outbox::Entity::update_many()
            .col_expr(
                worker_dispatch_outbox::Column::Status,
                Expr::value("completed"),
            )
            .col_expr(
                worker_dispatch_outbox::Column::VisibilityDeadline,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                worker_dispatch_outbox::Column::UpdatedAt,
                Expr::value(now_rfc3339()),
            )
            .filter(worker_dispatch_outbox::Column::InstanceId.eq(instance_id.to_owned()))
            .filter(worker_dispatch_outbox::Column::WorkerId.eq(worker_id.to_owned()))
            .filter(worker_dispatch_outbox::Column::AssignmentToken.eq(assignment_token.to_owned()))
            .filter(worker_dispatch_outbox::Column::Status.ne("completed"))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Load one outbox row.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get(
        &self,
        outbox_id: &str,
    ) -> Result<Option<WorkerDispatchOutboxSummary>, sea_orm::DbErr> {
        worker_dispatch_outbox::Entity::find_by_id(outbox_id.to_owned())
            .one(&self.db)
            .await
            .map(|row| row.map(WorkerDispatchOutboxSummary::from))
    }
}

impl From<worker_dispatch_outbox::Model> for WorkerDispatchOutboxSummary {
    fn from(value: worker_dispatch_outbox::Model) -> Self {
        Self {
            id: value.id,
            instance_id: value.instance_id,
            attempt_id: value.attempt_id,
            worker_id: value.worker_id,
            logical_instance_id: value.logical_instance_id,
            gateway_node_id: value.gateway_node_id,
            gateway_generation: value.gateway_generation,
            assignment_token: value.assignment_token,
            dispatch_payload: value.dispatch_payload,
            shard_id: value.shard_id,
            owner_node_id: value.owner_node_id,
            owner_epoch: value.owner_epoch,
            owner_fencing_token: value.owner_fencing_token,
            status: value.status,
            delivery_attempts: value.delivery_attempts,
            next_delivery_at: value.next_delivery_at,
            visibility_deadline: value.visibility_deadline,
            last_error: value.last_error,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
