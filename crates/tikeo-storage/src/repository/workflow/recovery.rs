use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};

use super::{
    AdvanceWorkflowInput, RebalanceWorkflowShardsInput, RebalanceWorkflowShardsResult,
    RecoverWorkflowNodeInput, RecoverWorkflowNodeResult, WorkflowRepository, WorkflowShardSummary,
    new_id, now_rfc3339, scheduler_shard_policy,
};
use crate::entities::{
    dispatch_queue, instance_event, workflow_instance, workflow_node_instance, workflow_shard,
};

fn workflow_runtime_dispatch_shard(
    namespace: &str,
    app: &str,
    durable_id: &str,
) -> (i32, i64, i32) {
    let policy = scheduler_shard_policy();
    (
        policy.shard_id_for(namespace, app, durable_id),
        policy.shard_map_version,
        policy.shard_count,
    )
}

impl WorkflowRepository {
    /// Rebalance workflow shards.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn rebalance_workflow_shards(
        &self,
        instance_id: &str,
        input: RebalanceWorkflowShardsInput,
    ) -> Result<Option<RebalanceWorkflowShardsResult>, sea_orm::DbErr> {
        let instance_exists = workflow_instance::Entity::find_by_id(instance_id.to_owned())
            .one(&self.db)
            .await?
            .is_some();
        if !instance_exists {
            return Ok(None);
        }
        let statuses = input.statuses.unwrap_or_else(|| vec!["failed".to_owned()]);
        let now = now_rfc3339();
        let mut query = workflow_shard::Entity::find()
            .filter(workflow_shard::Column::WorkflowInstanceId.eq(instance_id.to_owned()))
            .filter(workflow_shard::Column::Status.is_in(statuses));
        if let Some(node_key) = input
            .node_key
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            query = query.filter(workflow_shard::Column::NodeKey.eq(node_key.trim().to_owned()));
        }
        let shards = query.all(&self.db).await?;
        let txn = self.db.begin().await?;
        let mut requeued = Vec::new();
        for shard in shards {
            let next_retry_count = shard.retry_count.saturating_add(1);
            let job_instance_id = new_id("inst");
            crate::entities::job_instance::ActiveModel {
                id: Set(job_instance_id.clone()),
                job_id: Set(format!(
                    "workflow-shard-{}-{}",
                    shard.workflow_instance_id, shard.node_key
                )),
                status: Set("pending".to_owned()),
                trigger_type: Set("workflow_shard".to_owned()),
                execution_mode: Set("single".to_owned()),
                result_worker_id: Set(None),
                result_success: Set(None),
                result_message: Set(None),
                result_completed_at: Set(None),
                created_at: Set(now.clone()),
                updated_at: Set(now.clone()),
            }
            .insert(&txn)
            .await?;
            let (shard_id, shard_map_version, shard_count) =
                workflow_runtime_dispatch_shard("workflow", instance_id, &job_instance_id);
            dispatch_queue::ActiveModel {
                id: Set(new_id("dq")),
                job_instance_id: Set(Some(job_instance_id.clone())),
                workflow_node_instance_id: Set(None),
                shard_id: Set(Some(shard_id)),
                shard_map_version: Set(Some(shard_map_version)),
                shard_count: Set(Some(shard_count)),
                owner_epoch: Set(None),
                owner_fencing_token: Set(None),
                priority: Set(0),
                run_after: Set(now.clone()),
                status: Set("pending".to_owned()),
                attempt: Set(0),
                lease_owner: Set(None),
                lease_until: Set(None),
                fencing_token: Set(None),
                worker_selector: Set(None),
                namespace: Set(None),
                app: Set(None),
                worker_pool: Set(None),
                created_at: Set(now.clone()),
                updated_at: Set(now.clone()),
            }
            .insert(&txn)
            .await?;
            let mut active: workflow_shard::ActiveModel = shard.into();
            active.status = Set("pending".to_owned());
            active.output = Set(None);
            active.retry_count = Set(next_retry_count);
            active.job_instance_id = Set(Some(job_instance_id));
            active.updated_at = Set(now.clone());
            let updated = active.update(&txn).await?;
            requeued.push(WorkflowShardSummary::from(updated));
        }
        instance_event::ActiveModel {
            id: Set(new_id("evt")),
            instance_id: Set(instance_id.to_owned()),
            instance_type: Set("workflow".to_owned()),
            event_type: Set("workflow.shards.rebalanced".to_owned()),
            message: Set(input
                .message
                .unwrap_or_else(|| format!("rebalanced {} workflow shards", requeued.len()))),
            payload: Set(Some(
                serde_json::to_string(&requeued)
                    .map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?,
            )),
            created_at: Set(now),
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        Ok(Some(RebalanceWorkflowShardsResult {
            requeued_shards: requeued,
        }))
    }

    /// Recover workflow node.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying operation fails.
    pub async fn recover_workflow_node(
        &self,
        instance_id: &str,
        input: RecoverWorkflowNodeInput,
    ) -> Result<Option<RecoverWorkflowNodeResult>, sea_orm::DbErr> {
        let status = match input.action.as_str() {
            "retry" => "queued",
            "skip" => "skipped",
            "fail" => "failed",
            "succeed" => "succeeded",
            other => {
                return Err(sea_orm::DbErr::Custom(format!(
                    "unsupported recovery action: {other}"
                )));
            }
        };
        let Some(node) = workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::WorkflowInstanceId.eq(instance_id.to_owned()))
            .filter(workflow_node_instance::Column::NodeKey.eq(input.node_key.clone()))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let now = now_rfc3339();
        let txn = self.db.begin().await?;
        let mut node_active: workflow_node_instance::ActiveModel = node.into();
        node_active.status = Set(status.to_owned());
        node_active.updated_at = Set(now.clone());
        let updated = node_active.update(&txn).await?;
        if input.action == "retry" {
            let (shard_id, shard_map_version, shard_count) = workflow_runtime_dispatch_shard(
                "workflow",
                instance_id,
                &format!("{instance_id}:{}", updated.node_key),
            );
            dispatch_queue::ActiveModel {
                id: Set(new_id("dq")),
                job_instance_id: Set(None),
                workflow_node_instance_id: Set(Some(updated.id.clone())),
                shard_id: Set(Some(shard_id)),
                shard_map_version: Set(Some(shard_map_version)),
                shard_count: Set(Some(shard_count)),
                owner_epoch: Set(None),
                owner_fencing_token: Set(None),
                priority: Set(0),
                run_after: Set(now.clone()),
                status: Set("pending".to_owned()),
                attempt: Set(0),
                lease_owner: Set(None),
                lease_until: Set(None),
                fencing_token: Set(None),
                worker_selector: Set(None),
                namespace: Set(None),
                app: Set(None),
                worker_pool: Set(None),
                created_at: Set(now.clone()),
                updated_at: Set(now.clone()),
            }
            .insert(&txn)
            .await?;
        }
        instance_event::ActiveModel {
            id: Set(new_id("evt")),
            instance_id: Set(instance_id.to_owned()),
            instance_type: Set("workflow".to_owned()),
            event_type: Set(format!("workflow.node.recovery.{}", input.action)),
            message: Set(input
                .message
                .unwrap_or_else(|| format!("node {} {}", input.node_key, input.action))),
            payload: Set(None),
            created_at: Set(now),
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        let advance = if matches!(input.action.as_str(), "skip" | "fail" | "succeed") {
            self.advance_workflow(
                instance_id,
                AdvanceWorkflowInput {
                    node_key: input.node_key,
                    status: status.to_owned(),
                    message: None,
                },
            )
            .await?
        } else {
            None
        };
        let instance = self
            .get_workflow_instance(instance_id)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(instance_id.to_owned()))?;
        Ok(Some(RecoverWorkflowNodeResult {
            instance,
            queued_nodes: advance.map_or_else(Vec::new, |result| result.queued_nodes),
        }))
    }
}
