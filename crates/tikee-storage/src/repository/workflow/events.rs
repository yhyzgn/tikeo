use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::entities::instance_event;

use super::{InstanceEventSummary, WorkflowRepository};

impl WorkflowRepository {
    pub async fn list_instance_events(
        &self,
        instance_id: &str,
    ) -> Result<Vec<InstanceEventSummary>, sea_orm::DbErr> {
        let rows = instance_event::Entity::find()
            .filter(instance_event::Column::InstanceId.eq(instance_id.to_owned()))
            .order_by_asc(instance_event::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(InstanceEventSummary::from).collect())
    }

    pub async fn list_instance_events_after(
        &self,
        instance_id: &str,
        after_created_at: Option<&str>,
    ) -> Result<Vec<InstanceEventSummary>, sea_orm::DbErr> {
        let mut query = instance_event::Entity::find()
            .filter(instance_event::Column::InstanceId.eq(instance_id.to_owned()))
            .order_by_asc(instance_event::Column::CreatedAt)
            .order_by_asc(instance_event::Column::Id);
        if let Some(after_created_at) = after_created_at {
            query = query.filter(instance_event::Column::CreatedAt.gt(after_created_at.to_owned()));
        }
        let rows = query.all(&self.db).await?;
        Ok(rows.into_iter().map(InstanceEventSummary::from).collect())
    }
}
