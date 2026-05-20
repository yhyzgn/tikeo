//! Repository APIs over scheduler metadata tables.
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::missing_const_for_fn
)]

use scheduler_core::{ExecutionMode, InstanceStatus, TriggerType};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::entities::{
    app, auth_session, job, job_instance, job_instance_attempt, job_instance_log, namespace,
    script, script_version, user,
};

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

/// Persisted session creation input.
#[derive(Debug, Clone)]
pub struct CreateAuthSession {
    /// Related user id.
    pub user_id: String,
    /// SHA-256 hash of the opaque access token.
    pub token_hash: String,
    /// Optional device identifier.
    pub device_id: Option<String>,
    /// Optional device display name.
    pub device_name: Option<String>,
    /// RFC3339 expiration timestamp.
    pub expires_at: String,
}

/// Persisted auth session plus principal snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthSessionSummary {
    /// Session id.
    pub id: String,
    /// User id.
    pub user_id: String,
    /// Username.
    pub username: String,
    /// Role.
    pub role: String,
    /// Token hash.
    pub token_hash: String,
    /// Optional device id.
    pub device_id: Option<String>,
    /// Optional device name.
    pub device_name: Option<String>,
    /// Expiration timestamp.
    pub expires_at: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Auth session repository backed by the metadata database.
#[derive(Debug, Clone)]
pub struct AuthSessionRepository {
    db: DatabaseConnection,
}

impl AuthSessionRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Persist a new auth session.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn create_session(
        &self,
        input: CreateAuthSession,
    ) -> Result<AuthSessionSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let model = auth_session::ActiveModel {
            id: Set(new_id("sess")),
            user_id: Set(input.user_id),
            token_hash: Set(input.token_hash),
            device_id: Set(input.device_id),
            device_name: Set(input.device_name),
            expires_at: Set(input.expires_at),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;

        self.get_by_token_hash(&model.token_hash)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(model.id))
    }

    /// Lookup a valid session by token hash. Expired sessions are removed lazily.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<AuthSessionSummary>, sea_orm::DbErr> {
        let Some(session) = auth_session::Entity::find()
            .filter(auth_session::Column::TokenHash.eq(token_hash))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        if is_expired_rfc3339(&session.expires_at) {
            let _ = self.delete_by_token_hash(token_hash).await?;
            return Ok(None);
        }

        let Some(user) = user::Entity::find_by_id(session.user_id.clone())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        Ok(Some(AuthSessionSummary::from_models(session, user)))
    }

    /// Physically delete expired sessions.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_expired(&self) -> Result<u64, sea_orm::DbErr> {
        let now = now_rfc3339();
        let result = auth_session::Entity::delete_many()
            .filter(auth_session::Column::ExpiresAt.lte(now))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected)
    }

    /// Delete one session by token hash.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_by_token_hash(&self, token_hash: &str) -> Result<bool, sea_orm::DbErr> {
        let result = auth_session::Entity::delete_many()
            .filter(auth_session::Column::TokenHash.eq(token_hash))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Delete all sessions belonging to a user.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_by_user_id(&self, user_id: &str) -> Result<u64, sea_orm::DbErr> {
        let result = auth_session::Entity::delete_many()
            .filter(auth_session::Column::UserId.eq(user_id))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected)
    }

    /// Delete all sessions belonging to a username.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_by_username(&self, username: &str) -> Result<u64, sea_orm::DbErr> {
        let Some(user) = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.db)
            .await?
        else {
            return Ok(0);
        };
        self.delete_by_user_id(&user.id).await
    }
}

impl AuthSessionSummary {
    fn from_models(session: auth_session::Model, user: user::Model) -> Self {
        Self {
            id: session.id,
            user_id: user.id,
            username: user.username,
            role: user.role,
            token_hash: session.token_hash,
            device_id: session.device_id,
            device_name: session.device_name,
            expires_at: session.expires_at,
            created_at: session.created_at,
        }
    }
}

fn is_expired_rfc3339(value: &str) -> bool {
    OffsetDateTime::parse(value, &Rfc3339)
        .map_or(true, |expires_at| expires_at <= OffsetDateTime::now_utc())
}

/// DTO for creating a new user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    /// Unique username.
    pub username: String,
    /// `BCrypt` password hash stored in the `password` column.
    pub password: String,
    /// System role (e.g. "admin", "operator", "viewer").
    pub role: String,
}

/// DTO for user updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUser {
    /// `BCrypt` password hash to update, if provided.
    pub password: Option<String>,
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

    /// Clone the underlying database connection for sibling repositories.
    #[must_use]
    pub fn db(&self) -> DatabaseConnection {
        self.db.clone()
    }

    /// Create a new user.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails or username is unique violation.
    pub async fn create_user(&self, params: CreateUser) -> Result<UserSummary, sea_orm::DbErr> {
        use crate::entities::user;

        let active = user::ActiveModel {
            id: Set(format!("usr-{}", Uuid::now_v7())),
            username: Set(params.username),
            password: Set(params.password),
            role: Set(params.role),
            created_at: Set(now_rfc3339()),
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
    pub async fn get_by_username(
        &self,
        username: &str,
    ) -> Result<Option<crate::entities::user::Model>, sea_orm::DbErr> {
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

        let res = user::Entity::delete_by_id(id.to_owned())
            .exec(&self.db)
            .await?;
        Ok(res.rows_affected > 0)
    }

    /// Get user by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get_user(
        &self,
        id: &str,
    ) -> Result<Option<crate::entities::user::Model>, sea_orm::DbErr> {
        use crate::entities::user;

        user::Entity::find_by_id(id.to_owned()).one(&self.db).await
    }

    /// Update user details.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn update_user(
        &self,
        id: &str,
        params: UpdateUser,
    ) -> Result<Option<UserSummary>, sea_orm::DbErr> {
        use crate::entities::user;

        let Some(existing) = user::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        let mut active: user::ActiveModel = existing.into();
        if let Some(hash) = params.password {
            active.password = Set(hash);
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

    /// List enabled jobs whose schedule type is managed by the scheduler tick loop.
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

/// Script creation input.
#[derive(Debug, Clone)]
pub struct CreateScript {
    /// Display name.
    pub name: String,
    /// Script language.
    pub language: String,
    /// Semantic version.
    pub version: String,
    /// Script source content.
    pub content: String,
    /// Creator user id.
    pub created_by: String,
    /// Optional timeout seconds.
    pub timeout_seconds: Option<i64>,
    /// Optional max memory bytes.
    pub max_memory_bytes: Option<i64>,
    /// Whether network access is allowed.
    pub allow_network: bool,
    /// Allowed environment variable names.
    pub allowed_env_vars: Option<String>,
}

/// Script update input.
#[derive(Debug, Clone)]
pub struct UpdateScript {
    /// Optional name update.
    pub name: Option<String>,
    /// Optional language update.
    pub language: Option<String>,
    /// Optional version update.
    pub version: Option<String>,
    /// Optional content update.
    pub content: Option<String>,
    /// Optional status update.
    pub status: Option<String>,
    /// Optional timeout seconds update.
    pub timeout_seconds: Option<i64>,
    /// Optional max memory bytes update.
    pub max_memory_bytes: Option<i64>,
    /// Optional network policy update.
    pub allow_network: Option<bool>,
    /// Optional env vars update.
    pub allowed_env_vars: Option<String>,
}

/// Script summary returned to management API callers.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScriptSummary {
    /// Script identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Script language.
    pub language: String,
    /// Semantic version.
    pub version: String,
    /// Approval status.
    pub status: String,
    /// Timeout seconds for execution.
    pub timeout_seconds: Option<i64>,
    /// Max memory bytes for sandbox.
    pub max_memory_bytes: Option<i64>,
    /// Whether network access is allowed.
    pub allow_network: bool,
    /// Allowed environment variable names.
    pub allowed_env_vars: Option<String>,
    /// Creator user id.
    pub created_by: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Script repository.
#[derive(Debug, Clone)]
pub struct ScriptRepository {
    db: DatabaseConnection,
    versions: ScriptVersionRepository,
}

impl ScriptRepository {
    /// Create a repository using the provided database connection.
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(db: DatabaseConnection) -> Self {
        let versions = ScriptVersionRepository::new(db.clone());
        Self { db, versions }
    }

    /// Access the version repository.
    #[must_use]
    pub fn versions(&self) -> &ScriptVersionRepository {
        &self.versions
    }
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_scripts(&self) -> Result<Vec<ScriptSummary>, sea_orm::DbErr> {
        let rows = script::Entity::find()
            .order_by_asc(script::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(ScriptSummary::from).collect())
    }

    /// Get one script by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get(&self, id: &str) -> Result<Option<ScriptSummary>, sea_orm::DbErr> {
        script::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await
            .map(|model| model.map(ScriptSummary::from))
    }

    /// Create a new script definition.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn create_script(
        &self,
        input: CreateScript,
    ) -> Result<ScriptSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let model = script::ActiveModel {
            id: Set(new_id("script")),
            name: Set(input.name),
            language: Set(input.language),
            version: Set(input.version),
            content: Set(input.content),
            status: Set("draft".to_owned()),
            timeout_seconds: Set(input.timeout_seconds),
            max_memory_bytes: Set(input.max_memory_bytes),
            allow_network: Set(input.allow_network),
            allowed_env_vars: Set(input.allowed_env_vars),
            created_by: Set(input.created_by),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(ScriptSummary::from(model))
    }

    /// Update a script definition.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn update_script(
        &self,
        id: &str,
        params: UpdateScript,
    ) -> Result<Option<ScriptSummary>, sea_orm::DbErr> {
        let Some(existing) = script::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        // Snapshot current state before applying changes
        self.versions.create_version(&existing).await?;
        let mut active: script::ActiveModel = existing.into();
        if let Some(name) = params.name {
            active.name = Set(name);
        }
        if let Some(language) = params.language {
            active.language = Set(language);
        }
        if let Some(version) = params.version {
            active.version = Set(version);
        }
        if let Some(content) = params.content {
            active.content = Set(content);
        }
        if let Some(status) = params.status {
            active.status = Set(status);
        }
        if let Some(timeout) = params.timeout_seconds {
            active.timeout_seconds = Set(Some(timeout));
        }
        if let Some(mem) = params.max_memory_bytes {
            active.max_memory_bytes = Set(Some(mem));
        }
        if let Some(allow) = params.allow_network {
            active.allow_network = Set(allow);
        }
        if let Some(env) = params.allowed_env_vars {
            active.allowed_env_vars = Set(Some(env));
        }
        active.updated_at = Set(now_rfc3339());
        let updated = active.update(&self.db).await?;
        Ok(Some(ScriptSummary::from(updated)))
    }

    /// Delete a script by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_script(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        let result = script::Entity::delete_by_id(id.to_owned())
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }
}

/// Summary of a script version snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptVersionSummary {
    /// Version record id.
    pub id: String,
    /// Script id this version belongs to.
    pub script_id: String,
    /// Monotonically increasing version number.
    pub version_number: i64,
    /// Snapshot of script content.
    pub content: String,
    /// Snapshot of language.
    pub language: String,
    /// Snapshot of status.
    pub status: String,
    /// Snapshot of `timeout_seconds`.
    pub timeout_seconds: Option<i64>,
    /// Snapshot of `max_memory_bytes`.
    pub max_memory_bytes: Option<i64>,
    /// Snapshot of `allow_network`.
    pub allow_network: bool,
    /// Snapshot of `allowed_env_vars`.
    pub allowed_env_vars: Option<String>,
    /// User who created this version.
    pub created_by: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Repository for script version history.
#[derive(Debug, Clone)]
pub struct ScriptVersionRepository {
    db: DatabaseConnection,
}

impl ScriptVersionRepository {
    #[allow(clippy::all)]
    /// Create a new repository.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a version snapshot from a script model.
    pub async fn create_version(
        &self,
        script: &script::Model,
    ) -> Result<ScriptVersionSummary, sea_orm::DbErr> {
        let max_version: i64 = script_version::Entity::find()
            .filter(script_version::Column::ScriptId.eq(&script.id))
            .all(&self.db)
            .await?
            .iter()
            .map(|v| v.version_number)
            .max()
            .unwrap_or(0);

        let version_number = max_version + 1;
        let id = format!("sv_{version_number}_{}", Uuid::new_v4().simple());

        let active = script_version::ActiveModel {
            id: Set(id),
            script_id: Set(script.id.clone()),
            version_number: Set(version_number),
            content: Set(script.content.clone()),
            language: Set(script.language.clone()),
            status: Set(script.status.clone()),
            timeout_seconds: Set(script.timeout_seconds),
            max_memory_bytes: Set(script.max_memory_bytes),
            allow_network: Set(script.allow_network),
            allowed_env_vars: Set(script.allowed_env_vars.clone()),
            created_by: Set(script.created_by.clone()),
            created_at: Set(now_rfc3339()),
        };
        let model = active.insert(&self.db).await?;
        Ok(ScriptVersionSummary::from(model))
    }

    /// List versions for a script, newest first.
    pub async fn list_versions(
        &self,
        script_id: &str,
    ) -> Result<Vec<ScriptVersionSummary>, sea_orm::DbErr> {
        let versions = script_version::Entity::find()
            .filter(script_version::Column::ScriptId.eq(script_id))
            .order_by_desc(script_version::Column::VersionNumber)
            .all(&self.db)
            .await?;
        Ok(versions
            .into_iter()
            .map(ScriptVersionSummary::from)
            .collect())
    }

    /// Get a specific version by id.
    pub async fn get_version(
        &self,
        id: &str,
    ) -> Result<Option<ScriptVersionSummary>, sea_orm::DbErr> {
        let version = script_version::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?;
        Ok(version.map(ScriptVersionSummary::from))
    }
}

impl From<script_version::Model> for ScriptVersionSummary {
    fn from(value: script_version::Model) -> Self {
        Self {
            id: value.id,
            script_id: value.script_id,
            version_number: value.version_number,
            content: value.content,
            language: value.language,
            status: value.status,
            timeout_seconds: value.timeout_seconds,
            max_memory_bytes: value.max_memory_bytes,
            allow_network: value.allow_network,
            allowed_env_vars: value.allowed_env_vars,
            created_by: value.created_by,
            created_at: value.created_at,
        }
    }
}

impl From<script::Model> for ScriptSummary {
    fn from(value: script::Model) -> Self {
        Self {
            id: value.id,
            name: value.name,
            language: value.language,
            version: value.version,
            status: value.status,
            timeout_seconds: value.timeout_seconds,
            max_memory_bytes: value.max_memory_bytes,
            allow_network: value.allow_network,
            allowed_env_vars: value.allowed_env_vars,
            created_by: value.created_by,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
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
    use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, Set, Statement};
    use sea_orm_migration::MigratorTrait;

    use scheduler_core::{ExecutionMode, InstanceStatus, TriggerType};

    use crate::{
        entities::auth_session,
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
    async fn auth_session_repository_deletes_expired_rows() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));

        let users = super::UserRepository::new(db.clone());
        let admin = users
            .get_by_username("scheduler_init")
            .await
            .unwrap_or_else(|error| panic!("admin lookup should work: {error}"))
            .unwrap_or_else(|| panic!("seeded admin should exist"));
        let sessions = super::AuthSessionRepository::new(db);
        auth_session::ActiveModel {
            id: Set("expired-session".to_owned()),
            user_id: Set(admin.id),
            token_hash: Set("expired-token-hash".to_owned()),
            device_id: Set(None),
            device_name: Set(None),
            expires_at: Set("1970-01-01T00:00:00Z".to_owned()),
            created_at: Set("1970-01-01T00:00:00Z".to_owned()),
            updated_at: Set("1970-01-01T00:00:00Z".to_owned()),
        }
        .insert(&sessions.db)
        .await
        .unwrap_or_else(|error| panic!("expired session should insert: {error}"));

        let deleted = sessions
            .delete_expired()
            .await
            .unwrap_or_else(|error| panic!("expired session should delete: {error}"));
        assert_eq!(deleted, 1);
        let loaded = sessions
            .get_by_token_hash("expired-token-hash")
            .await
            .unwrap_or_else(|error| panic!("session lookup should work: {error}"));
        assert!(loaded.is_none());
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
        let admin = admin.unwrap_or_else(|| panic!("seeded admin should exist"));
        assert_eq!(admin.role, "admin");

        // Create user
        let user = users
            .create_user(super::CreateUser {
                username: "operator-1".to_owned(),
                password: "$2b$10$operatorhash".to_owned(),
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
                    password: None,
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
