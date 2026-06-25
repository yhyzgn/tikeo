use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};

use crate::entities::notification_template;

use super::util::{new_id, now_rfc3339};

#[derive(Debug, Clone)]
/// Public storage data type.
pub struct CreateNotificationTemplate {
    /// Template key value.
    pub template_key: String,
    /// Name value.
    pub name: String,
    /// Description value.
    pub description: Option<String>,
    /// Provider value.
    pub provider: String,
    /// Message type value.
    pub message_type: String,
    /// Boolean state flag.
    pub enabled: bool,
    /// Serialized data value.
    pub body_json: String,
    /// Serialized data value.
    pub variables_json: String,
}

#[derive(Debug, Clone, Default)]
/// Public storage data type.
pub struct UpdateNotificationTemplate {
    /// Template key value.
    pub template_key: Option<String>,
    /// Name value.
    pub name: Option<String>,
    /// Description value.
    pub description: Option<Option<String>>,
    /// Provider value.
    pub provider: Option<String>,
    /// Message type value.
    pub message_type: Option<String>,
    /// Boolean state flag.
    pub enabled: Option<bool>,
    /// Serialized data value.
    pub body_json: Option<String>,
    /// Serialized data value.
    pub variables_json: Option<String>,
    /// Updated by value.
    pub updated_by: Option<Option<String>>,
}

#[derive(Debug, Clone, Default)]
/// Public storage data type.
pub struct NotificationTemplateFilters {
    /// Provider value.
    pub provider: Option<String>,
    /// Message type value.
    pub message_type: Option<String>,
    /// Boolean state flag.
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
/// Public storage data type.
pub struct NotificationTemplateSummary {
    /// Identifier value.
    pub id: String,
    /// Template key value.
    pub template_key: String,
    /// Name value.
    pub name: String,
    /// Description value.
    pub description: Option<String>,
    /// Provider value.
    pub provider: String,
    /// Message type value.
    pub message_type: String,
    /// Boolean state flag.
    pub enabled: bool,
    /// Serialized data value.
    pub body_json: String,
    /// Serialized data value.
    pub variables_json: String,
    /// Created by value.
    pub created_by: Option<String>,
    /// Updated by value.
    pub updated_by: Option<String>,
    /// Timestamp value.
    pub created_at: String,
    /// Timestamp value.
    pub updated_at: String,
}

#[derive(Debug, Clone)]
/// Public storage data type.
pub struct NotificationTemplateRepository {
    db: DatabaseConnection,
}

impl NotificationTemplateRepository {
    /// New.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create template.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn create_template(
        &self,
        input: CreateNotificationTemplate,
    ) -> Result<NotificationTemplateSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let model = notification_template::ActiveModel {
            id: Set(new_id("notification-template")),
            template_key: Set(input.template_key),
            name: Set(input.name),
            description: Set(input.description),
            provider: Set(input.provider),
            message_type: Set(input.message_type),
            enabled: Set(input.enabled),
            body_json: Set(input.body_json),
            variables_json: Set(input.variables_json),
            created_by: Set(None),
            updated_by: Set(None),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(NotificationTemplateSummary::from(model))
    }

    /// Update template.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn update_template(
        &self,
        id: &str,
        input: UpdateNotificationTemplate,
    ) -> Result<Option<NotificationTemplateSummary>, sea_orm::DbErr> {
        let Some(row) = notification_template::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let mut active: notification_template::ActiveModel = row.into();
        if let Some(value) = input.template_key {
            active.template_key = Set(value);
        }
        if let Some(value) = input.name {
            active.name = Set(value);
        }
        if let Some(value) = input.description {
            active.description = Set(value);
        }
        if let Some(value) = input.provider {
            active.provider = Set(value);
        }
        if let Some(value) = input.message_type {
            active.message_type = Set(value);
        }
        if let Some(value) = input.enabled {
            active.enabled = Set(value);
        }
        if let Some(value) = input.body_json {
            active.body_json = Set(value);
        }
        if let Some(value) = input.variables_json {
            active.variables_json = Set(value);
        }
        if let Some(value) = input.updated_by {
            active.updated_by = Set(value);
        }
        active.updated_at = Set(now_rfc3339());
        active
            .update(&self.db)
            .await
            .map(NotificationTemplateSummary::from)
            .map(Some)
    }

    /// Get template.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn get_template(
        &self,
        id_or_key: &str,
    ) -> Result<Option<NotificationTemplateSummary>, sea_orm::DbErr> {
        if let Some(row) = notification_template::Entity::find_by_id(id_or_key.to_owned())
            .one(&self.db)
            .await?
        {
            return Ok(Some(NotificationTemplateSummary::from(row)));
        }
        notification_template::Entity::find()
            .filter(notification_template::Column::TemplateKey.eq(id_or_key.to_owned()))
            .one(&self.db)
            .await
            .map(|row| row.map(NotificationTemplateSummary::from))
    }

    /// List templates.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn list_templates(
        &self,
        filters: NotificationTemplateFilters,
    ) -> Result<Vec<NotificationTemplateSummary>, sea_orm::DbErr> {
        let mut query = notification_template::Entity::find();
        if let Some(value) = filters.provider {
            query = query.filter(notification_template::Column::Provider.eq(value));
        }
        if let Some(value) = filters.message_type {
            query = query.filter(notification_template::Column::MessageType.eq(value));
        }
        if let Some(value) = filters.enabled {
            query = query.filter(notification_template::Column::Enabled.eq(value));
        }
        let rows = query
            .order_by_desc(notification_template::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows
            .into_iter()
            .map(NotificationTemplateSummary::from)
            .collect())
    }

    /// Delete template.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn delete_template(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        let result = notification_template::Entity::delete_by_id(id.to_owned())
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }
}

impl From<notification_template::Model> for NotificationTemplateSummary {
    fn from(value: notification_template::Model) -> Self {
        Self {
            id: value.id,
            template_key: value.template_key,
            name: value.name,
            description: value.description,
            provider: value.provider,
            message_type: value.message_type,
            enabled: value.enabled,
            body_json: value.body_json,
            variables_json: value.variables_json,
            created_by: value.created_by,
            updated_by: value.updated_by,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
