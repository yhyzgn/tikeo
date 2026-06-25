use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::{job, job_version};

use super::job::JobRetryPolicy;

use super::util::now_rfc3339;

/// Actor used for version snapshots when no authenticated actor is available.
pub const SYSTEM_ACTOR: &str = "system";

/// Immutable job version summary returned by storage and HTTP APIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct JobVersionSummary {
    /// Identifier value.
    pub id: String,
    /// Identifier value.
    pub job_id: String,
    /// Version number value.
    pub version_number: i64,
    /// Name value.
    pub name: String,
    /// Schedule type value.
    pub schedule_type: String,
    /// Schedule expr value.
    pub schedule_expr: Option<String>,
    /// Misfire policy value.
    pub misfire_policy: String,
    /// Timestamp value.
    pub schedule_start_at: Option<String>,
    /// Timestamp value.
    pub schedule_end_at: Option<String>,
    /// Serialized data value.
    pub schedule_calendar_json: Option<String>,
    /// Processor name value.
    pub processor_name: Option<String>,
    /// Processor type value.
    pub processor_type: Option<String>,
    /// Identifier value.
    pub script_id: Option<String>,
    /// Boolean state flag.
    pub enabled: bool,
    /// Retry policy value.
    pub retry_policy: JobRetryPolicy,
    /// Created by value.
    pub created_by: String,
    /// Change reason value.
    pub change_reason: String,
    /// Rolled back from version value.
    pub rolled_back_from_version: Option<i64>,
    /// Timestamp value.
    pub created_at: String,
}

/// Repository for immutable job definition snapshots.
#[derive(Debug, Clone)]
pub struct JobVersionRepository {
    db: DatabaseConnection,
}

impl JobVersionRepository {
    #[must_use]
    /// New.
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create version.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn create_version(
        &self,
        job: &job::Model,
        actor: Option<&str>,
        reason: &str,
        rolled_back_from_version: Option<i64>,
    ) -> Result<JobVersionSummary, sea_orm::DbErr> {
        self.create_version_in(&self.db, job, actor, reason, rolled_back_from_version)
            .await
    }

    /// Create version in.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn create_version_in<C>(
        &self,
        db: &C,
        job: &job::Model,
        actor: Option<&str>,
        reason: &str,
        rolled_back_from_version: Option<i64>,
    ) -> Result<JobVersionSummary, sea_orm::DbErr>
    where
        C: ConnectionTrait,
    {
        let max_version: Option<Option<i64>> = job_version::Entity::find()
            .filter(job_version::Column::JobId.eq(&job.id))
            .select_only()
            .column_as(job_version::Column::VersionNumber.max(), "max_version")
            .into_tuple()
            .one(db)
            .await?;
        let version_number = max_version.flatten().unwrap_or(0) + 1;
        let id = format!("jv_{version_number}_{}", Uuid::new_v4().simple());
        let created_at = now_rfc3339();
        let created_by = actor
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(SYSTEM_ACTOR)
            .to_owned();
        let change_reason = if reason.trim().is_empty() {
            "update"
        } else {
            reason
        }
        .to_owned();

        job_version::Entity::insert(job_version::ActiveModel {
            id: Set(id.clone()),
            job_id: Set(job.id.clone()),
            version_number: Set(version_number),
            name: Set(job.name.clone()),
            schedule_type: Set(job.schedule_type.clone()),
            schedule_expr: Set(job.schedule_expr.clone()),
            misfire_policy: Set(job.misfire_policy.clone()),
            schedule_start_at: Set(job.schedule_start_at.clone()),
            schedule_end_at: Set(job.schedule_end_at.clone()),
            schedule_calendar_json: Set(job.schedule_calendar_json.clone()),
            processor_name: Set(job.processor_name.clone()),
            processor_type: Set(job.processor_type.clone()),
            script_id: Set(job.script_id.clone()),
            enabled: Set(job.enabled),
            retry_policy_json: Set(job.retry_policy_json.clone()),
            created_by: Set(created_by.clone()),
            change_reason: Set(change_reason.clone()),
            rolled_back_from_version: Set(rolled_back_from_version),
            created_at: Set(created_at.clone()),
        })
        .exec(db)
        .await?;

        Ok(JobVersionSummary {
            id,
            job_id: job.id.clone(),
            version_number,
            name: job.name.clone(),
            schedule_type: job.schedule_type.clone(),
            schedule_expr: job.schedule_expr.clone(),
            misfire_policy: job.misfire_policy.clone(),
            schedule_start_at: job.schedule_start_at.clone(),
            schedule_end_at: job.schedule_end_at.clone(),
            schedule_calendar_json: job.schedule_calendar_json.clone(),
            processor_name: job.processor_name.clone(),
            processor_type: job.processor_type.clone(),
            script_id: job.script_id.clone(),
            enabled: job.enabled,
            retry_policy: JobRetryPolicy::from_json(Some(&job.retry_policy_json)),
            created_by,
            change_reason,
            rolled_back_from_version,
            created_at,
        })
    }

    /// List versions.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn list_versions(
        &self,
        job_id: &str,
    ) -> Result<Vec<JobVersionSummary>, sea_orm::DbErr> {
        let rows = job_version::Entity::find()
            .filter(job_version::Column::JobId.eq(job_id))
            .order_by_desc(job_version::Column::VersionNumber)
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(JobVersionSummary::from).collect())
    }

    /// Get version by number.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn get_version_by_number(
        &self,
        job_id: &str,
        version_number: i64,
    ) -> Result<Option<JobVersionSummary>, sea_orm::DbErr> {
        let row = job_version::Entity::find()
            .filter(job_version::Column::JobId.eq(job_id))
            .filter(job_version::Column::VersionNumber.eq(version_number))
            .one(&self.db)
            .await?;
        Ok(row.map(JobVersionSummary::from))
    }

    /// Latest version number.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn latest_version_number(&self, job_id: &str) -> Result<i64, sea_orm::DbErr> {
        latest_version_number_in(&self.db, job_id).await
    }
}

/// Latest version number in.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub async fn latest_version_number_in<C>(db: &C, job_id: &str) -> Result<i64, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let max_version: Option<Option<i64>> = job_version::Entity::find()
        .filter(job_version::Column::JobId.eq(job_id))
        .select_only()
        .column_as(job_version::Column::VersionNumber.max(), "max_version")
        .into_tuple()
        .one(db)
        .await?;
    Ok(max_version.flatten().unwrap_or(0))
}

impl From<job_version::Model> for JobVersionSummary {
    fn from(value: job_version::Model) -> Self {
        Self {
            id: value.id,
            job_id: value.job_id,
            version_number: value.version_number,
            name: value.name,
            schedule_type: value.schedule_type,
            schedule_expr: value.schedule_expr,
            misfire_policy: value.misfire_policy,
            schedule_start_at: value.schedule_start_at,
            schedule_end_at: value.schedule_end_at,
            schedule_calendar_json: value.schedule_calendar_json,
            processor_name: value.processor_name,
            processor_type: value.processor_type,
            script_id: value.script_id,
            enabled: value.enabled,
            retry_policy: JobRetryPolicy::from_json(Some(&value.retry_policy_json)),
            created_by: value.created_by,
            change_reason: value.change_reason,
            rolled_back_from_version: value.rolled_back_from_version,
            created_at: value.created_at,
        }
    }
}
