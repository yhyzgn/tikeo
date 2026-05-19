//! Repository APIs over scheduler metadata tables.

use scheduler_core::{InstanceStatus, TriggerType};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::entities::{app, job, job_instance, namespace};

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
        let model = job_instance::ActiveModel {
            id: Set(new_id("inst")),
            job_id: Set(parent.id),
            status: Set(InstanceStatus::Pending.to_string()),
            trigger_type: Set(input.trigger_type.to_string()),
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

    /// List pending instances in creation order.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_pending(
        &self,
        limit: u64,
    ) -> Result<Vec<JobInstanceSummary>, sea_orm::DbErr> {
        let rows = job_instance::Entity::find()
            .filter(job_instance::Column::Status.eq(InstanceStatus::Pending.to_string()))
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

    use scheduler_core::{InstanceStatus, TriggerType};

    use crate::{
        migration::Migrator,
        repository::{CreateJob, CreateJobInstance},
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

        assert_eq!(created.name, "nightly");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].app, "billing");
        assert!(jobs[0].enabled);
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
            .list_pending(10)
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
}
