use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, Set,
};

use crate::entities::oidc_identity;

use super::util::{new_id, now_rfc3339};

/// Input for creating or replacing an OIDC identity mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertOidcIdentity {
    /// External OIDC issuer URL.
    pub issuer: String,
    /// External OIDC subject claim.
    pub subject: String,
    /// Local tikeo username to issue sessions for.
    pub username: String,
    /// Optional namespace scope binding for issued sessions.
    pub namespace: Option<String>,
    /// Optional app scope binding for issued sessions.
    pub app: Option<String>,
    /// Optional worker-pool scope binding for issued sessions.
    pub worker_pool: Option<String>,
}

/// Persisted OIDC identity mapping summary.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct OidcIdentitySummary {
    /// Mapping row id.
    pub id: String,
    /// External OIDC issuer URL.
    pub issuer: String,
    /// External OIDC subject claim.
    pub subject: String,
    /// Local tikeo username.
    pub username: String,
    /// Optional namespace scope binding.
    pub namespace: Option<String>,
    /// Optional app scope binding.
    pub app: Option<String>,
    /// Optional worker-pool scope binding.
    pub worker_pool: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Repository for OIDC external-subject to local-user mappings.
#[derive(Debug, Clone)]
pub struct OidcIdentityRepository {
    db: DatabaseConnection,
}

impl OidcIdentityRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create or replace the soft-link mapping for one `(issuer, subject)` pair.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn upsert_identity(
        &self,
        input: UpsertOidcIdentity,
    ) -> Result<OidcIdentitySummary, sea_orm::DbErr> {
        if let Some(existing) = self
            .get_model_by_issuer_subject(&input.issuer, &input.subject)
            .await?
        {
            let mut active = existing.into_active_model();
            active.username = Set(input.username);
            active.namespace = Set(input.namespace);
            active.app = Set(input.app);
            active.worker_pool = Set(input.worker_pool);
            active.updated_at = Set(now_rfc3339());
            let updated = active.update(&self.db).await?;
            return Ok(OidcIdentitySummary::from_model(updated));
        }

        let now = now_rfc3339();
        let model = oidc_identity::ActiveModel {
            id: Set(new_id("oidc-identity")),
            issuer: Set(input.issuer),
            subject: Set(input.subject),
            username: Set(input.username),
            namespace: Set(input.namespace),
            app: Set(input.app),
            worker_pool: Set(input.worker_pool),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(OidcIdentitySummary::from_model(model))
    }

    /// List all external OIDC identity mappings.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn list_identities(&self) -> Result<Vec<OidcIdentitySummary>, sea_orm::DbErr> {
        let rows = oidc_identity::Entity::find()
            .order_by_asc(oidc_identity::Column::Issuer)
            .order_by_asc(oidc_identity::Column::Subject)
            .all(&self.db)
            .await?;
        Ok(rows
            .into_iter()
            .map(OidcIdentitySummary::from_model)
            .collect())
    }

    /// Delete one external OIDC subject mapping by id.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_identity(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        let result = oidc_identity::Entity::delete_many()
            .filter(oidc_identity::Column::Id.eq(id.to_owned()))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Resolve one external OIDC subject mapping.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn get_by_issuer_subject(
        &self,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<OidcIdentitySummary>, sea_orm::DbErr> {
        Ok(self
            .get_model_by_issuer_subject(issuer, subject)
            .await?
            .map(OidcIdentitySummary::from_model))
    }

    async fn get_model_by_issuer_subject(
        &self,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<oidc_identity::Model>, sea_orm::DbErr> {
        oidc_identity::Entity::find()
            .filter(oidc_identity::Column::Issuer.eq(issuer.to_owned()))
            .filter(oidc_identity::Column::Subject.eq(subject.to_owned()))
            .one(&self.db)
            .await
    }
}

impl OidcIdentitySummary {
    fn from_model(model: oidc_identity::Model) -> Self {
        Self {
            id: model.id,
            issuer: model.issuer,
            subject: model.subject,
            username: model.username,
            namespace: model.namespace,
            app: model.app,
            worker_pool: model.worker_pool,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
