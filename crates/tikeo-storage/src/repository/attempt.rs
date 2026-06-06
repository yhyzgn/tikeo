use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, sea_query::Expr,
};
use tikeo_core::InstanceStatus;

use crate::entities::{job_instance, job_instance_attempt};

use super::{
    instance::JobInstanceResult,
    util::{new_id, now_rfc3339},
};
/// Job instance attempt creation input.
#[derive(Debug, Clone)]
pub struct CreateJobInstanceAttempt {
    /// Parent instance identifier.
    pub instance_id: String,
    /// Target worker identifier.
    pub worker_id: String,
}

/// Job instance attempt summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobInstanceAttemptSummary {
    /// Attempt identifier.
    pub id: String,
    /// Parent instance identifier.
    pub instance_id: String,
    /// Target worker identifier.
    pub worker_id: String,
    /// Current attempt status.
    pub status: InstanceStatus,
    /// Concrete worker result for this attempt.
    pub result: Option<JobInstanceResult>,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Job instance attempt repository.
#[derive(Debug, Clone)]
pub struct JobInstanceAttemptRepository {
    db: DatabaseConnection,
}

impl JobInstanceAttemptRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create pending attempts for selected workers if the parent instance exists.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn create_pending_for_workers(
        &self,
        instance_id: &str,
        worker_ids: &[String],
    ) -> Result<Vec<JobInstanceAttemptSummary>, sea_orm::DbErr> {
        if job_instance::Entity::find_by_id(instance_id.to_owned())
            .one(&self.db)
            .await?
            .is_none()
        {
            return Ok(Vec::new());
        }

        let mut created = Vec::with_capacity(worker_ids.len());
        for worker_id in worker_ids {
            let now = now_rfc3339();
            let model = job_instance_attempt::ActiveModel {
                id: Set(new_id("attempt")),
                instance_id: Set(instance_id.to_owned()),
                worker_id: Set(worker_id.clone()),
                status: Set(InstanceStatus::Pending.to_string()),
                result_success: Set(None),
                result_message: Set(None),
                result_completed_at: Set(None),
                created_at: Set(now.clone()),
                updated_at: Set(now),
            }
            .insert(&self.db)
            .await?;
            created.push(JobInstanceAttemptSummary::from(model));
        }

        Ok(created)
    }

    /// List attempts for one parent instance ordered by creation time.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_by_instance(
        &self,
        instance_id: &str,
    ) -> Result<Vec<JobInstanceAttemptSummary>, sea_orm::DbErr> {
        let rows = job_instance_attempt::Entity::find()
            .filter(job_instance_attempt::Column::InstanceId.eq(instance_id))
            .order_by_asc(job_instance_attempt::Column::CreatedAt)
            .all(&self.db)
            .await?;

        Ok(rows
            .into_iter()
            .map(JobInstanceAttemptSummary::from)
            .collect())
    }

    /// List pending attempts in creation order.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_pending(
        &self,
        limit: u64,
    ) -> Result<Vec<JobInstanceAttemptSummary>, sea_orm::DbErr> {
        let rows = job_instance_attempt::Entity::find()
            .filter(job_instance_attempt::Column::Status.eq(InstanceStatus::Pending.to_string()))
            .order_by_asc(job_instance_attempt::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(rows
            .into_iter()
            .map(JobInstanceAttemptSummary::from)
            .collect())
    }

    /// Atomically update an attempt only when it is still in the expected status.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn update_status_if_current(
        &self,
        instance_id: &str,
        worker_id: &str,
        expected: InstanceStatus,
        status: InstanceStatus,
    ) -> Result<bool, sea_orm::DbErr> {
        let result = job_instance_attempt::Entity::update_many()
            .col_expr(
                job_instance_attempt::Column::Status,
                Expr::value(status.to_string()),
            )
            .col_expr(
                job_instance_attempt::Column::UpdatedAt,
                Expr::value(now_rfc3339()),
            )
            .filter(job_instance_attempt::Column::InstanceId.eq(instance_id.to_owned()))
            .filter(job_instance_attempt::Column::WorkerId.eq(worker_id.to_owned()))
            .filter(job_instance_attempt::Column::Status.eq(expected.to_string()))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Persist a concrete worker result for one broadcast attempt.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn record_result(
        &self,
        instance_id: &str,
        worker_id: &str,
        success: bool,
        message: &str,
    ) -> Result<Option<JobInstanceAttemptSummary>, sea_orm::DbErr> {
        let Some(model) = job_instance_attempt::Entity::find()
            .filter(job_instance_attempt::Column::InstanceId.eq(instance_id))
            .filter(job_instance_attempt::Column::WorkerId.eq(worker_id))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        let completed_at = now_rfc3339();
        let mut active: job_instance_attempt::ActiveModel = model.into();
        active.result_success = Set(Some(success));
        active.result_message = Set(Some(message.to_owned()));
        active.result_completed_at = Set(Some(completed_at));
        active.updated_at = Set(now_rfc3339());
        active
            .update(&self.db)
            .await
            .map(|model| Some(JobInstanceAttemptSummary::from(model)))
    }

    /// Update one attempt status.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn update_status(
        &self,
        instance_id: &str,
        worker_id: &str,
        status: InstanceStatus,
    ) -> Result<Option<JobInstanceAttemptSummary>, sea_orm::DbErr> {
        let Some(model) = job_instance_attempt::Entity::find()
            .filter(job_instance_attempt::Column::InstanceId.eq(instance_id))
            .filter(job_instance_attempt::Column::WorkerId.eq(worker_id))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        let mut active: job_instance_attempt::ActiveModel = model.into();
        active.status = Set(status.to_string());
        active.updated_at = Set(now_rfc3339());
        active
            .update(&self.db)
            .await
            .map(|model| Some(JobInstanceAttemptSummary::from(model)))
    }
}

impl From<job_instance_attempt::Model> for JobInstanceAttemptSummary {
    fn from(value: job_instance_attempt::Model) -> Self {
        Self {
            id: value.id,
            instance_id: value.instance_id,
            worker_id: value.worker_id.clone(),
            status: value.status.parse().unwrap_or(InstanceStatus::Failed),
            result: match (
                value.result_success,
                value.result_message,
                value.result_completed_at,
            ) {
                (Some(success), Some(message), Some(completed_at)) => Some(JobInstanceResult {
                    worker_id: value.worker_id.clone(),
                    success,
                    message,
                    completed_at,
                }),
                _ => None,
            },
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
