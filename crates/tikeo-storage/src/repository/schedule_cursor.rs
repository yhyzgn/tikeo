use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use tikeo_core::{ExecutionMode, InstanceStatus};

use crate::entities::{dispatch_queue, job, job_instance, schedule_cursor};

use super::{
    instance::job_scope,
    instance::{CreateJobInstance, JobInstanceSummary},
    util::{new_id, now_rfc3339},
};

/// Persistent automatic schedule cursor repository.
#[derive(Debug, Clone)]
pub struct ScheduleCursorRepository {
    db: DatabaseConnection,
}

impl ScheduleCursorRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Return the latest claimed fire timestamp for one job.
    pub async fn latest_fire_at(&self, job_id: &str) -> Result<Option<String>, sea_orm::DbErr> {
        let row = schedule_cursor::Entity::find()
            .filter(schedule_cursor::Column::JobId.eq(job_id.to_owned()))
            .order_by_desc(schedule_cursor::Column::FireAt)
            .one(&self.db)
            .await?;
        Ok(row.map(|row| row.fire_at))
    }

    /// Atomically claim one scheduled fire window and create its pending instance.
    ///
    /// Returns `Ok(None)` when either the parent job is missing or another tick loop has
    /// already claimed the same `(job_id, trigger_type, fire_at)` window.
    pub async fn create_pending_once(
        &self,
        input: CreateJobInstance,
        fire_at: String,
    ) -> Result<Option<JobInstanceSummary>, sea_orm::DbErr> {
        let Some(parent) = job::Entity::find_by_id(input.job_id.clone())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let trigger_type = input.trigger_type.to_string();
        if schedule_cursor::Entity::find()
            .filter(schedule_cursor::Column::JobId.eq(parent.id.clone()))
            .filter(schedule_cursor::Column::TriggerType.eq(trigger_type.clone()))
            .filter(schedule_cursor::Column::FireAt.eq(fire_at.clone()))
            .one(&self.db)
            .await?
            .is_some()
        {
            return Ok(None);
        }

        let scope = job_scope(&self.db, &parent).await?;
        let now = now_rfc3339();
        let txn = self.db.begin().await?;
        let model = job_instance::ActiveModel {
            id: Set(new_id("inst")),
            job_id: Set(parent.id.clone()),
            status: Set(InstanceStatus::Pending.to_string()),
            trigger_type: Set(trigger_type.clone()),
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

        schedule_cursor::ActiveModel {
            id: Set(new_id("sched")),
            job_id: Set(parent.id),
            trigger_type: Set(trigger_type),
            fire_at: Set(fire_at),
            instance_id: Set(model.id.clone()),
            created_at: Set(now.clone()),
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
}
