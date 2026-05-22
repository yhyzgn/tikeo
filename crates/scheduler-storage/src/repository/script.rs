use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::entities::{script, script_version};

use super::util::{new_id, now_rfc3339};
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
    /// Script source content.
    pub content: String,
    /// Lowercase hex SHA-256 digest of the script content.
    pub content_sha256: String,
    /// Approval status.
    pub status: String,
    /// Timeout seconds for execution.
    pub timeout_seconds: Option<i64>,
    /// Max memory bytes for sandbox.
    pub max_memory_bytes: Option<i64>,
    /// Whether network access is allowed.
    pub allow_network: bool,
    /// Allowed environment variable names.
    pub allowed_env_vars: Option<Vec<String>>,
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
        self.versions.create_version(&model).await?;
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
        let before = existing.clone();
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
        if !script_changed(&before, &active) {
            return Ok(Some(ScriptSummary::from(before)));
        }

        active.updated_at = Set(now_rfc3339());
        let txn = self.db.begin().await?;
        let updated = active.update(&txn).await?;
        self.versions.create_version_in(&txn, &updated).await?;
        txn.commit().await?;
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

fn script_changed(before: &script::Model, active: &script::ActiveModel) -> bool {
    use sea_orm::ActiveValue;

    fn changed<T>(value: &ActiveValue<T>, before: &T) -> bool
    where
        T: PartialEq + Into<sea_orm::Value>,
    {
        matches!(value, ActiveValue::Set(after) if after != before)
    }

    changed(&active.name, &before.name)
        || changed(&active.language, &before.language)
        || changed(&active.version, &before.version)
        || changed(&active.content, &before.content)
        || changed(&active.status, &before.status)
        || changed(&active.timeout_seconds, &before.timeout_seconds)
        || changed(&active.max_memory_bytes, &before.max_memory_bytes)
        || changed(&active.allow_network, &before.allow_network)
        || changed(&active.allowed_env_vars, &before.allowed_env_vars)
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
    /// Lowercase hex SHA-256 digest of the content snapshot.
    pub content_sha256: String,
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
    pub allowed_env_vars: Option<Vec<String>>,
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
    /// Create a new repository.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a version snapshot from a script model.
    pub async fn create_version(
        &self,
        script: &script::Model,
    ) -> Result<ScriptVersionSummary, sea_orm::DbErr> {
        self.create_version_in(&self.db, script).await
    }

    async fn create_version_in<C>(
        &self,
        db: &C,
        script: &script::Model,
    ) -> Result<ScriptVersionSummary, sea_orm::DbErr>
    where
        C: sea_orm::ConnectionTrait,
    {
        let max_version: Option<i64> = script_version::Entity::find()
            .filter(script_version::Column::ScriptId.eq(&script.id))
            .select_only()
            .column_as(script_version::Column::VersionNumber.max(), "max_version")
            .into_tuple()
            .one(db)
            .await?;

        let version_number = max_version.unwrap_or(0) + 1;
        let id = format!("sv_{version_number}_{}", Uuid::new_v4().simple());

        let active = script_version::ActiveModel {
            id: Set(id),
            script_id: Set(script.id.clone()),
            version_number: Set(version_number),
            content_sha256: Set(content_sha256(&script.content)),
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
        let mut model = active.insert(db).await?;
        if model.content_sha256.is_empty() {
            model.content_sha256 = content_sha256(&model.content);
        }
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

    /// Get a specific version by script id and version number.
    pub async fn get_version_by_number(
        &self,
        script_id: &str,
        version_number: i64,
    ) -> Result<Option<ScriptVersionSummary>, sea_orm::DbErr> {
        let version = script_version::Entity::find()
            .filter(script_version::Column::ScriptId.eq(script_id))
            .filter(script_version::Column::VersionNumber.eq(version_number))
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
            content_sha256: if value.content_sha256.is_empty() {
                content_sha256(&value.content)
            } else {
                value.content_sha256
            },
            content: value.content,
            language: value.language,
            status: value.status,
            timeout_seconds: value.timeout_seconds,
            max_memory_bytes: value.max_memory_bytes,
            allow_network: value.allow_network,
            allowed_env_vars: value
                .allowed_env_vars
                .and_then(|s| serde_json::from_str(&s).ok()),
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
            content_sha256: content_sha256(&value.content),
            content: value.content,
            status: value.status,
            timeout_seconds: value.timeout_seconds,
            max_memory_bytes: value.max_memory_bytes,
            allow_network: value.allow_network,
            allowed_env_vars: value
                .allowed_env_vars
                .and_then(|s| serde_json::from_str(&s).ok()),
            created_by: value.created_by,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

fn content_sha256(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}
