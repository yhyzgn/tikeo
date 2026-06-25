use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use tikeo_core::MisfirePolicy;

use crate::entities::{app, job, namespace};

use super::{
    job::{CreateJob, JobCanaryPolicy, JobRetryPolicy, JobSummary, UpdateJob},
    job_version::{JobVersionRepository, latest_version_number_in},
    util::{new_id, now_rfc3339},
};
/// Job repository.
#[derive(Debug, Clone)]
pub struct JobRepository {
    db: DatabaseConnection,
    versions: JobVersionRepository,
}

impl JobRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    /// New.
    pub fn new(db: DatabaseConnection) -> Self {
        let versions = JobVersionRepository::new(db.clone());
        Self { db, versions }
    }

    #[must_use]
    /// Versions.
    pub const fn versions(&self) -> &JobVersionRepository {
        &self.versions
    }

    #[must_use]
    /// Db.
    pub fn db(&self) -> DatabaseConnection {
        self.db.clone()
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

    /// List enabled jobs whose schedule type is managed by the tikeo tick loop.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_enabled_scheduled_jobs(&self) -> Result<Vec<JobSummary>, sea_orm::DbErr> {
        let rows = job::Entity::find()
            .filter(job::Column::Enabled.eq(true))
            .filter(job::Column::ScheduleType.is_in([
                "cron",
                "fixed_rate",
                "fixed_delay",
                "once",
                "daily_time_interval",
            ]))
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
        let processor_type = if script_id.is_some() {
            None
        } else {
            normalize_processor_name(input.processor_type)
        };
        let processor_name = if script_id.is_some() {
            None
        } else {
            normalize_processor_name(input.processor_name)
        };
        let canary_policy = input.canary_policy.unwrap_or_default().normalized();
        let retry_policy = input.retry_policy.unwrap_or_default().normalized();

        let active = job::ActiveModel {
            id: Set(id.clone()),
            namespace_id: Set(namespace.id.clone()),
            app_id: Set(app.id.clone()),
            name: Set(input.name),
            schedule_type: Set(input.schedule_type),
            schedule_expr: Set(input.schedule_expr),
            misfire_policy: Set(normalize_misfire_policy(Some(input.misfire_policy))),
            schedule_start_at: Set(normalize_processor_name(input.schedule_start_at)),
            schedule_end_at: Set(normalize_processor_name(input.schedule_end_at)),
            schedule_calendar_json: Set(normalize_processor_name(input.schedule_calendar_json)),
            processor_name: Set(processor_name),
            processor_type: Set(processor_type),
            script_id: Set(script_id),
            enabled: Set(input.enabled),
            canary_job_id: Set(normalize_processor_name(input.canary_job_id)),
            canary_percent: Set(input.canary_percent.clamp(0, 100)),
            canary_policy_json: Set(canary_policy.to_json()),
            retry_policy_json: Set(retry_policy.to_json()),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        };
        let txn = self.db.begin().await?;
        let model = active.insert(&txn).await?;
        let version = self
            .versions
            .create_version_in(&txn, &model, input.created_by.as_deref(), "create", None)
            .await?;
        txn.commit().await?;

        Ok(JobSummary {
            version_number: version.version_number,
            id,
            namespace: namespace.name,
            app: app.name,
            name: model.name,
            schedule_type: model.schedule_type,
            schedule_expr: model.schedule_expr,
            misfire_policy: model.misfire_policy,
            schedule_start_at: model.schedule_start_at,
            schedule_end_at: model.schedule_end_at,
            schedule_calendar_json: model.schedule_calendar_json,
            processor_name: model.processor_name,
            processor_type: model.processor_type,
            script_id: model.script_id,
            enabled: model.enabled,
            canary_job_id: model.canary_job_id,
            canary_percent: model.canary_percent,
            canary_policy: JobCanaryPolicy::from_json(Some(&model.canary_policy_json)),
            retry_policy: JobRetryPolicy::from_json(Some(&model.retry_policy_json)),
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
        let before = model.clone();
        let mut active: job::ActiveModel = model.into();
        let target_namespace_name = normalize_processor_name(input.namespace);
        let target_app_name = normalize_processor_name(input.app);
        if target_namespace_name.is_some() || target_app_name.is_some() {
            let now = now_rfc3339();
            let namespace = if let Some(name) = target_namespace_name {
                self.ensure_namespace(&name, &now).await?
            } else {
                namespace::Entity::find_by_id(before.namespace_id.clone())
                    .one(&self.db)
                    .await?
                    .ok_or_else(|| {
                        sea_orm::DbErr::RecordNotFound(format!(
                            "namespace not found: {}",
                            before.namespace_id
                        ))
                    })?
            };
            let app_name = if let Some(name) = target_app_name {
                name
            } else {
                app::Entity::find_by_id(before.app_id.clone())
                    .one(&self.db)
                    .await?
                    .ok_or_else(|| {
                        sea_orm::DbErr::RecordNotFound(format!("app not found: {}", before.app_id))
                    })?
                    .name
            };
            let app = self.ensure_app(&namespace.id, &app_name, &now).await?;
            active.namespace_id = Set(namespace.id);
            active.app_id = Set(app.id);
        }
        if let Some(name) = input.name {
            active.name = Set(name);
        }
        if let Some(schedule_type) = input.schedule_type {
            active.schedule_type = Set(schedule_type);
        }
        if let Some(schedule_expr) = input.schedule_expr {
            active.schedule_expr = Set(schedule_expr);
        }
        if let Some(misfire_policy) = input.misfire_policy {
            active.misfire_policy = Set(normalize_misfire_policy(Some(misfire_policy)));
        }
        if let Some(schedule_start_at) = input.schedule_start_at {
            active.schedule_start_at = Set(normalize_processor_name(schedule_start_at));
        }
        if let Some(schedule_end_at) = input.schedule_end_at {
            active.schedule_end_at = Set(normalize_processor_name(schedule_end_at));
        }
        if let Some(schedule_calendar_json) = input.schedule_calendar_json {
            active.schedule_calendar_json = Set(normalize_processor_name(schedule_calendar_json));
        }
        if let Some(processor_name) = input.processor_name {
            active.processor_name = Set(normalize_processor_name(processor_name));
            if matches!(active.processor_name, sea_orm::ActiveValue::Set(Some(_))) {
                active.script_id = Set(None);
            }
        }
        if let Some(processor_type) = input.processor_type {
            active.processor_type = Set(normalize_processor_name(processor_type));
        }
        if let Some(script_id) = input.script_id {
            active.script_id = Set(normalize_processor_name(script_id));
            if matches!(active.script_id, sea_orm::ActiveValue::Set(Some(_))) {
                active.processor_name = Set(None);
                active.processor_type = Set(None);
            }
        }
        if let Some(enabled) = input.enabled {
            active.enabled = Set(enabled);
        }
        if let Some(canary_job_id) = input.canary_job_id {
            active.canary_job_id = Set(normalize_processor_name(canary_job_id));
        }
        if let Some(canary_percent) = input.canary_percent {
            active.canary_percent = Set(canary_percent.clamp(0, 100));
        }
        if let Some(canary_policy) = input.canary_policy {
            active.canary_policy_json = Set(canary_policy.normalized().to_json());
        }
        if let Some(retry_policy) = input.retry_policy {
            active.retry_policy_json = Set(retry_policy.normalized().to_json());
        }
        if !job_changed(&before, &active) {
            return Ok(self
                .hydrate_job_summaries(vec![before])
                .await?
                .into_iter()
                .next());
        }
        active.updated_at = Set(now_rfc3339());
        let txn = self.db.begin().await?;
        let updated = active.update(&txn).await?;
        self.versions
            .create_version_in(&txn, &updated, input.updated_by.as_deref(), "update", None)
            .await?;
        txn.commit().await?;
        Ok(self
            .hydrate_job_summaries(vec![updated])
            .await?
            .into_iter()
            .next())
    }

    /// Roll back a job to a previous immutable version by creating a new latest snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn rollback_job(
        &self,
        job_id: &str,
        version_number: i64,
        actor: Option<String>,
    ) -> Result<Option<JobSummary>, sea_orm::DbErr> {
        let Some(version) = self
            .versions
            .get_version_by_number(job_id, version_number)
            .await?
        else {
            return Ok(None);
        };
        let Some(existing) = job::Entity::find_by_id(job_id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let mut active: job::ActiveModel = existing.into();
        active.name = Set(version.name);
        active.schedule_type = Set(version.schedule_type);
        active.schedule_expr = Set(version.schedule_expr);
        active.misfire_policy = Set(version.misfire_policy);
        active.schedule_start_at = Set(version.schedule_start_at);
        active.schedule_end_at = Set(version.schedule_end_at);
        active.schedule_calendar_json = Set(version.schedule_calendar_json);
        active.processor_name = Set(version.processor_name);
        active.processor_type = Set(version.processor_type);
        active.script_id = Set(version.script_id);
        active.enabled = Set(version.enabled);
        active.retry_policy_json = Set(version.retry_policy.to_json());
        active.updated_at = Set(now_rfc3339());
        let txn = self.db.begin().await?;
        let updated = active.update(&txn).await?;
        self.versions
            .create_version_in(
                &txn,
                &updated,
                actor.as_deref(),
                "rollback",
                Some(version_number),
            )
            .await?;
        txn.commit().await?;
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
            let version_number = latest_version_number_in(&self.db, &job.id).await?;
            jobs.push(JobSummary {
                version_number,
                id: job.id,
                namespace: ns.map_or_else(|| "unknown".to_owned(), |namespace| namespace.name),
                app: app.map_or_else(|| "unknown".to_owned(), |app| app.name),
                name: job.name,
                schedule_type: job.schedule_type,
                schedule_expr: job.schedule_expr,
                misfire_policy: job.misfire_policy,
                schedule_start_at: job.schedule_start_at,
                schedule_end_at: job.schedule_end_at,
                schedule_calendar_json: job.schedule_calendar_json,
                processor_name: job.processor_name,
                processor_type: job.processor_type,
                script_id: job.script_id,
                enabled: job.enabled,
                canary_job_id: job.canary_job_id,
                canary_percent: job.canary_percent,
                canary_policy: JobCanaryPolicy::from_json(Some(&job.canary_policy_json)),
                retry_policy: JobRetryPolicy::from_json(Some(&job.retry_policy_json)),
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

fn job_changed(before: &job::Model, active: &job::ActiveModel) -> bool {
    active.namespace_id.as_ref() != &before.namespace_id
        || active.app_id.as_ref() != &before.app_id
        || active.name.as_ref() != &before.name
        || active.schedule_type.as_ref() != &before.schedule_type
        || active.schedule_expr.as_ref() != &before.schedule_expr
        || active.misfire_policy.as_ref() != &before.misfire_policy
        || active.schedule_start_at.as_ref() != &before.schedule_start_at
        || active.schedule_end_at.as_ref() != &before.schedule_end_at
        || active.schedule_calendar_json.as_ref() != &before.schedule_calendar_json
        || active.processor_name.as_ref() != &before.processor_name
        || active.processor_type.as_ref() != &before.processor_type
        || active.script_id.as_ref() != &before.script_id
        || active.enabled.as_ref() != &before.enabled
        || active.canary_job_id.as_ref() != &before.canary_job_id
        || active.canary_percent.as_ref() != &before.canary_percent
        || active.canary_policy_json.as_ref() != &before.canary_policy_json
        || active.retry_policy_json.as_ref() != &before.retry_policy_json
}

fn normalize_misfire_policy(value: Option<String>) -> String {
    value
        .and_then(|policy| policy.parse::<MisfirePolicy>().ok())
        .unwrap_or_default()
        .to_string()
}
