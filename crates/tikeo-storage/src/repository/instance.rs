use std::collections::BTreeMap;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait, sea_query::Expr,
};
use tikeo_core::{ExecutionMode, InstanceStatus, TriggerType};

use crate::entities::{app, dispatch_queue, job, job_instance, namespace};

use super::{
    scheduler_shard_policy,
    util::{new_id, now_rfc3339},
};
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

/// Latest concrete execution result for a job instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobInstanceResult {
    /// Worker that reported the result.
    pub worker_id: String,
    /// Whether the execution succeeded.
    pub success: bool,
    /// Worker-reported result message.
    pub message: String,
    /// Result completion timestamp.
    pub completed_at: String,
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
    /// Latest concrete execution result.
    pub result: Option<JobInstanceResult>,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

/// Job instance count summary by status.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobInstanceStatusCounts {
    /// Total instance rows.
    pub total: u64,
    /// Per-status counts keyed by canonical status string.
    pub by_status: BTreeMap<String, u64>,
}

/// Historical duration statistics for one job.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobDurationHistory {
    /// Total inspected instance rows.
    pub inspected_instances: u64,
    /// Terminal succeeded/failed rows used for duration statistics.
    pub completed_instances: u64,
    /// Failed terminal rows in inspected history.
    pub failed_instances: u64,
    /// Average duration over completed rows.
    pub average_duration_seconds: u64,
    /// Median duration over completed rows.
    pub p50_duration_seconds: u64,
    /// P95 duration over completed rows.
    pub p95_duration_seconds: u64,
    /// Maximum duration over completed rows.
    pub max_duration_seconds: u64,
}

/// Job instance repository.
#[derive(Debug, Clone)]
pub struct JobInstanceRepository {
    db: DatabaseConnection,
}

impl JobInstanceRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    /// New.
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

        let scope = job_scope(&self.db, &parent).await?;
        let shard_policy = scheduler_shard_policy();
        let shard_id = scope
            .as_ref()
            .map(|(namespace, app, _)| shard_policy.shard_id_for(namespace, app, &parent.id));
        let now = now_rfc3339();
        let txn = self.db.begin().await?;
        let model = job_instance::ActiveModel {
            id: Set(new_id("inst")),
            job_id: Set(parent.id),
            status: Set(InstanceStatus::Pending.to_string()),
            trigger_type: Set(input.trigger_type.to_string()),
            execution_mode: Set(input.execution_mode.to_string()),
            result_worker_id: Set(None),
            result_success: Set(None),
            result_message: Set(None),
            result_completed_at: Set(None),
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
                shard_id: Set(shard_id),
                shard_map_version: Set(shard_id.map(|_| shard_policy.shard_map_version)),
                shard_count: Set(shard_id.map(|_| shard_policy.shard_count)),
                owner_epoch: Set(None),
                owner_fencing_token: Set(None),
                priority: Set(0),
                run_after: Set(now.clone()),
                status: Set("pending".to_owned()),
                attempt: Set(0),
                lease_owner: Set(None),
                lease_until: Set(None),
                fencing_token: Set(None),
                worker_selector: Set(None),
                namespace: Set(scope.as_ref().map(|scope| scope.0.clone())),
                app: Set(scope.as_ref().map(|scope| scope.1.clone())),
                worker_pool: Set(scope.as_ref().map(|scope| scope.2.clone())),
                created_at: Set(now.clone()),
                updated_at: Set(now),
            }
            .insert(&txn)
            .await?;
        }
        txn.commit().await?;

        Ok(Some(JobInstanceSummary::from(model)))
    }

    /// List instances for a job ordered by newest creation timestamp first.
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
            .order_by_desc(job_instance::Column::CreatedAt)
            .all(&self.db)
            .await?;

        Ok(rows.into_iter().map(JobInstanceSummary::from).collect())
    }

    /// Summarize historical terminal durations for a job.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn duration_history(
        &self,
        job_id: &str,
        limit: u64,
    ) -> Result<JobDurationHistory, sea_orm::DbErr> {
        let rows = job_instance::Entity::find()
            .filter(job_instance::Column::JobId.eq(job_id))
            .order_by_desc(job_instance::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await?;
        Ok(duration_history_from_rows(rows))
    }

    /// Test helper for deterministic duration statistics.
    #[doc(hidden)]
    /// Set timestamps for test.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn set_timestamps_for_test(
        &self,
        instance_id: &str,
        created_at: &str,
        updated_at: &str,
    ) -> Result<(), sea_orm::DbErr> {
        let result = job_instance::Entity::update_many()
            .col_expr(
                job_instance::Column::CreatedAt,
                Expr::value(created_at.to_owned()),
            )
            .col_expr(
                job_instance::Column::UpdatedAt,
                Expr::value(updated_at.to_owned()),
            )
            .filter(job_instance::Column::Id.eq(instance_id.to_owned()))
            .exec(&self.db)
            .await?;
        let _ = result;
        Ok(())
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

    /// Return the latest terminal instance for a job.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn latest_terminal_by_job(
        &self,
        job_id: &str,
    ) -> Result<Option<JobInstanceSummary>, sea_orm::DbErr> {
        let rows = job_instance::Entity::find()
            .filter(job_instance::Column::JobId.eq(job_id))
            .filter(job_instance::Column::Status.is_in([
                InstanceStatus::Succeeded.to_string(),
                InstanceStatus::Failed.to_string(),
                InstanceStatus::PartialFailed.to_string(),
                InstanceStatus::Cancelled.to_string(),
            ]))
            .order_by_desc(job_instance::Column::UpdatedAt)
            .limit(1)
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().next().map(JobInstanceSummary::from))
    }

    /// Count all instances grouped by their current status.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn count_by_status(&self) -> Result<JobInstanceStatusCounts, sea_orm::DbErr> {
        let rows = job_instance::Entity::find().all(&self.db).await?;
        let mut counts = JobInstanceStatusCounts::default();
        for row in rows {
            counts.total = counts.total.saturating_add(1);
            *counts.by_status.entry(row.status).or_insert(0) += 1;
        }
        Ok(counts)
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

    /// Atomically update an instance only when it is still in the expected status.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn update_status_if_current(
        &self,
        instance_id: &str,
        expected: InstanceStatus,
        status: InstanceStatus,
    ) -> Result<bool, sea_orm::DbErr> {
        let result = job_instance::Entity::update_many()
            .col_expr(
                job_instance::Column::Status,
                Expr::value(status.to_string()),
            )
            .col_expr(job_instance::Column::UpdatedAt, Expr::value(now_rfc3339()))
            .filter(job_instance::Column::Id.eq(instance_id.to_owned()))
            .filter(job_instance::Column::Status.eq(expected.to_string()))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Persist the latest concrete execution result without implying terminal status.
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
    ) -> Result<Option<JobInstanceSummary>, sea_orm::DbErr> {
        let Some(model) = job_instance::Entity::find_by_id(instance_id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let completed_at = now_rfc3339();
        let mut active: job_instance::ActiveModel = model.into();
        active.result_worker_id = Set(Some(worker_id.to_owned()));
        active.result_success = Set(Some(success));
        active.result_message = Set(Some(message.to_owned()));
        active.result_completed_at = Set(Some(completed_at));
        active.updated_at = Set(now_rfc3339());
        active
            .update(&self.db)
            .await
            .map(|model| Some(JobInstanceSummary::from(model)))
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

/// Job scope.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub(super) async fn job_scope(
    db: &DatabaseConnection,
    parent: &job::Model,
) -> Result<Option<(String, String, String)>, sea_orm::DbErr> {
    let Some(ns) = namespace::Entity::find_by_id(parent.namespace_id.clone())
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let Some(app) = app::Entity::find_by_id(parent.app_id.clone())
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    Ok(Some((ns.name, app.name, "default".to_owned())))
}

fn duration_history_from_rows(rows: Vec<job_instance::Model>) -> JobDurationHistory {
    let inspected_instances = u64::try_from(rows.len()).unwrap_or(u64::MAX);
    let mut durations = Vec::new();
    let mut failed_instances = 0_u64;
    for row in rows {
        match row.status.as_str() {
            "succeeded" => durations.push(elapsed_seconds(&row.created_at, &row.updated_at)),
            "failed" => {
                failed_instances = failed_instances.saturating_add(1);
                durations.push(elapsed_seconds(&row.created_at, &row.updated_at));
            }
            _ => {}
        }
    }
    durations.sort_unstable();
    let completed_instances = u64::try_from(durations.len()).unwrap_or(u64::MAX);
    let total = durations.iter().copied().sum::<u64>();
    JobDurationHistory {
        inspected_instances,
        completed_instances,
        failed_instances,
        average_duration_seconds: total.checked_div(completed_instances).unwrap_or(0),
        p50_duration_seconds: percentile(&durations, 50),
        p95_duration_seconds: percentile(&durations, 95),
        max_duration_seconds: durations.last().copied().unwrap_or(0),
    }
}

fn percentile(values: &[u64], percentile: u64) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let max_index = values.len().saturating_sub(1);
    let index = (max_index * usize::try_from(percentile).unwrap_or(100)).div_ceil(100);
    values[index.min(max_index)]
}

fn elapsed_seconds(start: &str, end: &str) -> u64 {
    let Ok(start) =
        time::OffsetDateTime::parse(start, &time::format_description::well_known::Rfc3339)
    else {
        return 0;
    };
    let Ok(end) = time::OffsetDateTime::parse(end, &time::format_description::well_known::Rfc3339)
    else {
        return 0;
    };
    u64::try_from((end - start).whole_seconds().max(0)).unwrap_or(0)
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
            result: match (
                value.result_worker_id,
                value.result_success,
                value.result_message,
                value.result_completed_at,
            ) {
                (Some(worker_id), Some(success), Some(message), Some(completed_at)) => {
                    Some(JobInstanceResult {
                        worker_id,
                        success,
                        message,
                        completed_at,
                    })
                }
                _ => None,
            },
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
