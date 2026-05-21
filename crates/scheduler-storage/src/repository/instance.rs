use scheduler_core::{ExecutionMode, InstanceStatus, TriggerType};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait, sea_query::Expr,
};

use crate::entities::{dispatch_queue, job, job_instance};

use super::util::{new_id, now_rfc3339};
/// Minimal job instance creation input.
#[derive(Debug, Clone)]
pub struct CreateJobInstance {
    /// Parent job identifier.
    pub job_id: String,
    /// Trigger source for this instance.
    pub trigger_type: TriggerType,
    /// Execution fan-out mode.
    pub execution_mode: ExecutionMode,
}

/// Job instance summary returned to management API callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobInstanceSummary {
    /// Instance identifier.
    pub id: String,
    /// Parent job identifier.
    pub job_id: String,
    /// Current instance status.
    pub status: InstanceStatus,
    /// Trigger source.
    pub trigger_type: TriggerType,
    /// Execution fan-out mode.
    pub execution_mode: ExecutionMode,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Job instance repository.
#[derive(Debug, Clone)]
pub struct JobInstanceRepository {
    db: DatabaseConnection,
}

impl JobInstanceRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a pending job instance if the parent job exists.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn create_pending(
        &self,
        input: CreateJobInstance,
    ) -> Result<Option<JobInstanceSummary>, sea_orm::DbErr> {
        let Some(parent) = job::Entity::find_by_id(input.job_id.clone())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        let now = now_rfc3339();
        let txn = self.db.begin().await?;
        let model = job_instance::ActiveModel {
            id: Set(new_id("inst")),
            job_id: Set(parent.id),
            status: Set(InstanceStatus::Pending.to_string()),
            trigger_type: Set(input.trigger_type.to_string()),
            execution_mode: Set(input.execution_mode.to_string()),
            created_at: Set(now.clone()),
            updated_at: Set(now.clone()),
        }
        .insert(&txn)
        .await?;
        if input.execution_mode == ExecutionMode::Single {
            dispatch_queue::ActiveModel {
                id: Set(new_id("dq")),
                job_instance_id: Set(Some(model.id.clone())),
                workflow_node_instance_id: Set(None),
                priority: Set(0),
                run_after: Set(now.clone()),
                status: Set("pending".to_owned()),
                attempt: Set(0),
                lease_owner: Set(None),
                lease_until: Set(None),
                fencing_token: Set(None),
                worker_selector: Set(None),
                created_at: Set(now.clone()),
                updated_at: Set(now),
            }
            .insert(&txn)
            .await?;
        }
        txn.commit().await?;

        Ok(Some(JobInstanceSummary::from(model)))
    }

    /// List instances for a job ordered by creation timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_by_job(
        &self,
        job_id: &str,
    ) -> Result<Vec<JobInstanceSummary>, sea_orm::DbErr> {
        let rows = job_instance::Entity::find()
            .filter(job_instance::Column::JobId.eq(job_id))
            .order_by_asc(job_instance::Column::CreatedAt)
            .all(&self.db)
            .await?;

        Ok(rows.into_iter().map(JobInstanceSummary::from).collect())
    }

    /// Get one instance by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get(
        &self,
        instance_id: &str,
    ) -> Result<Option<JobInstanceSummary>, sea_orm::DbErr> {
        job_instance::Entity::find_by_id(instance_id.to_owned())
            .one(&self.db)
            .await
            .map(|model| model.map(JobInstanceSummary::from))
    }

    /// List pending single-mode instances in creation order.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_pending_single(
        &self,
        limit: u64,
    ) -> Result<Vec<JobInstanceSummary>, sea_orm::DbErr> {
        let rows = job_instance::Entity::find()
            .filter(job_instance::Column::Status.eq(InstanceStatus::Pending.to_string()))
            .filter(job_instance::Column::ExecutionMode.eq(ExecutionMode::Single.to_string()))
            .order_by_asc(job_instance::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(rows.into_iter().map(JobInstanceSummary::from).collect())
    }

    /// Atomically mark a pending instance as dispatching before a worker send attempt.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn claim_pending_for_dispatch(
        &self,
        instance_id: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let result = job_instance::Entity::update_many()
            .col_expr(
                job_instance::Column::Status,
                Expr::value(InstanceStatus::Dispatching.to_string()),
            )
            .col_expr(job_instance::Column::UpdatedAt, Expr::value(now_rfc3339()))
            .filter(job_instance::Column::Id.eq(instance_id.to_owned()))
            .filter(job_instance::Column::Status.eq(InstanceStatus::Pending.to_string()))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Update one instance status.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn update_status(
        &self,
        instance_id: &str,
        status: InstanceStatus,
    ) -> Result<Option<JobInstanceSummary>, sea_orm::DbErr> {
        let Some(model) = job_instance::Entity::find_by_id(instance_id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        let mut active: job_instance::ActiveModel = model.into();
        active.status = Set(status.to_string());
        active.updated_at = Set(now_rfc3339());
        active
            .update(&self.db)
            .await
            .map(|model| Some(JobInstanceSummary::from(model)))
    }
}

impl From<job_instance::Model> for JobInstanceSummary {
    fn from(value: job_instance::Model) -> Self {
        Self {
            id: value.id,
            job_id: value.job_id,
            status: value.status.parse().unwrap_or(InstanceStatus::Failed),
            trigger_type: value.trigger_type.parse().unwrap_or(TriggerType::Api),
            execution_mode: value
                .execution_mode
                .parse()
                .unwrap_or(ExecutionMode::Single),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
