use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryOrder, Set};
use serde::{Deserialize, Serialize};

use crate::entities::service_account;

use super::util::{new_id, now_rfc3339};

/// Service account creation input.
#[derive(Debug, Clone)]
pub struct CreateServiceAccount {
    pub name: String,
    pub description: Option<String>,
    pub namespace: String,
    pub app: String,
    pub worker_pool: Option<String>,
    pub created_by: String,
}

/// Service account update input.
#[derive(Debug, Clone)]
pub struct UpdateServiceAccount {
    pub name: String,
    pub description: Option<String>,
    pub namespace: String,
    pub app: String,
    pub worker_pool: Option<String>,
    pub status: String,
    pub updated_by: String,
}

/// Service account metadata returned by repositories and HTTP APIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub namespace: String,
    pub app: String,
    pub worker_pool: Option<String>,
    pub status: String,
    pub created_by: String,
    pub updated_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Service account repository backed by the metadata database.
#[derive(Debug, Clone)]
pub struct ServiceAccountRepository {
    db: DatabaseConnection,
}

impl ServiceAccountRepository {
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create(
        &self,
        input: CreateServiceAccount,
    ) -> Result<ServiceAccountSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let model = service_account::ActiveModel {
            id: Set(new_id("sa")),
            name: Set(input.name),
            description: Set(input.description),
            namespace: Set(input.namespace),
            app: Set(input.app),
            worker_pool: Set(input.worker_pool),
            status: Set("active".to_owned()),
            created_by: Set(input.created_by),
            updated_by: Set(None),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(ServiceAccountSummary::from(model))
    }

    pub async fn list(&self) -> Result<Vec<ServiceAccountSummary>, sea_orm::DbErr> {
        let rows = service_account::Entity::find()
            .order_by_asc(service_account::Column::Namespace)
            .order_by_asc(service_account::Column::App)
            .order_by_asc(service_account::Column::Name)
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(ServiceAccountSummary::from).collect())
    }

    pub async fn get(&self, id: &str) -> Result<Option<ServiceAccountSummary>, sea_orm::DbErr> {
        service_account::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await
            .map(|row| row.map(ServiceAccountSummary::from))
    }

    pub async fn update(
        &self,
        id: &str,
        input: UpdateServiceAccount,
    ) -> Result<Option<ServiceAccountSummary>, sea_orm::DbErr> {
        let Some(model) = service_account::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let mut active: service_account::ActiveModel = model.into();
        active.name = Set(input.name);
        active.description = Set(input.description);
        active.namespace = Set(input.namespace);
        active.app = Set(input.app);
        active.worker_pool = Set(input.worker_pool);
        active.status = Set(input.status);
        active.updated_by = Set(Some(input.updated_by));
        active.updated_at = Set(now_rfc3339());
        let updated = active.update(&self.db).await?;
        Ok(Some(ServiceAccountSummary::from(updated)))
    }

    pub async fn disable(&self, id: &str, actor: &str) -> Result<bool, sea_orm::DbErr> {
        let Some(model) = service_account::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let mut active: service_account::ActiveModel = model.into();
        active.status = Set("disabled".to_owned());
        active.updated_by = Set(Some(actor.to_owned()));
        active.updated_at = Set(now_rfc3339());
        active.update(&self.db).await?;
        Ok(true)
    }
}

impl From<service_account::Model> for ServiceAccountSummary {
    fn from(model: service_account::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            namespace: model.namespace,
            app: model.app,
            worker_pool: model.worker_pool,
            status: model.status,
            created_by: model.created_by,
            updated_by: model.updated_by,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
