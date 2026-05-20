use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};

use crate::entities::{job_instance, job_instance_log};

use super::util::{new_id, now_rfc3339};
/// Job instance log append input.
#[derive(Debug, Clone)]
pub struct AppendJobInstanceLog {
    /// Parent instance identifier.
    pub instance_id: String,
    /// Worker identifier.
    pub worker_id: String,
    /// Log level.
    pub level: String,
    /// Log message.
    pub message: String,
    /// Worker-local monotonic sequence.
    pub sequence: i64,
}

/// Job instance log summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInstanceLogSummary {
    /// Log identifier.
    pub id: String,
    /// Parent instance identifier.
    pub instance_id: String,
    /// Worker identifier.
    pub worker_id: String,
    /// Log level.
    pub level: String,
    /// Log message.
    pub message: String,
    /// Worker-local monotonic sequence.
    pub sequence: i64,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
}

/// Job instance log repository.
#[derive(Debug, Clone)]
pub struct JobInstanceLogRepository {
    db: DatabaseConnection,
}

impl JobInstanceLogRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Append a log row if the parent instance exists.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn append(
        &self,
        input: AppendJobInstanceLog,
    ) -> Result<Option<JobInstanceLogSummary>, sea_orm::DbErr> {
        if job_instance::Entity::find_by_id(input.instance_id.clone())
            .one(&self.db)
            .await?
            .is_none()
        {
            return Ok(None);
        }

        let model = job_instance_log::ActiveModel {
            id: Set(new_id("log")),
            instance_id: Set(input.instance_id),
            worker_id: Set(input.worker_id),
            level: Set(input.level),
            message: Set(input.message),
            sequence: Set(input.sequence),
            created_at: Set(now_rfc3339()),
        }
        .insert(&self.db)
        .await?;

        Ok(Some(JobInstanceLogSummary::from(model)))
    }

    /// List logs for an instance in sequence order.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_by_instance(
        &self,
        instance_id: &str,
    ) -> Result<Vec<JobInstanceLogSummary>, sea_orm::DbErr> {
        let rows = job_instance_log::Entity::find()
            .filter(job_instance_log::Column::InstanceId.eq(instance_id))
            .order_by_asc(job_instance_log::Column::Sequence)
            .order_by_asc(job_instance_log::Column::CreatedAt)
            .all(&self.db)
            .await?;

        Ok(rows.into_iter().map(JobInstanceLogSummary::from).collect())
    }
}

impl From<job_instance_log::Model> for JobInstanceLogSummary {
    fn from(value: job_instance_log::Model) -> Self {
        Self {
            id: value.id,
            instance_id: value.instance_id,
            worker_id: value.worker_id,
            level: value.level,
            message: value.message,
            sequence: value.sequence,
            created_at: value.created_at,
        }
    }
}
