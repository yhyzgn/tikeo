use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use crate::entities::sdk_api_key;

use super::util::{new_id, now_rfc3339};

/// Persisted SDK API key creation input.
#[derive(Debug, Clone)]
pub struct CreateSdkApiKey {
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub namespace: String,
    pub app: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<String>,
    pub created_by: String,
    pub rotated_from: Option<String>,
}

/// SDK API key metadata returned by repositories and HTTP APIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SdkApiKeySummary {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub namespace: String,
    pub app: String,
    pub scopes: Vec<String>,
    pub status: String,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
    pub created_by: String,
    pub revoked_by: Option<String>,
    pub rotated_from: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// SDK API key repository backed by the metadata database.
#[derive(Debug, Clone)]
pub struct SdkApiKeyRepository {
    db: DatabaseConnection,
}

impl SdkApiKeyRepository {
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_key(
        &self,
        input: CreateSdkApiKey,
    ) -> Result<SdkApiKeySummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let model = sdk_api_key::ActiveModel {
            id: Set(new_id("sk")),
            name: Set(input.name),
            key_hash: Set(input.key_hash),
            key_prefix: Set(input.key_prefix),
            namespace: Set(input.namespace),
            app: Set(input.app),
            scopes: Set(encode_scopes(&input.scopes)),
            status: Set("active".to_owned()),
            expires_at: Set(input.expires_at),
            last_used_at: Set(None),
            created_by: Set(input.created_by),
            revoked_by: Set(None),
            rotated_from: Set(input.rotated_from),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(SdkApiKeySummary::from(model))
    }

    pub async fn list_keys(&self) -> Result<Vec<SdkApiKeySummary>, sea_orm::DbErr> {
        let rows = sdk_api_key::Entity::find().all(&self.db).await?;
        Ok(rows.into_iter().map(SdkApiKeySummary::from).collect())
    }

    pub async fn get_active_by_hash(
        &self,
        key_hash: &str,
    ) -> Result<Option<SdkApiKeySummary>, sea_orm::DbErr> {
        let Some(model) = sdk_api_key::Entity::find()
            .filter(sdk_api_key::Column::KeyHash.eq(key_hash))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let summary = SdkApiKeySummary::from(model);
        if summary.status != "active" || summary.is_expired() {
            return Ok(None);
        }
        Ok(Some(summary))
    }

    pub async fn mark_used(&self, id: &str) -> Result<(), sea_orm::DbErr> {
        let Some(model) = sdk_api_key::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(());
        };
        let mut active: sdk_api_key::ActiveModel = model.into();
        let now = now_rfc3339();
        active.last_used_at = Set(Some(now.clone()));
        active.updated_at = Set(now);
        active.update(&self.db).await?;
        Ok(())
    }

    pub async fn revoke_key(&self, id: &str, actor: &str) -> Result<bool, sea_orm::DbErr> {
        let Some(model) = sdk_api_key::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let mut active: sdk_api_key::ActiveModel = model.into();
        active.status = Set("revoked".to_owned());
        active.revoked_by = Set(Some(actor.to_owned()));
        active.updated_at = Set(now_rfc3339());
        active.update(&self.db).await?;
        Ok(true)
    }
}

impl SdkApiKeySummary {
    fn is_expired(&self) -> bool {
        let Some(expires_at) = self.expires_at.as_deref() else {
            return false;
        };
        time::OffsetDateTime::parse(expires_at, &time::format_description::well_known::Rfc3339)
            .map_or(true, |value| value <= time::OffsetDateTime::now_utc())
    }
}

impl From<sdk_api_key::Model> for SdkApiKeySummary {
    fn from(model: sdk_api_key::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            key_prefix: model.key_prefix,
            namespace: model.namespace,
            app: model.app,
            scopes: decode_scopes(&model.scopes),
            status: model.status,
            expires_at: model.expires_at,
            last_used_at: model.last_used_at,
            created_by: model.created_by,
            revoked_by: model.revoked_by,
            rotated_from: model.rotated_from,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

fn encode_scopes(scopes: &[String]) -> String {
    scopes.join(",")
}

fn decode_scopes(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|scope| !scope.is_empty())
        .map(str::to_owned)
        .collect()
}
