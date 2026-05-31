use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use crate::entities::secret;

use super::util::{new_id, now_rfc3339};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretSummary {
    pub id: String,
    pub namespace: String,
    pub app: String,
    pub name: String,
    pub value_ref: String,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CreateSecret {
    pub namespace: String,
    pub app: String,
    pub name: String,
    pub value_ref: String,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct SecretRepository {
    db: DatabaseConnection,
}

impl SecretRepository {
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list(
        &self,
        namespace: Option<&str>,
        app: Option<&str>,
    ) -> Result<Vec<SecretSummary>, sea_orm::DbErr> {
        let mut query = secret::Entity::find().filter(secret::Column::Status.eq("active"));
        if let Some(namespace) = namespace {
            query = query.filter(secret::Column::Namespace.eq(namespace));
        }
        if let Some(app) = app {
            query = query.filter(secret::Column::App.eq(app));
        }
        let rows = query.all(&self.db).await?;
        Ok(rows.into_iter().map(SecretSummary::from).collect())
    }

    pub async fn create(&self, input: CreateSecret) -> Result<SecretSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        if let Some(existing) = secret::Entity::find()
            .filter(secret::Column::Namespace.eq(input.namespace.clone()))
            .filter(secret::Column::App.eq(input.app.clone()))
            .filter(secret::Column::Name.eq(input.name.clone()))
            .one(&self.db)
            .await?
        {
            let mut active: secret::ActiveModel = existing.into();
            active.value_ref = Set(input.value_ref);
            active.status = Set("active".to_owned());
            active.updated_at = Set(now);
            return active.update(&self.db).await.map(SecretSummary::from);
        }
        secret::ActiveModel {
            id: Set(new_id("sec")),
            namespace: Set(input.namespace),
            app: Set(input.app),
            name: Set(input.name),
            value_ref: Set(input.value_ref),
            status: Set("active".to_owned()),
            created_by: Set(input.created_by),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await
        .map(SecretSummary::from)
    }

    pub async fn delete(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        let Some(existing) = secret::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let mut active: secret::ActiveModel = existing.into();
        active.status = Set("deleted".to_owned());
        active.updated_at = Set(now_rfc3339());
        active.update(&self.db).await?;
        Ok(true)
    }
}

impl From<secret::Model> for SecretSummary {
    fn from(value: secret::Model) -> Self {
        Self {
            id: value.id,
            namespace: value.namespace,
            app: value.app,
            name: value.name,
            value_ref: value.value_ref,
            status: value.status,
            created_by: value.created_by,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
