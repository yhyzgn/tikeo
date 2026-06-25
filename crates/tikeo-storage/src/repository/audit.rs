use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
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
    /// Optional JSON snapshot before the action.
    pub before: Option<String>,
    /// Optional JSON snapshot after the action.
    pub after: Option<String>,
    /// Request trace id.
    pub trace_id: Option<String>,
    /// Result status (`success` or `failed`).
    pub result: String,
    /// Optional failure reason.
    pub failure_reason: Option<String>,
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
    /// Optional JSON snapshot before the action.
    pub before: Option<String>,
    /// Optional JSON snapshot after the action.
    pub after: Option<String>,
    /// Request trace id.
    pub trace_id: Option<String>,
    /// Result status (`success` or `failed`).
    pub result: String,
    /// Optional failure reason.
    pub failure_reason: Option<String>,
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
    /// Filter by failure reason.
    pub failure_reason: Option<String>,
    /// Maximum number of results (default 100).
    pub limit: Option<u64>,
    /// Number of rows to skip.
    pub offset: Option<u64>,
}

/// Paginated audit log query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogPageSummary {
    /// Matching page items.
    pub items: Vec<AuditLogSummary>,
    /// Total matching row count before pagination.
    pub total: u64,
    /// Opaque next page token, currently the next offset.
    pub next_page_token: Option<String>,
}

/// Audit log repository.
#[derive(Debug, Clone)]
pub struct AuditLogRepository {
    db: DatabaseConnection,
}

impl AuditLogRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    /// New.
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    #[must_use]
    /// Db.
    pub fn db(&self) -> DatabaseConnection {
        self.db.clone()
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
            before: Set(input.before),
            after: Set(input.after),
            trace_id: Set(input.trace_id),
            result: Set(if input.result.trim().is_empty() {
                "success".to_owned()
            } else {
                input.result
            }),
            failure_reason: Set(input.failure_reason),
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
        self.list_page(filters).await.map(|page| page.items)
    }

    /// List one page of audit logs with optional filters.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_page(
        &self,
        filters: AuditLogFilters,
    ) -> Result<AuditLogPageSummary, sea_orm::DbErr> {
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
        if let Some(failure_reason) = &filters.failure_reason {
            query = query.filter(audit_log::Column::FailureReason.eq(failure_reason.clone()));
        }

        let limit = filters.limit.unwrap_or(100).clamp(1, 500);
        let offset = filters.offset.unwrap_or(0);
        let total = query.clone().count(&self.db).await?;
        let rows = query
            .order_by_desc(audit_log::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(&self.db)
            .await?;
        let next_offset = offset.saturating_add(rows.len() as u64);
        let next_page_token = (next_offset < total).then(|| next_offset.to_string());

        Ok(AuditLogPageSummary {
            items: rows.into_iter().map(AuditLogSummary::from).collect(),
            total,
            next_page_token,
        })
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
            before: value.before,
            after: value.after,
            trace_id: value.trace_id,
            result: value.result,
            failure_reason: value.failure_reason,
            ip_address: value.ip_address,
            created_at: value.created_at,
        }
    }
}
