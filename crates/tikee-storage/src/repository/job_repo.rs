use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entities::{app, job, namespace};

use super::{
    job::{CreateJob, JobSummary, UpdateJob},
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
        let script_id = normalize_processor_name(input.script_id);
        let processor_name = if script_id.is_some() {
            None
        } else {
            normalize_processor_name(input.processor_name)
        };

        let model = job::ActiveModel {
            id: Set(id.clone()),
            namespace_id: Set(namespace.id.clone()),
            app_id: Set(app.id.clone()),
            name: Set(input.name),
            schedule_type: Set(input.schedule_type),
            schedule_expr: Set(input.schedule_expr),
            processor_name: Set(processor_name),
            script_id: Set(script_id),
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
            script_id: model.script_id,
            enabled: model.enabled,
        })
    }

    /// Update a job by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn update_job(
        &self,
        job_id: &str,
        input: UpdateJob,
    ) -> Result<Option<JobSummary>, sea_orm::DbErr> {
        let Some(model) = job::Entity::find_by_id(job_id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let mut active: job::ActiveModel = model.into();
        if let Some(name) = input.name {
            active.name = Set(name);
        }
        if let Some(schedule_type) = input.schedule_type {
            active.schedule_type = Set(schedule_type);
        }
        if let Some(schedule_expr) = input.schedule_expr {
            active.schedule_expr = Set(schedule_expr);
        }
        if let Some(processor_name) = input.processor_name {
            active.processor_name = Set(normalize_processor_name(processor_name));
            if matches!(active.processor_name, sea_orm::ActiveValue::Set(Some(_))) {
                active.script_id = Set(None);
            }
        }
        if let Some(script_id) = input.script_id {
            active.script_id = Set(normalize_processor_name(script_id));
            if matches!(active.script_id, sea_orm::ActiveValue::Set(Some(_))) {
                active.processor_name = Set(None);
            }
        }
        if let Some(enabled) = input.enabled {
            active.enabled = Set(enabled);
        }
        active.updated_at = Set(now_rfc3339());
        let updated = active.update(&self.db).await?;
        Ok(self
            .hydrate_job_summaries(vec![updated])
            .await?
            .into_iter()
            .next())
    }

    /// Delete a job by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_job(&self, job_id: &str) -> Result<bool, sea_orm::DbErr> {
        let result = job::Entity::delete_by_id(job_id.to_owned())
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
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
                script_id: job.script_id,
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
