//! Repository APIs over scheduler metadata tables.

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::entities::{app, job, namespace};

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

    use crate::{migration::Migrator, repository::CreateJob};

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
}
