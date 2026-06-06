use sea_orm::{EntityTrait, QueryOrder, QuerySelect};

use crate::entities::dispatch_queue;

use super::{DispatchQueueSummary, QueueOverview, WorkflowRepository};

impl WorkflowRepository {
    pub async fn queue_overview(&self, limit: u64) -> Result<QueueOverview, sea_orm::DbErr> {
        let rows = dispatch_queue::Entity::find()
            .order_by_desc(dispatch_queue::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await?;
        let mut pending = 0;
        let mut running = 0;
        let mut done = 0;
        let mut failed = 0;
        for row in &rows {
            match row.status.as_str() {
                "pending" => pending += 1,
                "running" => running += 1,
                "done" => done += 1,
                "failed" => failed += 1,
                _ => {}
            }
        }
        Ok(QueueOverview {
            pending,
            running,
            done,
            failed,
            items: rows.into_iter().map(DispatchQueueSummary::from).collect(),
        })
    }
}
