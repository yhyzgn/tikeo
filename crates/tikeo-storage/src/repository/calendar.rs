use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};

use crate::entities::calendar;

use super::util::{new_id, now_rfc3339};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarWindowSummary {
    /// Start value.
    pub start: String,
    /// End value.
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CalendarSummary {
    /// Identifier value.
    pub id: String,
    /// Namespace value.
    pub namespace: String,
    /// App value.
    pub app: String,
    /// Name value.
    pub name: String,
    /// Timezone value.
    pub timezone: String,
    /// Excluded dates value.
    pub excluded_dates: Vec<String>,
    /// Holidays value.
    pub holidays: Vec<String>,
    /// Maintenance windows value.
    pub maintenance_windows: Vec<CalendarWindowSummary>,
    /// Freeze windows value.
    pub freeze_windows: Vec<CalendarWindowSummary>,
    /// Created by value.
    pub created_by: String,
    /// Timestamp value.
    pub created_at: String,
    /// Timestamp value.
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct UpsertCalendar {
    /// Namespace value.
    pub namespace: String,
    /// App value.
    pub app: String,
    /// Name value.
    pub name: String,
    /// Timezone value.
    pub timezone: String,
    /// Excluded dates value.
    pub excluded_dates: Vec<String>,
    /// Holidays value.
    pub holidays: Vec<String>,
    /// Maintenance windows value.
    pub maintenance_windows: Vec<CalendarWindowSummary>,
    /// Freeze windows value.
    pub freeze_windows: Vec<CalendarWindowSummary>,
    /// Created by value.
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct CalendarRepository {
    db: DatabaseConnection,
}

impl CalendarRepository {
    /// New.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// List.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn list(
        &self,
        namespace: Option<&str>,
        app: Option<&str>,
    ) -> Result<Vec<CalendarSummary>, sea_orm::DbErr> {
        let mut query = calendar::Entity::find().order_by_asc(calendar::Column::Name);
        if let Some(namespace) = namespace {
            query = query.filter(calendar::Column::Namespace.eq(namespace));
        }
        if let Some(app) = app {
            query = query.filter(calendar::Column::App.eq(app));
        }
        let rows = query.all(&self.db).await?;
        Ok(rows.into_iter().map(CalendarSummary::from).collect())
    }

    /// Get by name.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn get_by_name(
        &self,
        namespace: &str,
        app: &str,
        name: &str,
    ) -> Result<Option<CalendarSummary>, sea_orm::DbErr> {
        calendar::Entity::find()
            .filter(calendar::Column::Namespace.eq(namespace))
            .filter(calendar::Column::App.eq(app))
            .filter(calendar::Column::Name.eq(name))
            .one(&self.db)
            .await
            .map(|row| row.map(CalendarSummary::from))
    }

    /// Upsert.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn upsert(&self, input: UpsertCalendar) -> Result<CalendarSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let excluded_dates_json = to_json(&input.excluded_dates)?;
        let holidays_json = to_json(&input.holidays)?;
        let maintenance_windows_json = to_json(&input.maintenance_windows)?;
        let freeze_windows_json = to_json(&input.freeze_windows)?;
        if let Some(existing) = calendar::Entity::find()
            .filter(calendar::Column::Namespace.eq(input.namespace.clone()))
            .filter(calendar::Column::App.eq(input.app.clone()))
            .filter(calendar::Column::Name.eq(input.name.clone()))
            .one(&self.db)
            .await?
        {
            let mut active: calendar::ActiveModel = existing.into();
            active.timezone = Set(input.timezone);
            active.excluded_dates_json = Set(excluded_dates_json);
            active.holidays_json = Set(holidays_json);
            active.maintenance_windows_json = Set(maintenance_windows_json);
            active.freeze_windows_json = Set(freeze_windows_json);
            active.updated_at = Set(now);
            return active.update(&self.db).await.map(CalendarSummary::from);
        }
        calendar::ActiveModel {
            id: Set(new_id("cal")),
            namespace: Set(input.namespace),
            app: Set(input.app),
            name: Set(input.name),
            timezone: Set(input.timezone),
            excluded_dates_json: Set(excluded_dates_json),
            holidays_json: Set(holidays_json),
            maintenance_windows_json: Set(maintenance_windows_json),
            freeze_windows_json: Set(freeze_windows_json),
            created_by: Set(input.created_by),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await
        .map(CalendarSummary::from)
    }

    /// Delete.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn delete(&self, id: &str) -> Result<bool, sea_orm::DbErr> {
        let result = calendar::Entity::delete_by_id(id.to_owned())
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }
}

fn to_json<T: Serialize>(value: &T) -> Result<String, sea_orm::DbErr> {
    serde_json::to_string(value).map_err(|error| sea_orm::DbErr::Custom(error.to_string()))
}

fn from_json<T>(value: &str) -> T
where
    T: for<'de> Deserialize<'de> + Default,
{
    serde_json::from_str(value).unwrap_or_default()
}

impl From<calendar::Model> for CalendarSummary {
    fn from(value: calendar::Model) -> Self {
        Self {
            id: value.id,
            namespace: value.namespace,
            app: value.app,
            name: value.name,
            timezone: value.timezone,
            excluded_dates: from_json(&value.excluded_dates_json),
            holidays: from_json(&value.holidays_json),
            maintenance_windows: from_json(&value.maintenance_windows_json),
            freeze_windows: from_json(&value.freeze_windows_json),
            created_by: value.created_by,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
