//! Repository APIs over scheduler metadata tables.

use scheduler_core::{ExecutionMode, InstanceStatus, TriggerType};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::entities::{app, job, job_instance, job_instance_attempt, job_instance_log, namespace};

/// Minimal job creation input.
#[derive(Debug, Clone)]
pub struct CreateJob {
    /// Namespace name. Defaults to `default` at HTTP boundary.
    pub namespace: String,
    /// Application name. Defaults to `default` at HTTP boundary.
    pub app: String,
    /// Job display name.
    pub name: String,
    /// Schedule type such as `api`, `cron`, `fixed_rate`.
    pub schedule_type: String,
    /// Optional schedule expression.
    pub schedule_expr: Option<String>,
    /// Whether the job is enabled.
    pub enabled: bool,
}

/// Job summary returned to management API callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSummary {
    /// Job identifier.
    pub id: String,
    /// Namespace name.
    pub namespace: String,
    /// Application name.
    pub app: String,
    /// Job display name.
    pub name: String,
    /// Schedule type.
    pub schedule_type: String,
    /// Optional schedule expression.
    pub schedule_expr: Option<String>,
    /// Enabled flag.
    pub enabled: bool,
}

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
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Last update timestamp in RFC3339 format.
    pub updated_at: String,
}

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

/// DTO for creating a new user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    /// Unique username.
    pub username: String,
    /// BCrypt password hash.
    pub password_hash: String,
    /// System role (e.g. "admin", "operator", "viewer").
    pub role: String,
}

/// DTO for user updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUser {
    /// Password hash to update, if provided.
    pub password_hash: Option<String>,
    /// Role to update, if provided.
    pub role: Option<String>,
}

/// Lightweight platform user summary representation.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserSummary {
    /// Unique user identifier.
    pub id: String,
    /// Unique username.
    pub username: String,
    /// System role.
    pub role: String,
    /// RFC3339 formatted creation timestamp.
    pub created_at: String,
}

/// User repository.
#[derive(Debug, Clone)]
pub struct UserRepository {
    db: DatabaseConnection,
}

impl UserRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a new user.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails or username is unique violation.
    pub async fn create_user(&self, params: CreateUser) -> Result<UserSummary, sea_orm::DbErr> {
        use crate::entities::user;

        let active = user::ActiveModel {
            id: Set(format!("usr-{}", Uuid::now_v7().to_string())),
            username: Set(params.username),
            password_hash: Set(params.password_hash),
            role: Set(params.role),
            created_at: Set(OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap()),
        };

        let inserted = active.insert(&self.db).await?;
        Ok(UserSummary {
            id: inserted.id,
            username: inserted.username,
            role: inserted.role,
            created_at: inserted.created_at,
        })
    }

    /// List all platform users.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_users(&self) -> Result<Vec<UserSummary>, sea_orm::DbErr> {
        use crate::entities::user;

        let rows = user::Entity::find().all(&self.db).await?;
        Ok(rows
            .into_iter()
            .map(|r| UserSummary {
                id: r.id,
                username: r.username,
                role: r.role,
                created_at: r.created_at,
            })
            .collect())
    }

    /// Get user by username.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get_by_username(&self, username: &str) -> Result<Option<crate::entities::user::Model>, sea_orm::DbErr> {
        use crate::entities::user;

        user::Entity::find()
            .filter(user::Column::Username.eq(username.to_owned()))
            .one(&self.db)
            .await
    }

    /// Delete user by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_user(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        use crate::entities::user;

        let res = user::Entity::delete_by_id(id.to_owned()).exec(&self.db).await?;
        Ok(res.rows_affected > 0)
    }

    /// Get user by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get_user(&self, id: &str) -> Result<Option<crate::entities::user::Model>, sea_orm::DbErr> {
        use crate::entities::user;

        user::Entity::find_by_id(id.to_owned()).one(&self.db).await
    }

    /// Update user details.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn update_user(&self, id: &str, params: UpdateUser) -> Result<Option<UserSummary>, sea_orm::DbErr> {
        use crate::entities::user;

        let Some(existing) = user::Entity::find_by_id(id.to_owned()).one(&self.db).await? else {
            return Ok(None);
        };

        let mut active: user::ActiveModel = existing.into();
        if let Some(hash) = params.password_hash {
            active.password_hash = Set(hash);
        }
        if let Some(role) = params.role {
            active.role = Set(role);
        }

        let updated = active.update(&self.db).await?;
        Ok(Some(UserSummary {
            id: updated.id,
            username: updated.username,
            role: updated.role,
            created_at: updated.created_at,
        }))
    }
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
            worker_id: value.worker_id,
            status: value.status.parse().unwrap_or(InstanceStatus::Failed),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
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
        let model = job_instance::ActiveModel {
            id: Set(new_id("inst")),
            job_id: Set(parent.id),
            status: Set(InstanceStatus::Pending.to_string()),
            trigger_type: Set(input.trigger_type.to_string()),
            execution_mode: Set(input.execution_mode.to_string()),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;

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

/// Job repository.
#[derive(Debug, Clone)]
pub struct JobRepository {
    db: DatabaseConnection,
}

impl JobRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// List jobs ordered by creation order.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_jobs(&self) -> Result<Vec<JobSummary>, sea_orm::DbErr> {
        let rows = job::Entity::find()
            .find_also_related(app::Entity)
            .all(&self.db)
            .await?;
        self.hydrate_job_summaries(rows).await
    }

    /// Get one job by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get(&self, job_id: &str) -> Result<Option<JobSummary>, sea_orm::DbErr> {
        let rows = job::Entity::find_by_id(job_id.to_owned())
            .find_also_related(app::Entity)
            .all(&self.db)
            .await?;
        
        let summaries = self.hydrate_job_summaries(rows).await?;
        Ok(summaries.into_iter().next())
    }

    /// List enabled jobs whose schedule type is managed by the scheduler tick loop.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_enabled_scheduled_jobs(&self) -> Result<Vec<JobSummary>, sea_orm::DbErr> {
        let rows = job::Entity::find()
            .filter(job::Column::Enabled.eq(true))
            .filter(job::Column::ScheduleType.is_in(["cron", "fixed_rate"]))
            .find_also_related(app::Entity)
            .all(&self.db)
            .await?;

        self.hydrate_job_summaries(rows).await
    }

    /// Create a job, creating namespace/app metadata if needed.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails or uniqueness constraints reject the job.
    pub async fn create_job(&self, input: CreateJob) -> Result<JobSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let namespace = self.ensure_namespace(&input.namespace, &now).await?;
        let app = self.ensure_app(&namespace.id, &input.app, &now).await?;
        let id = new_id("job");

        let model = job::ActiveModel {
            id: Set(id.clone()),
            namespace_id: Set(namespace.id.clone()),
            app_id: Set(app.id.clone()),
            name: Set(input.name),
            schedule_type: Set(input.schedule_type),
            schedule_expr: Set(input.schedule_expr),
            enabled: Set(input.enabled),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;

        Ok(JobSummary {
            id,
            namespace: namespace.name,
            app: app.name,
            name: model.name,
            schedule_type: model.schedule_type,
            schedule_expr: model.schedule_expr,
            enabled: model.enabled,
        })
    }

    async fn hydrate_job_summaries(
        &self,
        rows: Vec<(job::Model, Option<app::Model>)>,
    ) -> Result<Vec<JobSummary>, sea_orm::DbErr> {
        let mut jobs = Vec::with_capacity(rows.len());

        for (job, app) in rows {
            let app = app.unwrap_or_else(|| app::Model {
                id: job.app_id.clone(),
                namespace_id: job.namespace_id.clone(),
                name: "unknown".to_owned(),
                created_at: String::new(),
                updated_at: String::new(),
            });
            let ns = namespace::Entity::find_by_id(job.namespace_id.clone())
                .one(&self.db)
                .await?;
            jobs.push(JobSummary {
                id: job.id,
                namespace: ns.map_or_else(|| "unknown".to_owned(), |namespace| namespace.name),
                app: app.name,
                name: job.name,
                schedule_type: job.schedule_type,
                schedule_expr: job.schedule_expr,
                enabled: job.enabled,
            });
        }

        Ok(jobs)
    }

    async fn ensure_namespace(
        &self,
        name: &str,
        now: &str,
    ) -> Result<namespace::Model, sea_orm::DbErr> {
        if let Some(model) = namespace::Entity::find()
            .filter(namespace::Column::Name.eq(name))
            .one(&self.db)
            .await?
        {
            return Ok(model);
        }

        namespace::ActiveModel {
            id: Set(new_id("ns")),
            name: Set(name.to_owned()),
            created_at: Set(now.to_owned()),
            updated_at: Set(now.to_owned()),
        }
        .insert(&self.db)
        .await
    }

    async fn ensure_app(
        &self,
        namespace_id: &str,
        name: &str,
        now: &str,
    ) -> Result<app::Model, sea_orm::DbErr> {
        if let Some(model) = app::Entity::find()
            .filter(app::Column::NamespaceId.eq(namespace_id))
            .filter(app::Column::Name.eq(name))
            .one(&self.db)
            .await?
        {
            return Ok(model);
        }

        app::ActiveModel {
            id: Set(new_id("app")),
            namespace_id: Set(namespace_id.to_owned()),
            name: Set(name.to_owned()),
            created_at: Set(now.to_owned()),
            updated_at: Set(now.to_owned()),
        }
        .insert(&self.db)
        .await
    }
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::now_v7().simple())
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database, Statement};
    use sea_orm_migration::MigratorTrait;

    use scheduler_core::{ExecutionMode, InstanceStatus, TriggerType};

    use crate::{
        migration::Migrator,
        repository::{AppendJobInstanceLog, CreateJob, CreateJobInstance},
    };

    use super::JobRepository;

    #[tokio::test]
    async fn migration_creates_metadata_tables() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));

        let result = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT name FROM sqlite_master WHERE type='table' AND name='jobs'".to_owned(),
            ))
            .await
            .unwrap_or_else(|error| panic!("sqlite_master query should run: {error}"));

        assert!(result.is_some());
    }

    #[tokio::test]
    async fn repository_creates_and_lists_jobs() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));
        let repository = JobRepository::new(db);

        let created = repository
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "nightly".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));

        let jobs = repository
            .list_jobs()
            .await
            .unwrap_or_else(|error| panic!("jobs should list: {error}"));
        let scheduled = repository
            .list_enabled_scheduled_jobs()
            .await
            .unwrap_or_else(|error| panic!("scheduled jobs should list: {error}"));

        assert_eq!(created.name, "nightly");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].app, "billing");
        assert!(jobs[0].enabled);
        assert!(scheduled.is_empty());
    }

    #[tokio::test]
    async fn repository_creates_and_lists_job_instances() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = super::JobInstanceRepository::new(db);

        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));

        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));

        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));

        assert_eq!(instance.status, InstanceStatus::Pending);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].trigger_type, TriggerType::Api);

        let pending = instances
            .list_pending_single(10)
            .await
            .unwrap_or_else(|error| panic!("pending instances should list: {error}"));
        assert_eq!(pending.len(), 1);

        let updated = instances
            .update_status(&instance.id, InstanceStatus::Running)
            .await
            .unwrap_or_else(|error| panic!("instance status should update: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Running);
    }

    #[tokio::test]
    async fn repository_appends_and_lists_job_instance_logs() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = super::JobInstanceRepository::new(db.clone());
        let logs = super::JobInstanceLogRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));

        logs.append(AppendJobInstanceLog {
            instance_id: instance.id.clone(),
            worker_id: "worker-1".to_owned(),
            level: "info".to_owned(),
            message: "hello".to_owned(),
            sequence: 1,
        })
        .await
        .unwrap_or_else(|error| panic!("log should append: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));

        let listed = logs
            .list_by_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("logs should list: {error}"));
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].message, "hello");
    }

    #[tokio::test]
    async fn user_repository_crud_operations() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));
        
        let users = super::UserRepository::new(db);

        // Seeding checked
        let admin = users
            .get_by_username("scheduler_init")
            .await
            .unwrap_or_else(|error| panic!("should load admin user: {error}"));
        assert!(admin.is_some());
        assert_eq!(admin.unwrap().role, "admin");

        // Create user
        let user = users
            .create_user(super::CreateUser {
                username: "operator-1".to_owned(),
                password_hash: "$2b$10$operatorhash".to_owned(),
                role: "operator".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("should create user: {error}"));
        assert_eq!(user.username, "operator-1");
        assert_eq!(user.role, "operator");

        // List users
        let listed = users
            .list_users()
            .await
            .unwrap_or_else(|error| panic!("should list users: {error}"));
        assert_eq!(listed.len(), 2); // admin + operator-1

        // Update user
        let updated = users
            .update_user(
                &user.id,
                super::UpdateUser {
                    password_hash: None,
                    role: Some("viewer".to_owned()),
                },
            )
            .await
            .unwrap_or_else(|error| panic!("should update user: {error}"))
            .unwrap_or_else(|| panic!("user should exist"));
        assert_eq!(updated.role, "viewer");

        // Delete user
        let deleted = users
            .delete_user(&user.id)
            .await
            .unwrap_or_else(|error| panic!("should delete user: {error}"));
        assert!(deleted);

        // List users again
        let listed_again = users
            .list_users()
            .await
            .unwrap_or_else(|error| panic!("should list users: {error}"));
        assert_eq!(listed_again.len(), 1); // just admin
    }
}
