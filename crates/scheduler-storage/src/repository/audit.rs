use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};

use crate::entities::audit_log;

use super::util::{new_id, now_rfc3339};
/// Audit log creation input.
#[derive(Debug, Clone)]
pub struct CreateAuditLog {
    /// Actor who performed the action.
    pub actor: String,
    /// Action performed (e.g. `create`, `update`, `delete`).
    pub action: String,
    /// Resource type (e.g. `job`, `script`, `user`).
    pub resource_type: String,
    /// Resource identifier affected by the action.
    pub resource_id: String,
    /// Optional detail about the action.
    pub detail: Option<String>,
    /// Client IP address at the time of the action.
    pub ip_address: Option<String>,
}

/// Audit log summary returned to management API callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogSummary {
    /// Audit log identifier.
    pub id: String,
    /// Actor who performed the action.
    pub actor: String,
    /// Action performed.
    pub action: String,
    /// Resource type.
    pub resource_type: String,
    /// Resource identifier.
    pub resource_id: String,
    /// Optional detail about the action.
    pub detail: Option<String>,
    /// Client IP address.
    pub ip_address: Option<String>,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
}

/// Optional filters for listing audit logs.
#[derive(Debug, Clone, Default)]
pub struct AuditLogFilters {
    /// Filter by actor.
    pub actor: Option<String>,
    /// Filter by action.
    pub action: Option<String>,
    /// Filter by resource type.
    pub resource_type: Option<String>,
    /// Filter by resource id.
    pub resource_id: Option<String>,
    /// Maximum number of results (default 100).
    pub limit: Option<u64>,
}

/// Audit log repository.
#[derive(Debug, Clone)]
pub struct AuditLogRepository {
    db: DatabaseConnection,
}

impl AuditLogRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Append a new audit log entry.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn append(&self, input: CreateAuditLog) -> Result<AuditLogSummary, sea_orm::DbErr> {
        let model = audit_log::ActiveModel {
            id: Set(new_id("audit")),
            actor: Set(input.actor),
            action: Set(input.action),
            resource_type: Set(input.resource_type),
            resource_id: Set(input.resource_id),
            detail: Set(input.detail),
            ip_address: Set(input.ip_address),
            created_at: Set(now_rfc3339()),
        }
        .insert(&self.db)
        .await?;

        Ok(AuditLogSummary::from(model))
    }

    /// List audit logs with optional filters, ordered by creation time descending.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list(
        &self,
        filters: AuditLogFilters,
    ) -> Result<Vec<AuditLogSummary>, sea_orm::DbErr> {
        let mut query = audit_log::Entity::find();

        if let Some(actor) = &filters.actor {
            query = query.filter(audit_log::Column::Actor.eq(actor.clone()));
        }
        if let Some(action) = &filters.action {
            query = query.filter(audit_log::Column::Action.eq(action.clone()));
        }
        if let Some(resource_type) = &filters.resource_type {
            query = query.filter(audit_log::Column::ResourceType.eq(resource_type.clone()));
        }
        if let Some(resource_id) = &filters.resource_id {
            query = query.filter(audit_log::Column::ResourceId.eq(resource_id.clone()));
        }

        let rows = query
            .order_by_desc(audit_log::Column::CreatedAt)
            .limit(filters.limit.unwrap_or(100))
            .all(&self.db)
            .await?;

        Ok(rows.into_iter().map(AuditLogSummary::from).collect())
    }

    /// Get one audit log by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get(&self, id: &str) -> Result<Option<AuditLogSummary>, sea_orm::DbErr> {
        audit_log::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await
            .map(|model| model.map(AuditLogSummary::from))
    }
}

impl From<audit_log::Model> for AuditLogSummary {
    fn from(value: audit_log::Model) -> Self {
        Self {
            id: value.id,
            actor: value.actor,
            action: value.action,
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            detail: value.detail,
            ip_address: value.ip_address,
            created_at: value.created_at,
        }
    }
}
