use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait, sea_query::Expr,
};
use tikeo_core::InstanceStatus;

use super::{
    AdvanceWorkflowInput, CompleteWorkflowShardInput, CompleteWorkflowShardResult,
    CompletedShardContext, DispatchQueueClaim, DispatchQueueClaimKind, DispatchQueueShardOwner,
    DispatchQueueSloSummary, DispatchQueueSummary, RebalanceWorkflowShardsInput,
    RebalanceWorkflowShardsResult, RecoverWorkflowNodeInput, RecoverWorkflowNodeResult,
    ShardCompletionEventInput, WorkflowInstanceSummary, WorkflowJobBindingSummary,
    WorkflowJobResultOutcome, WorkflowNodeInstanceSummary, WorkflowRepository,
    WorkflowShardSummary, WorkflowSloSummary, aggregate_shard_node_status,
    dispatch_queue_age_seconds, elapsed_seconds, insert_shard_completion_event, json_string,
    maybe_persist_map_reduce_result, new_id, node_kind, normalize_processor_name,
    normalize_terminal_status, now_rfc3339, rfc3339_after_seconds, scheduler_shard_policy,
    success_ratio, update_shard_terminal, workflow_config_i64, workflow_config_string,
};
use crate::entities::{
    app as app_entity, dispatch_queue, instance_event, job_instance, namespace as namespace_entity,
    workflow_instance, workflow_node_instance, workflow_shard,
};
use crate::repository::ClusterShardOwnershipRepository;

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

fn dispatch_queue_current_shard_owner(
    row: &dispatch_queue::Model,
    ownership_by_shard: &std::collections::BTreeMap<(i32, i64, i32), String>,
) -> Option<String> {
    Some((row.shard_id?, row.shard_map_version?, row.shard_count?))
        .and_then(|key| ownership_by_shard.get(&key).cloned())
}

impl WorkflowRepository {
    pub async fn expire_timed_out_approval_nodes(&self) -> Result<u64, sea_orm::DbErr> {
        let now = now_rfc3339();
        let running = workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::Status.eq("running"))
            .all(&self.db)
            .await?;
        let mut expired = 0_u64;
        for node in running {
            let Some(instance) =
                workflow_instance::Entity::find_by_id(node.workflow_instance_id.clone())
                    .one(&self.db)
                    .await?
            else {
                continue;
            };
            let Some(workflow) = self.get_workflow(&instance.workflow_id).await? else {
                continue;
            };
            let Some(node_spec) = workflow
                .definition
                .nodes
                .iter()
                .find(|candidate| {
                    candidate.key == node.node_key && node_kind(candidate) == "approval"
                })
                .cloned()
            else {
                continue;
            };
            let timeout_seconds = workflow_config_i64(&node_spec, "timeoutSeconds")
                .or_else(|| workflow_config_i64(&node_spec, "timeout_seconds"))
                .filter(|value| *value >= 0)
                .unwrap_or(0);
            if timeout_seconds == 0
                || elapsed_seconds(&node.updated_at, &now) < timeout_seconds.cast_unsigned()
            {
                continue;
            }
            let status = workflow_config_string(&node_spec, "onTimeout")
                .or_else(|| workflow_config_string(&node_spec, "on_timeout"))
                .filter(|value| matches!(*value, "succeeded" | "failed" | "skipped"))
                .unwrap_or("failed")
                .to_owned();
            if self
                .advance_workflow(
                    &instance.id,
                    AdvanceWorkflowInput {
                        node_key: node.node_key.clone(),
                        status,
                        message: Some("approval SLA timed out".to_owned()),
                    },
                )
                .await?
                .is_some()
            {
                expired = expired.saturating_add(1);
            }
        }
        Ok(expired)
    }

    pub async fn job_binding_for_instance(
        &self,
        job_instance_id: &str,
    ) -> Result<Option<WorkflowJobBindingSummary>, sea_orm::DbErr> {
        if crate::entities::job_instance::Entity::find_by_id(job_instance_id.to_owned())
            .one(&self.db)
            .await?
            .is_none()
        {
            return Ok(None);
        }

        if let Some(node_instance) = workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&self.db)
            .await?
            && let Some(workflow_instance) =
                workflow_instance::Entity::find_by_id(node_instance.workflow_instance_id.clone())
                    .one(&self.db)
                    .await?
            && let Some(workflow) = self.get_workflow(&workflow_instance.workflow_id).await?
            && let Some(node) = workflow
                .definition
                .nodes
                .iter()
                .find(|node| node.key == node_instance.node_key)
        {
            return Ok(Some(WorkflowJobBindingSummary {
                node_kind: node_kind(node).to_owned(),
                processor_name: normalize_processor_name(node.processor_name.clone()),
                config: node.config.clone(),
            }));
        }

        if let Some(shard) = workflow_shard::Entity::find()
            .filter(workflow_shard::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&self.db)
            .await?
            && let Some(workflow) = self.get_workflow(&shard.workflow_instance_id).await?
            && let Some(node) = workflow
                .definition
                .nodes
                .iter()
                .find(|node| node.key == shard.node_key)
        {
            return Ok(Some(WorkflowJobBindingSummary {
                node_kind: node_kind(node).to_owned(),
                processor_name: normalize_processor_name(node.processor_name.clone()),
                config: node.config.clone(),
            }));
        }

        Ok(None)
    }

    pub async fn processor_name_for_job_instance(
        &self,
        job_instance_id: &str,
    ) -> Result<Option<String>, sea_orm::DbErr> {
        Ok(self
            .job_binding_for_instance(job_instance_id)
            .await?
            .and_then(|binding| binding.processor_name))
    }

    pub async fn get_node_by_job_instance(
        &self,
        job_instance_id: &str,
    ) -> Result<Option<WorkflowNodeInstanceSummary>, sea_orm::DbErr> {
        workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&self.db)
            .await
            .map(|model| model.map(WorkflowNodeInstanceSummary::from))
    }

    pub async fn complete_job_node_from_result(
        &self,
        job_instance_id: &str,
        status: InstanceStatus,
        message: Option<String>,
    ) -> Result<Option<WorkflowJobResultOutcome>, sea_orm::DbErr> {
        let terminal_status = if status == InstanceStatus::Succeeded {
            "succeeded"
        } else {
            "failed"
        }
        .to_owned();
        if let Some(shard_result) = self
            .complete_shard_by_job_instance(
                job_instance_id,
                CompleteWorkflowShardInput {
                    status: terminal_status.clone(),
                    output: None,
                    checkpoint: None,
                    message: message.clone(),
                },
            )
            .await?
        {
            self.mark_job_queue_done(job_instance_id, terminal_status.as_str())
                .await?;
            return Ok(shard_result
                .advance
                .map(|advance| WorkflowJobResultOutcome {
                    workflow_instance_id: advance.instance.id,
                    node_key: shard_result.shard.node_key,
                    status: terminal_status,
                    queued_nodes: advance.queued_nodes,
                    completed: advance.completed,
                }));
        }
        let Some(node) = workflow_node_instance::Entity::find()
            .filter(workflow_node_instance::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let node_key = node.node_key.clone();
        let workflow_instance_id = node.workflow_instance_id.clone();
        let advance = self
            .advance_workflow(
                &workflow_instance_id,
                AdvanceWorkflowInput {
                    node_key: node_key.clone(),
                    status: terminal_status.clone(),
                    message: message.or_else(|| {
                        Some(format!(
                            "job instance {job_instance_id} completed as {terminal_status}"
                        ))
                    }),
                },
            )
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(workflow_instance_id.clone()))?;
        self.mark_job_queue_done(job_instance_id, terminal_status.as_str())
            .await?;
        Ok(Some(WorkflowJobResultOutcome {
            workflow_instance_id,
            node_key,
            status: terminal_status,
            queued_nodes: advance.queued_nodes,
            completed: advance.completed,
        }))
    }

    async fn complete_shard_by_job_instance(
        &self,
        job_instance_id: &str,
        input: CompleteWorkflowShardInput,
    ) -> Result<Option<CompleteWorkflowShardResult>, sea_orm::DbErr> {
        let Some(shard) = workflow_shard::Entity::find()
            .filter(workflow_shard::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        self.complete_workflow_shard(&shard.id, input).await
    }

    async fn mark_job_queue_done(
        &self,
        job_instance_id: &str,
        node_status: &str,
    ) -> Result<(), sea_orm::DbErr> {
        let Some(queue_row) = dispatch_queue::Entity::find()
            .filter(dispatch_queue::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&self.db)
            .await?
        else {
            return Ok(());
        };
        let mut active: dispatch_queue::ActiveModel = queue_row.into();
        active.status = Set(if node_status == "succeeded" {
            "done".to_owned()
        } else {
            "failed".to_owned()
        });
        active.lease_owner = Set(None);
        active.lease_until = Set(None);
        active.fencing_token = Set(None);
        active.updated_at = Set(now_rfc3339());
        active.update(&self.db).await?;
        Ok(())
    }

    pub async fn list_workflow_shards(
        &self,
        instance_id: &str,
    ) -> Result<Vec<WorkflowShardSummary>, sea_orm::DbErr> {
        let rows = workflow_shard::Entity::find()
            .filter(workflow_shard::Column::WorkflowInstanceId.eq(instance_id.to_owned()))
            .order_by_asc(workflow_shard::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(WorkflowShardSummary::from).collect())
    }

    pub async fn complete_workflow_shard(
        &self,
        shard_id: &str,
        input: CompleteWorkflowShardInput,
    ) -> Result<Option<CompleteWorkflowShardResult>, sea_orm::DbErr> {
        let status = normalize_terminal_status(&input.status)?;
        let Some(shard) = workflow_shard::Entity::find_by_id(shard_id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        if matches!(shard.status.as_str(), "succeeded" | "failed") {
            return Ok(Some(CompleteWorkflowShardResult {
                shard: WorkflowShardSummary::from(shard),
                node_completed: false,
                node_status: None,
                advance: None,
            }));
        }

        let context = self.persist_completed_shard(shard, input, &status).await?;
        if let Some(job_instance_id) = &context.job_instance_id {
            self.mark_job_queue_done(job_instance_id, &status).await?;
        }

        let node_status = aggregate_shard_node_status(context.has_failed, context.all_succeeded);
        let advance = if let Some(node_status) = &node_status {
            self.advance_workflow(
                &context.workflow_instance_id,
                AdvanceWorkflowInput {
                    node_key: context.node_key,
                    status: node_status.clone(),
                    message: Some(format!(
                        "workflow shards completed with aggregate status {node_status}"
                    )),
                },
            )
            .await?
        } else {
            None
        };

        Ok(Some(CompleteWorkflowShardResult {
            shard: context.updated,
            node_completed: node_status.is_some(),
            node_status,
            advance,
        }))
    }

    async fn persist_completed_shard(
        &self,
        shard: workflow_shard::Model,
        input: CompleteWorkflowShardInput,
        status: &str,
    ) -> Result<CompletedShardContext, sea_orm::DbErr> {
        let now = now_rfc3339();
        let output = json_string(input.output.as_ref())?;
        let checkpoint = json_string(input.checkpoint.as_ref())?;
        let workflow_instance_id = shard.workflow_instance_id.clone();
        let workflow_node_instance_id = shard.workflow_node_instance_id.clone();
        let job_instance_id = shard.job_instance_id.clone();
        let node_key = shard.node_key.clone();
        let shard_index = shard.shard_index;
        let txn = self.db.begin().await?;
        let updated =
            update_shard_terminal(&txn, shard, status, output.clone(), checkpoint, &now).await?;
        insert_shard_completion_event(
            &txn,
            ShardCompletionEventInput {
                workflow_instance_id: workflow_instance_id.clone(),
                node_key: node_key.clone(),
                shard_index,
                status: status.to_owned(),
                message: input.message,
                output,
                now: now.clone(),
            },
        )
        .await?;
        let sibling_rows = workflow_shard::Entity::find()
            .filter(
                workflow_shard::Column::WorkflowNodeInstanceId
                    .eq(workflow_node_instance_id.clone()),
            )
            .all(&txn)
            .await?;
        let has_failed = sibling_rows.iter().any(|row| row.status == "failed");
        let all_succeeded = sibling_rows.iter().all(|row| row.status == "succeeded");
        maybe_persist_map_reduce_result(
            &txn,
            &workflow_instance_id,
            &workflow_node_instance_id,
            &sibling_rows,
            &now,
        )
        .await?;
        txn.commit().await?;
        Ok(CompletedShardContext {
            workflow_instance_id,
            job_instance_id,
            node_key,
            updated: WorkflowShardSummary::from(updated),
            has_failed,
            all_succeeded,
        })
    }

    pub(super) async fn propagate_child_workflow_completion(
        &self,
        child_instance: &WorkflowInstanceSummary,
    ) -> Result<(), sea_orm::DbErr> {
        let Some(parent_node) = workflow_node_instance::Entity::find()
            .filter(
                workflow_node_instance::Column::ChildWorkflowInstanceId
                    .eq(child_instance.id.clone()),
            )
            .one(&self.db)
            .await?
        else {
            return Ok(());
        };
        if matches!(
            parent_node.status.as_str(),
            "succeeded" | "failed" | "skipped"
        ) {
            return Ok(());
        }
        let parent_status = if child_instance.status == "succeeded" {
            "succeeded"
        } else {
            "failed"
        };
        let _ = Box::pin(self.advance_workflow(
            &parent_node.workflow_instance_id,
            AdvanceWorkflowInput {
                node_key: parent_node.node_key,
                status: parent_status.to_owned(),
                message: Some(format!(
                    "child workflow {} completed as {}",
                    child_instance.id, child_instance.status
                )),
            },
        ))
        .await?;
        Ok(())
    }

    pub async fn claim_next_dispatch_queue_item(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_next_dispatch_queue_item_with_fencing(lease_owner, lease_seconds, None)
            .await
    }

    pub async fn claim_next_dispatch_queue_item_with_fencing(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
        fencing_token: Option<&str>,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_next_dispatch_queue_item_matching(
            lease_owner,
            lease_seconds,
            DispatchQueueClaimKind::Any,
            fencing_token,
            None,
        )
        .await
    }

    pub async fn claim_next_workflow_node_queue_item(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_next_workflow_node_queue_item_with_fencing(
            lease_owner,
            lease_seconds,
            lease_owner,
        )
        .await
    }

    pub async fn claim_next_workflow_node_queue_item_with_fencing(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
        fencing_token: &str,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_next_dispatch_queue_item_matching(
            lease_owner,
            lease_seconds,
            DispatchQueueClaimKind::WorkflowNode,
            Some(fencing_token),
            None,
        )
        .await
    }

    pub async fn claim_next_job_queue_item(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_next_job_queue_item_with_fencing(lease_owner, lease_seconds, lease_owner)
            .await
    }

    pub async fn claim_next_workflow_node_queue_item_for_shard_owner(
        &self,
        owner: DispatchQueueShardOwner,
        lease_seconds: i64,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        if !ClusterShardOwnershipRepository::new(self.db.clone())
            .accepts_fencing_token(
                owner.shard_id,
                &owner.owner_node_id,
                owner.owner_epoch,
                &owner.owner_fencing_token,
            )
            .await?
        {
            return Ok(None);
        }
        let fencing_token = owner.owner_fencing_token.clone();
        let owner_node_id = owner.owner_node_id.clone();
        self.claim_next_dispatch_queue_item_matching(
            &owner_node_id,
            lease_seconds,
            DispatchQueueClaimKind::WorkflowNode,
            Some(&fencing_token),
            Some(owner),
        )
        .await
    }

    pub async fn claim_next_job_queue_item_with_fencing(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
        fencing_token: &str,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_next_dispatch_queue_item_matching(
            lease_owner,
            lease_seconds,
            DispatchQueueClaimKind::JobInstance,
            Some(fencing_token),
            None,
        )
        .await
    }

    pub async fn claim_next_job_queue_item_for_shard_owner(
        &self,
        owner: DispatchQueueShardOwner,
        lease_seconds: i64,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        if !ClusterShardOwnershipRepository::new(self.db.clone())
            .accepts_fencing_token(
                owner.shard_id,
                &owner.owner_node_id,
                owner.owner_epoch,
                &owner.owner_fencing_token,
            )
            .await?
        {
            return Ok(None);
        }
        let fencing_token = owner.owner_fencing_token.clone();
        let owner_node_id = owner.owner_node_id.clone();
        self.claim_next_dispatch_queue_item_matching(
            &owner_node_id,
            lease_seconds,
            DispatchQueueClaimKind::JobInstance,
            Some(&fencing_token),
            Some(owner),
        )
        .await
    }

    async fn claim_next_dispatch_queue_item_matching(
        &self,
        lease_owner: &str,
        lease_seconds: i64,
        kind: DispatchQueueClaimKind,
        fencing_token: Option<&str>,
        shard_owner: Option<DispatchQueueShardOwner>,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        let now = now_rfc3339();
        let mut query = dispatch_queue::Entity::find()
            .filter(dispatch_queue::Column::Status.eq("pending"))
            .filter(dispatch_queue::Column::RunAfter.lte(now.clone()))
            .filter(
                dispatch_queue::Column::LeaseUntil
                    .is_null()
                    .or(dispatch_queue::Column::LeaseUntil.lt(now.clone())),
            )
            .order_by_asc(dispatch_queue::Column::Priority)
            .order_by_asc(dispatch_queue::Column::RunAfter);
        query = match kind {
            DispatchQueueClaimKind::Any => query,
            DispatchQueueClaimKind::WorkflowNode => {
                query.filter(dispatch_queue::Column::WorkflowNodeInstanceId.is_not_null())
            }
            DispatchQueueClaimKind::JobInstance => {
                query.filter(dispatch_queue::Column::JobInstanceId.is_not_null())
            }
        };
        if let Some(owner) = shard_owner.as_ref() {
            query = query.filter(dispatch_queue::Column::ShardId.eq(owner.shard_id));
        }
        let candidates = query
            .select_only()
            .column(dispatch_queue::Column::Id)
            .into_tuple::<(String,)>()
            .all(&self.db)
            .await?;
        let mut queue_id = None;
        for (candidate_id,) in candidates {
            if kind == DispatchQueueClaimKind::JobInstance
                && self
                    .dispatch_queue_item_blocked_by_quota(&candidate_id)
                    .await?
            {
                continue;
            }
            queue_id = Some(candidate_id);
            break;
        }
        let Some(queue_id) = queue_id else {
            return Ok(None);
        };
        self.claim_dispatch_queue_item_with_owner_fencing(
            &queue_id,
            lease_owner,
            lease_seconds,
            fencing_token,
            shard_owner,
        )
        .await
    }

    pub async fn claim_dispatch_queue_item(
        &self,
        queue_id: &str,
        lease_owner: &str,
        lease_seconds: i64,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_dispatch_queue_item_with_fencing(queue_id, lease_owner, lease_seconds, None)
            .await
    }

    async fn dispatch_queue_item_blocked_by_quota(
        &self,
        queue_id: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let Some(row) = dispatch_queue::Entity::find_by_id(queue_id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let (Some(namespace), Some(app), Some(pool)) = (row.namespace, row.app, row.worker_pool)
        else {
            return Ok(false);
        };
        let Some(namespace_model) = namespace_entity::Entity::find()
            .filter(namespace_entity::Column::Name.eq(namespace.clone()))
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let Some(app_model) = app_entity::Entity::find()
            .filter(app_entity::Column::NamespaceId.eq(namespace_model.id.clone()))
            .filter(app_entity::Column::Name.eq(app.clone()))
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let Some(scope) = crate::entities::worker_pool::Entity::find()
            .filter(crate::entities::worker_pool::Column::NamespaceId.eq(namespace_model.id))
            .filter(crate::entities::worker_pool::Column::AppId.eq(app_model.id))
            .filter(crate::entities::worker_pool::Column::Name.eq(pool.clone()))
            .one(&self.db)
            .await?
        else {
            return Ok(false);
        };
        let active_depth = dispatch_queue::Entity::find()
            .filter(dispatch_queue::Column::Namespace.eq(namespace.clone()))
            .filter(dispatch_queue::Column::App.eq(app.clone()))
            .filter(dispatch_queue::Column::WorkerPool.eq(pool.clone()))
            .filter(dispatch_queue::Column::Status.is_in(["pending", "running"]))
            .all(&self.db)
            .await?
            .len();
        if scope.max_queue_depth > 0
            && active_depth > usize::try_from(scope.max_queue_depth).unwrap_or(usize::MAX)
        {
            return Ok(true);
        }
        let running_depth = dispatch_queue::Entity::find()
            .filter(dispatch_queue::Column::Namespace.eq(namespace))
            .filter(dispatch_queue::Column::App.eq(app))
            .filter(dispatch_queue::Column::WorkerPool.eq(pool))
            .filter(dispatch_queue::Column::Status.eq("running"))
            .all(&self.db)
            .await?
            .len();
        Ok(scope.max_concurrency > 0
            && running_depth >= usize::try_from(scope.max_concurrency).unwrap_or(usize::MAX))
    }

    pub async fn claim_dispatch_queue_item_with_fencing(
        &self,
        queue_id: &str,
        lease_owner: &str,
        lease_seconds: i64,
        fencing_token: Option<&str>,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        self.claim_dispatch_queue_item_with_owner_fencing(
            queue_id,
            lease_owner,
            lease_seconds,
            fencing_token,
            None,
        )
        .await
    }

    async fn claim_dispatch_queue_item_with_owner_fencing(
        &self,
        queue_id: &str,
        lease_owner: &str,
        lease_seconds: i64,
        fencing_token: Option<&str>,
        shard_owner: Option<DispatchQueueShardOwner>,
    ) -> Result<Option<DispatchQueueClaim>, sea_orm::DbErr> {
        let now = now_rfc3339();
        let lease_until = rfc3339_after_seconds(lease_seconds.max(1));
        let fencing_token = fencing_token.map_or_else(
            || format!("lease:{lease_owner}:{queue_id}:{lease_until}"),
            ToOwned::to_owned,
        );
        let shard_map_version = shard_owner.as_ref().map(|owner| owner.shard_map_version);
        let shard_count = shard_owner.as_ref().map(|owner| owner.shard_count);
        let owner_epoch = shard_owner.as_ref().map(|owner| owner.owner_epoch);
        let owner_fencing_token = shard_owner
            .as_ref()
            .map(|owner| owner.owner_fencing_token.clone());
        let txn = self.db.begin().await?;
        let mut update = dispatch_queue::Entity::update_many()
            .col_expr(
                dispatch_queue::Column::LeaseOwner,
                Expr::value(Some(lease_owner.to_owned())),
            )
            .col_expr(
                dispatch_queue::Column::LeaseUntil,
                Expr::value(Some(lease_until.clone())),
            )
            .col_expr(
                dispatch_queue::Column::FencingToken,
                Expr::value(Some(fencing_token.clone())),
            )
            .col_expr(
                dispatch_queue::Column::ShardMapVersion,
                Expr::value(shard_map_version),
            )
            .col_expr(dispatch_queue::Column::ShardCount, Expr::value(shard_count))
            .col_expr(dispatch_queue::Column::OwnerEpoch, Expr::value(owner_epoch))
            .col_expr(
                dispatch_queue::Column::OwnerFencingToken,
                Expr::value(owner_fencing_token),
            )
            .col_expr(
                dispatch_queue::Column::Attempt,
                Expr::col(dispatch_queue::Column::Attempt).add(1),
            )
            .col_expr(dispatch_queue::Column::UpdatedAt, Expr::value(now.clone()))
            .filter(dispatch_queue::Column::Id.eq(queue_id.to_owned()))
            .filter(dispatch_queue::Column::Status.eq("pending"));
        if let Some(owner) = shard_owner.as_ref() {
            update = update.filter(dispatch_queue::Column::ShardId.eq(owner.shard_id));
        }
        let result = update
            .filter(
                dispatch_queue::Column::LeaseUntil
                    .is_null()
                    .or(dispatch_queue::Column::LeaseUntil.lt(now)),
            )
            .exec(&txn)
            .await?;
        if result.rows_affected == 0 {
            txn.commit().await?;
            return Ok(None);
        }
        let updated = dispatch_queue::Entity::find_by_id(queue_id.to_owned())
            .one(&txn)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(queue_id.to_owned()))?;
        txn.commit().await?;
        Ok(Some(DispatchQueueClaim {
            item: DispatchQueueSummary::from(updated),
            lease_owner: lease_owner.to_owned(),
            lease_until,
            fencing_token,
        }))
    }

    pub async fn mark_dispatch_queue_running(
        &self,
        queue_id: &str,
        lease_owner: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let result = dispatch_queue::Entity::update_many()
            .col_expr(dispatch_queue::Column::Status, Expr::value("running"))
            .col_expr(
                dispatch_queue::Column::LeaseOwner,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::LeaseUntil,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::FencingToken,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::UpdatedAt,
                Expr::value(now_rfc3339()),
            )
            .filter(dispatch_queue::Column::Id.eq(queue_id.to_owned()))
            .filter(dispatch_queue::Column::LeaseOwner.eq(lease_owner.to_owned()))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    pub async fn requeue_stale_running_job_dispatches(
        &self,
        stale_after_seconds: i64,
    ) -> Result<u64, sea_orm::DbErr> {
        let cutoff = rfc3339_after_seconds(-stale_after_seconds.max(1));
        let now = now_rfc3339();
        let txn = self.db.begin().await?;
        let instance_ids = dispatch_queue::Entity::find()
            .select_only()
            .column(dispatch_queue::Column::JobInstanceId)
            .filter(dispatch_queue::Column::Status.eq("running"))
            .filter(dispatch_queue::Column::JobInstanceId.is_not_null())
            .filter(dispatch_queue::Column::UpdatedAt.lt(cutoff))
            .into_tuple::<(Option<String>,)>()
            .all(&txn)
            .await?
            .into_iter()
            .filter_map(|(id,)| id)
            .collect::<Vec<_>>();
        if instance_ids.is_empty() {
            txn.commit().await?;
            return Ok(0);
        }
        let queue_result = dispatch_queue::Entity::update_many()
            .col_expr(dispatch_queue::Column::Status, Expr::value("pending"))
            .col_expr(
                dispatch_queue::Column::LeaseOwner,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::LeaseUntil,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::FencingToken,
                Expr::value(Option::<String>::None),
            )
            .col_expr(dispatch_queue::Column::UpdatedAt, Expr::value(now.clone()))
            .filter(dispatch_queue::Column::JobInstanceId.is_in(instance_ids.clone()))
            .filter(dispatch_queue::Column::Status.eq("running"))
            .exec(&txn)
            .await?;
        job_instance::Entity::update_many()
            .col_expr(
                job_instance::Column::Status,
                Expr::value(InstanceStatus::Pending.to_string()),
            )
            .col_expr(job_instance::Column::UpdatedAt, Expr::value(now))
            .filter(job_instance::Column::Id.is_in(instance_ids))
            .filter(job_instance::Column::Status.eq(InstanceStatus::Running.to_string()))
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(queue_result.rows_affected)
    }

    pub async fn clear_expired_dispatch_queue_leases(&self) -> Result<u64, sea_orm::DbErr> {
        let now = now_rfc3339();
        let result = dispatch_queue::Entity::update_many()
            .col_expr(
                dispatch_queue::Column::LeaseOwner,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::LeaseUntil,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::FencingToken,
                Expr::value(Option::<String>::None),
            )
            .col_expr(dispatch_queue::Column::UpdatedAt, Expr::value(now.clone()))
            .filter(dispatch_queue::Column::Status.eq("pending"))
            .filter(dispatch_queue::Column::LeaseUntil.is_not_null())
            .filter(dispatch_queue::Column::LeaseUntil.lt(now))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected)
    }

    pub async fn dispatch_queue_for_instance(
        &self,
        instance_id: &str,
    ) -> Result<Option<DispatchQueueSummary>, sea_orm::DbErr> {
        let row = dispatch_queue::Entity::find()
            .filter(dispatch_queue::Column::JobInstanceId.eq(instance_id.to_owned()))
            .one(&self.db)
            .await?;
        Ok(row.map(DispatchQueueSummary::from))
    }

    pub async fn requeue_dispatch_queue_for_retry(
        &self,
        instance_id: &str,
        delay_seconds: i64,
    ) -> Result<Option<DispatchQueueSummary>, sea_orm::DbErr> {
        let Some(row) = dispatch_queue::Entity::find()
            .filter(dispatch_queue::Column::JobInstanceId.eq(instance_id.to_owned()))
            .filter(dispatch_queue::Column::Status.is_in(["pending", "running"]))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let run_after = if delay_seconds > 0 {
            rfc3339_after_seconds(delay_seconds)
        } else {
            now_rfc3339()
        };
        let now = now_rfc3339();
        let mut active: dispatch_queue::ActiveModel = row.into();
        active.status = Set("pending".to_owned());
        active.run_after = Set(run_after);
        active.lease_owner = Set(None);
        active.lease_until = Set(None);
        active.fencing_token = Set(None);
        active.updated_at = Set(now.clone());
        let updated = active.update(&self.db).await?;
        job_instance::Entity::update_many()
            .col_expr(
                job_instance::Column::Status,
                Expr::value(InstanceStatus::Retrying.to_string()),
            )
            .col_expr(job_instance::Column::UpdatedAt, Expr::value(now))
            .filter(job_instance::Column::Id.eq(instance_id.to_owned()))
            .filter(job_instance::Column::Status.is_in([
                InstanceStatus::Running.to_string(),
                InstanceStatus::Retrying.to_string(),
                InstanceStatus::Dispatching.to_string(),
                InstanceStatus::Failed.to_string(),
            ]))
            .exec(&self.db)
            .await?;
        Ok(Some(DispatchQueueSummary::from(updated)))
    }

    pub async fn mark_dispatch_queue_done_by_instance(
        &self,
        instance_id: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let result = dispatch_queue::Entity::update_many()
            .col_expr(dispatch_queue::Column::Status, Expr::value("done"))
            .col_expr(
                dispatch_queue::Column::LeaseOwner,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::LeaseUntil,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::FencingToken,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::UpdatedAt,
                Expr::value(now_rfc3339()),
            )
            .filter(dispatch_queue::Column::JobInstanceId.eq(instance_id.to_owned()))
            .filter(dispatch_queue::Column::Status.is_in(["pending", "running"]))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    pub async fn mark_dispatch_queue_failed(
        &self,
        queue_id: &str,
        lease_owner: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        let result = dispatch_queue::Entity::update_many()
            .col_expr(dispatch_queue::Column::Status, Expr::value("failed"))
            .col_expr(
                dispatch_queue::Column::LeaseOwner,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::LeaseUntil,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::FencingToken,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::UpdatedAt,
                Expr::value(now_rfc3339()),
            )
            .filter(dispatch_queue::Column::Id.eq(queue_id.to_owned()))
            .filter(dispatch_queue::Column::LeaseOwner.eq(lease_owner.to_owned()))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected > 0)
    }

    pub async fn release_dispatch_queue_item(
        &self,
        queue_id: &str,
        lease_owner: &str,
    ) -> Result<bool, sea_orm::DbErr> {
        self.release_dispatch_queue_item_after(queue_id, lease_owner, 0)
            .await
    }

    pub async fn release_dispatch_queue_item_after(
        &self,
        queue_id: &str,
        lease_owner: &str,
        delay_seconds: i64,
    ) -> Result<bool, sea_orm::DbErr> {
        let Some(row) = dispatch_queue::Entity::find_by_id(queue_id.to_owned())
            .one(&self.db)
            .await?
            .filter(|row| row.lease_owner.as_deref() == Some(lease_owner))
        else {
            return Ok(false);
        };
        let mut active: dispatch_queue::ActiveModel = row.into();
        active.lease_owner = Set(None);
        active.lease_until = Set(None);
        active.fencing_token = Set(None);
        active.run_after = Set(if delay_seconds > 0 {
            rfc3339_after_seconds(delay_seconds)
        } else {
            now_rfc3339()
        });
        active.updated_at = Set(now_rfc3339());
        active.update(&self.db).await?;
        Ok(true)
    }

    pub async fn dispatch_queue_slo_summary(
        &self,
    ) -> Result<DispatchQueueSloSummary, sea_orm::DbErr> {
        let rows = dispatch_queue::Entity::find().all(&self.db).await?;
        let ownership_by_shard = ClusterShardOwnershipRepository::new(self.db.clone())
            .list()
            .await?
            .into_iter()
            .filter(|row| row.status == "active")
            .map(|row| {
                (
                    (row.shard_id, row.shard_map_version, row.shard_count),
                    row.owner_node_id,
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let now = time::OffsetDateTime::now_utc();
        let mut summary = DispatchQueueSloSummary::default();
        let mut pending_age_total = 0_u64;
        let mut dispatch_latency_total = 0_u64;

        for row in rows {
            summary.total = summary.total.saturating_add(1);
            *summary.by_status.entry(row.status.clone()).or_insert(0) += 1;
            match row.status.as_str() {
                "pending" => {
                    summary.pending = summary.pending.saturating_add(1);
                    if self.dispatch_queue_item_blocked_by_quota(&row.id).await? {
                        summary.blocked_by_quota = summary.blocked_by_quota.saturating_add(1);
                    }
                    let age = dispatch_queue_age_seconds(&row.created_at, now);
                    pending_age_total = pending_age_total.saturating_add(age);
                    summary.oldest_pending_age_seconds =
                        summary.oldest_pending_age_seconds.max(age);
                    if let Some(owner) =
                        dispatch_queue_current_shard_owner(&row, &ownership_by_shard)
                    {
                        *summary
                            .pending_by_shard_owner
                            .entry(owner.clone())
                            .or_insert(0) += 1;
                        let entry = summary
                            .oldest_pending_age_by_shard_owner
                            .entry(owner)
                            .or_insert(0);
                        *entry = (*entry).max(age);
                    }
                }
                "running" => {
                    summary.running = summary.running.saturating_add(1);
                    if let Some(owner) =
                        dispatch_queue_current_shard_owner(&row, &ownership_by_shard)
                            .or_else(|| row.lease_owner.clone())
                    {
                        *summary.running_by_shard_owner.entry(owner).or_insert(0) += 1;
                    }
                }
                "done" | "failed" => {
                    summary.completed_dispatches = summary.completed_dispatches.saturating_add(1);
                    let latency = elapsed_seconds(&row.created_at, &row.updated_at);
                    dispatch_latency_total = dispatch_latency_total.saturating_add(latency);
                    summary.longest_dispatch_latency_seconds =
                        summary.longest_dispatch_latency_seconds.max(latency);
                }
                _ => {}
            }
        }

        summary.average_pending_age_seconds =
            pending_age_total.checked_div(summary.pending).unwrap_or(0);
        summary.average_dispatch_latency_seconds = dispatch_latency_total
            .checked_div(summary.completed_dispatches)
            .unwrap_or(0);

        Ok(summary)
    }

    pub async fn workflow_slo_summary(&self) -> Result<WorkflowSloSummary, sea_orm::DbErr> {
        let instances = workflow_instance::Entity::find().all(&self.db).await?;
        let shards = workflow_shard::Entity::find().all(&self.db).await?;
        let mut summary = WorkflowSloSummary::default();
        let mut instance_successes = 0_u64;
        let mut instance_failures = 0_u64;
        let mut instance_duration_total = 0_u64;
        let mut shard_successes = 0_u64;
        let mut shard_failures = 0_u64;
        let mut shard_duration_total = 0_u64;

        for row in instances {
            summary.instances_total = summary.instances_total.saturating_add(1);
            *summary
                .instances_by_status
                .entry(row.status.clone())
                .or_insert(0) += 1;
            match row.status.as_str() {
                "succeeded" => {
                    instance_successes = instance_successes.saturating_add(1);
                    summary.completed_instances = summary.completed_instances.saturating_add(1);
                    let duration = elapsed_seconds(&row.created_at, &row.updated_at);
                    instance_duration_total = instance_duration_total.saturating_add(duration);
                    summary.longest_instance_duration_seconds =
                        summary.longest_instance_duration_seconds.max(duration);
                }
                "failed" => {
                    instance_failures = instance_failures.saturating_add(1);
                    summary.completed_instances = summary.completed_instances.saturating_add(1);
                    let duration = elapsed_seconds(&row.created_at, &row.updated_at);
                    instance_duration_total = instance_duration_total.saturating_add(duration);
                    summary.longest_instance_duration_seconds =
                        summary.longest_instance_duration_seconds.max(duration);
                }
                _ => {}
            }
        }
        summary.average_instance_duration_seconds = instance_duration_total
            .checked_div(summary.completed_instances)
            .unwrap_or(0);
        summary.instance_success_ratio = success_ratio(instance_successes, instance_failures);

        for row in shards {
            summary.shards_total = summary.shards_total.saturating_add(1);
            *summary
                .shards_by_status
                .entry(row.status.clone())
                .or_insert(0) += 1;
            match row.status.as_str() {
                "succeeded" => {
                    shard_successes = shard_successes.saturating_add(1);
                    summary.completed_shards = summary.completed_shards.saturating_add(1);
                    let duration = elapsed_seconds(&row.created_at, &row.updated_at);
                    shard_duration_total = shard_duration_total.saturating_add(duration);
                    summary.longest_shard_duration_seconds =
                        summary.longest_shard_duration_seconds.max(duration);
                }
                "failed" => {
                    shard_failures = shard_failures.saturating_add(1);
                    summary.completed_shards = summary.completed_shards.saturating_add(1);
                    let duration = elapsed_seconds(&row.created_at, &row.updated_at);
                    shard_duration_total = shard_duration_total.saturating_add(duration);
                    summary.longest_shard_duration_seconds =
                        summary.longest_shard_duration_seconds.max(duration);
                }
                _ => {}
            }
        }
        summary.average_shard_duration_seconds = shard_duration_total
            .checked_div(summary.completed_shards)
            .unwrap_or(0);
        summary.shard_success_ratio = success_ratio(shard_successes, shard_failures);

        Ok(summary)
    }

    pub async fn cancel_job_instance(&self, job_instance_id: &str) -> Result<bool, sea_orm::DbErr> {
        let now = now_rfc3339();
        let txn = self.db.begin().await?;
        let instance_result = crate::entities::job_instance::Entity::update_many()
            .col_expr(
                crate::entities::job_instance::Column::Status,
                Expr::value("cancelled"),
            )
            .col_expr(
                crate::entities::job_instance::Column::UpdatedAt,
                Expr::value(now.clone()),
            )
            .filter(crate::entities::job_instance::Column::Id.eq(job_instance_id.to_owned()))
            .filter(crate::entities::job_instance::Column::Status.is_in([
                "pending",
                "dispatching",
                "running",
            ]))
            .exec(&txn)
            .await?;
        dispatch_queue::Entity::update_many()
            .col_expr(dispatch_queue::Column::Status, Expr::value("cancelled"))
            .col_expr(
                dispatch_queue::Column::LeaseOwner,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::LeaseUntil,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                dispatch_queue::Column::FencingToken,
                Expr::value(Option::<String>::None),
            )
            .col_expr(dispatch_queue::Column::UpdatedAt, Expr::value(now.clone()))
            .filter(dispatch_queue::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .exec(&txn)
            .await?;
        if let Some(shard) = workflow_shard::Entity::find()
            .filter(workflow_shard::Column::JobInstanceId.eq(job_instance_id.to_owned()))
            .one(&txn)
            .await?
        {
            let mut active: workflow_shard::ActiveModel = shard.into();
            active.status = Set("cancelled".to_owned());
            active.updated_at = Set(now.clone());
            active.update(&txn).await?;
        }
        instance_event::ActiveModel {
            id: Set(new_id("evt")),
            instance_id: Set(job_instance_id.to_owned()),
            instance_type: Set("job".to_owned()),
            event_type: Set("job.instance.cancelled".to_owned()),
            message: Set("job instance cancelled".to_owned()),
            payload: Set(None),
            created_at: Set(now),
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        Ok(instance_result.rows_affected > 0)
    }

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
