use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entities::{app, job, namespace};

use super::{
    job::{CreateJob, JobSummary},
    util::{new_id, now_rfc3339},
};
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
        let rows = job::Entity::find().all(&self.db).await?;
        self.hydrate_job_summaries(rows).await
    }

    /// Get one job by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get(&self, job_id: &str) -> Result<Option<JobSummary>, sea_orm::DbErr> {
        let rows = job::Entity::find_by_id(job_id.to_owned())
            .all(&self.db)
            .await?;

        let summaries = self.hydrate_job_summaries(rows).await?;
        Ok(summaries.into_iter().next())
    }

    /// List enabled jobs whose schedule type is managed by the tikee tick loop.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_enabled_scheduled_jobs(&self) -> Result<Vec<JobSummary>, sea_orm::DbErr> {
        let rows = job::Entity::find()
            .filter(job::Column::Enabled.eq(true))
            .filter(job::Column::ScheduleType.is_in(["cron", "fixed_rate"]))
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
            processor_name: Set(normalize_processor_name(input.processor_name)),
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
            processor_name: model.processor_name,
            enabled: model.enabled,
        })
    }

    async fn hydrate_job_summaries(
        &self,
        rows: Vec<job::Model>,
    ) -> Result<Vec<JobSummary>, sea_orm::DbErr> {
        let mut jobs = Vec::with_capacity(rows.len());

        for job in rows {
            let app = app::Entity::find_by_id(job.app_id.clone())
                .one(&self.db)
                .await?;
            let ns = namespace::Entity::find_by_id(job.namespace_id.clone())
                .one(&self.db)
                .await?;
            jobs.push(JobSummary {
                id: job.id,
                namespace: ns.map_or_else(|| "unknown".to_owned(), |namespace| namespace.name),
                app: app.map_or_else(|| "unknown".to_owned(), |app| app.name),
                name: job.name,
                schedule_type: job.schedule_type,
                schedule_expr: job.schedule_expr,
                processor_name: job.processor_name,
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

fn normalize_processor_name(value: Option<String>) -> Option<String> {
    value.and_then(|name| {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}
