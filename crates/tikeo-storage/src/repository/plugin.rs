use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryOrder, Set};
use serde::{Deserialize, Serialize};

use crate::entities::plugin;

use super::util::{new_id, now_rfc3339};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginProcessorTypeSummary {
    /// Record type discriminator.
    pub r#type: String,
    /// Label value.
    pub label: String,
    /// Capability value.
    pub capability: String,
    #[serde(default)]
    /// Processor names value.
    pub processor_names: Vec<String>,
    /// Description value.
    pub description: Option<String>,
    /// Artifact ref value.
    pub artifact_ref: Option<String>,
    /// Container image value.
    pub container_image: Option<String>,
    /// Entrypoint value.
    pub entrypoint: Option<Vec<String>>,
    /// Checksum value.
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginAlertChannelTypeSummary {
    /// Record type discriminator.
    pub r#type: String,
    /// Label value.
    pub label: String,
    /// Target kind value.
    pub target_kind: String,
    /// Description value.
    pub description: Option<String>,
    /// Template value.
    pub template: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct CreatePlugin {
    /// Name value.
    pub name: String,
    pub kind: String,
    /// Processor types value.
    pub processor_types: Vec<PluginProcessorTypeSummary>,
    /// Alert channel types value.
    pub alert_channel_types: Vec<PluginAlertChannelTypeSummary>,
    /// Boolean state flag.
    pub enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UpdatePlugin {
    /// Name value.
    pub name: Option<String>,
    pub kind: Option<String>,
    /// Processor types value.
    pub processor_types: Option<Vec<PluginProcessorTypeSummary>>,
    /// Alert channel types value.
    pub alert_channel_types: Option<Vec<PluginAlertChannelTypeSummary>>,
    /// Boolean state flag.
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginSummary {
    /// Identifier value.
    pub id: String,
    /// Name value.
    pub name: String,
    pub kind: String,
    /// Processor types value.
    pub processor_types: Vec<PluginProcessorTypeSummary>,
    /// Alert channel types value.
    pub alert_channel_types: Vec<PluginAlertChannelTypeSummary>,
    /// Boolean state flag.
    pub enabled: bool,
    /// Timestamp value.
    pub created_at: String,
    /// Timestamp value.
    pub updated_at: String,
}

/// Persistent plugin registry repository.
#[derive(Debug, Clone)]
pub struct PluginRepository {
    db: DatabaseConnection,
}

impl PluginRepository {
    #[must_use]
    /// New.
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create plugin.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn create_plugin(
        &self,
        input: CreatePlugin,
    ) -> Result<PluginSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let model = plugin::ActiveModel {
            id: Set(new_id("plugin")),
            name: Set(input.name),
            kind: Set(input.kind),
            processor_types_json: Set(to_json(&input.processor_types)),
            alert_channel_types_json: Set(to_json(&input.alert_channel_types)),
            enabled: Set(input.enabled),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(PluginSummary::from(model))
    }

    /// Update plugin.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn update_plugin(
        &self,
        id: &str,
        input: UpdatePlugin,
    ) -> Result<Option<PluginSummary>, sea_orm::DbErr> {
        let Some(row) = plugin::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let mut active: plugin::ActiveModel = row.into();
        if let Some(name) = input.name {
            active.name = Set(name);
        }
        if let Some(kind) = input.kind {
            active.kind = Set(kind);
        }
        if let Some(processor_types) = input.processor_types {
            active.processor_types_json = Set(to_json(&processor_types));
        }
        if let Some(alert_channel_types) = input.alert_channel_types {
            active.alert_channel_types_json = Set(to_json(&alert_channel_types));
        }
        if let Some(enabled) = input.enabled {
            active.enabled = Set(enabled);
        }
        active.updated_at = Set(now_rfc3339());
        active
            .update(&self.db)
            .await
            .map(PluginSummary::from)
            .map(Some)
    }

    /// Delete plugin.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn delete_plugin(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        let result = plugin::Entity::delete_by_id(id.to_owned())
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// List plugins.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn list_plugins(&self) -> Result<Vec<PluginSummary>, sea_orm::DbErr> {
        let rows = plugin::Entity::find()
            .order_by_desc(plugin::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(PluginSummary::from).collect())
    }

    /// Get plugin.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn get_plugin(&self, id: &str) -> Result<Option<PluginSummary>, sea_orm::DbErr> {
        plugin::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await
            .map(|row| row.map(PluginSummary::from))
    }

    /// Resolve processor type.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn resolve_processor_type(
        &self,
        processor_type: &str,
    ) -> Result<Option<PluginProcessorTypeSummary>, sea_orm::DbErr> {
        let wanted = processor_type.trim();
        if wanted.is_empty() || wanted == "sdk" || wanted == "script" {
            return Ok(None);
        }
        Ok(self
            .list_plugins()
            .await?
            .into_iter()
            .filter(|plugin| plugin.enabled)
            .flat_map(|plugin| plugin.processor_types)
            .find(|item| item.r#type == wanted))
    }

    /// Resolve alert channel type.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn resolve_alert_channel_type(
        &self,
        channel_type: &str,
    ) -> Result<Option<PluginAlertChannelTypeSummary>, sea_orm::DbErr> {
        let wanted = channel_type.trim();
        if wanted.is_empty() {
            return Ok(None);
        }
        Ok(self
            .list_plugins()
            .await?
            .into_iter()
            .filter(|plugin| plugin.enabled)
            .flat_map(|plugin| plugin.alert_channel_types)
            .find(|item| item.r#type == wanted))
    }
}

impl From<plugin::Model> for PluginSummary {
    fn from(value: plugin::Model) -> Self {
        Self {
            id: value.id,
            name: value.name,
            kind: value.kind,
            processor_types: from_json(&value.processor_types_json),
            alert_channel_types: from_json(&value.alert_channel_types_json),
            enabled: value.enabled,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_owned())
}

fn from_json<T>(value: &str) -> Vec<T>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(value).unwrap_or_default()
}
