//! Minimal pending-instance dispatcher for Worker Tunnel sessions.

use std::time::Duration;

use scheduler_core::InstanceStatus;
use scheduler_proto::worker::v1::DispatchTask;
use scheduler_storage::{
    JobInstanceAttemptRepository, JobInstanceRepository, JobRepository, WorkflowRepository,
};
use tokio::time;
use tracing::{debug, warn};

use super::WorkerRegistry;
use crate::cluster::SharedClusterCoordinator;

const DISPATCH_INTERVAL: Duration = Duration::from_millis(500);
const DISPATCH_BATCH_SIZE: u64 = 16;
const DISPATCH_LEASE_SECONDS: i64 = 30;
const DISPATCHER_LEASE_OWNER: &str = "scheduler-dispatcher";

fn dispatcher_fencing_token(node_id: &str, leader_fencing_token: Option<&str>) -> String {
    leader_fencing_token.map_or_else(
        || format!("standalone:{node_id}:{DISPATCHER_LEASE_OWNER}"),
        |token| format!("raft:{node_id}:{token}"),
    )
}

/// Run the minimal single-node dispatch loop forever.
pub async fn run(
    jobs: JobRepository,
    instances: JobInstanceRepository,
    attempts: JobInstanceAttemptRepository,
    workflows: WorkflowRepository,
    registry: WorkerRegistry,
    cluster: SharedClusterCoordinator,
) {
    let mut ticker = time::interval(DISPATCH_INTERVAL);
    loop {
        ticker.tick().await;
        if let Err(error) = dispatch_once_if_owner(
            &jobs, &instances, &attempts, &workflows, &registry, &cluster,
        )
        .await
        {
            warn!(%error, "worker dispatch iteration failed");
        }
    }
}

async fn dispatch_once_if_owner(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    workflows: &WorkflowRepository,
    registry: &WorkerRegistry,
    cluster: &SharedClusterCoordinator,
) -> Result<(), scheduler_storage::DbErr> {
    let status = cluster.status().await;
    if !status.can_schedule {
        debug!(role = status.role.as_str(), node_id = %status.node_id, "skip worker dispatch without cluster ownership");
        return Ok(());
    }
    let fencing_token =
        dispatcher_fencing_token(&status.node_id, status.leader_fencing_token.as_deref());
    dispatch_once(
        jobs,
        instances,
        attempts,
        workflows,
        registry,
        &fencing_token,
    )
    .await
}

async fn dispatch_once(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    workflows: &WorkflowRepository,
    registry: &WorkerRegistry,
    fencing_token: &str,
) -> Result<(), scheduler_storage::DbErr> {
    let _expired = workflows.clear_expired_dispatch_queue_leases().await?;
    let _ = workflows
        .materialize_next_queued_node_with_fencing(
            DISPATCHER_LEASE_OWNER,
            DISPATCH_LEASE_SECONDS,
            fencing_token,
        )
        .await?;
    dispatch_single_instances(jobs, instances, workflows, registry, fencing_token).await?;
    dispatch_broadcast_attempts(jobs, instances, attempts, workflows, registry).await
}

async fn dispatch_single_instances(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    workflows: &WorkflowRepository,
    registry: &WorkerRegistry,
    fencing_token: &str,
) -> Result<(), scheduler_storage::DbErr> {
    for _ in 0..DISPATCH_BATCH_SIZE {
        let Some(claim) = workflows
            .claim_next_job_queue_item_with_fencing(
                DISPATCHER_LEASE_OWNER,
                DISPATCH_LEASE_SECONDS,
                fencing_token,
            )
            .await?
        else {
            break;
        };
        let Some(instance_id) = claim.item.job_instance_id.clone() else {
            continue;
        };
        let Some(instance) = instances.get(&instance_id).await? else {
            let _ = workflows
                .release_dispatch_queue_item(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            continue;
        };
        if !instances.claim_pending_for_dispatch(&instance.id).await? {
            let _ = workflows
                .release_dispatch_queue_item(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            continue;
        }
        let Some(job) = jobs.get(&instance.job_id).await? else {
            let _ = workflows
                .release_dispatch_queue_item(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            instances
                .update_status(&instance.id, InstanceStatus::Pending)
                .await?;
            continue;
        };

        let task = DispatchTask {
            instance_id: instance.id.clone(),
            job_id: instance.job_id.clone(),
            payload: Vec::new(),
            processor_name: resolve_processor_name(workflows, &instance.id, &job).await?,
        };

        let eligible_workers = registry
            .find_eligible_workers(&job.namespace, &job.app)
            .await;
        if let Some(worker_id) = eligible_workers.first()
            && let Some(worker_id) = registry.dispatch_to_worker(worker_id, task).await
        {
            instances
                .update_status(&instance.id, InstanceStatus::Running)
                .await?;
            let _ = workflows
                .mark_dispatch_queue_running(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            debug!(%worker_id, instance_id = %instance.id, "dispatched instance to worker");
        } else {
            let _ = workflows
                .release_dispatch_queue_item(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            instances
                .update_status(&instance.id, InstanceStatus::Pending)
                .await?;
        }
    }

    Ok(())
}

async fn dispatch_broadcast_attempts(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    workflows: &WorkflowRepository,
    registry: &WorkerRegistry,
) -> Result<(), scheduler_storage::DbErr> {
    let pending = attempts.list_pending(DISPATCH_BATCH_SIZE).await?;

    for attempt in pending {
        let Some(instance) = instances.get(&attempt.instance_id).await? else {
            continue;
        };
        let processor_name = if let Some(job) = jobs.get(&instance.job_id).await? {
            resolve_processor_name(workflows, &instance.id, &job).await?
        } else {
            instance.job_id.clone()
        };
        let task = DispatchTask {
            instance_id: attempt.instance_id.clone(),
            job_id: instance.job_id.clone(),
            payload: Vec::new(),
            processor_name,
        };

        if let Some(worker_id) = registry.dispatch_to_worker(&attempt.worker_id, task).await {
            attempts
                .update_status(
                    &attempt.instance_id,
                    &attempt.worker_id,
                    InstanceStatus::Running,
                )
                .await?;
            instances
                .update_status(&attempt.instance_id, InstanceStatus::Running)
                .await?;
            debug!(%worker_id, instance_id = %attempt.instance_id, "dispatched broadcast attempt to worker");
        }
    }

    Ok(())
}

async fn resolve_processor_name(
    workflows: &WorkflowRepository,
    instance_id: &str,
    job: &scheduler_storage::JobSummary,
) -> Result<String, scheduler_storage::DbErr> {
    if let Some(processor_name) = workflows
        .processor_name_for_job_instance(instance_id)
        .await?
    {
        return Ok(processor_name);
    }
    Ok(job
        .processor_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&job.id)
        .to_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::cluster::{ClusterMode, ClusterRole, ClusterStatus, StaticCoordinator};
    use scheduler_core::{ExecutionMode, InstanceStatus, TriggerType};
    use scheduler_proto::worker::v1::{RegisterWorker, server_message};
    use scheduler_storage::{
        CreateJob, CreateJobInstance, JobInstanceAttemptRepository, JobInstanceRepository,
        JobRepository, WorkflowRepository, connect_and_migrate,
    };
    use tokio::sync::mpsc;

    use super::{WorkerRegistry, dispatch_once, dispatch_once_if_owner};

    #[tokio::test]
    async fn dispatch_once_sends_pending_instance_to_registered_worker() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: Some("billing.manual".to_owned()),
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));
        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(1);
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &workflows,
            &registry,
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, instance.id);
                assert_eq!(task.processor_name, "billing.manual");
            }
            other => panic!("unexpected server message: {other:?}"),
        }

        let updated = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Running);
    }

    #[tokio::test]
    async fn dispatch_once_filters_by_namespace_and_app() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));
        let registry = WorkerRegistry::default();
        let (tx1, mut rx1) = mpsc::channel(1);
        let (tx2, _rx2) = mpsc::channel(1);

        // This worker should match
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    labels: HashMap::default(),
                },
                tx1,
            )
            .await;

        // This worker should NOT match
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-2".to_owned(),
                    app: "analytics".to_owned(), // Different app
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    labels: HashMap::default(),
                },
                tx2,
            )
            .await;

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &workflows,
            &registry,
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let message = rx1
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker-1 should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, instance.id);
                assert_eq!(task.processor_name, job.id);
            }
            other => panic!("unexpected server message: {other:?}"),
        }

        let updated = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Running);
    }

    #[tokio::test]
    async fn follower_dispatch_does_not_claim_queue_items() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "follower-dispatch".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));
        let registry = WorkerRegistry::default();
        let follower = StaticCoordinator::shared(ClusterStatus {
            mode: ClusterMode::Raft,
            role: ClusterRole::Follower,
            node_id: "node-b".to_owned(),
            nodes: 3,
            can_schedule: false,
            leader_fencing_token: None,
            detail: "test follower".to_owned(),
        });

        dispatch_once_if_owner(
            &jobs, &instances, &attempts, &workflows, &registry, &follower,
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch gate should run: {error}"));

        let overview = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue should load: {error}"));
        assert_eq!(overview.pending, 1);
        assert_eq!(overview.running, 0);
        assert!(overview.items[0].lease_owner.is_none());
    }

    #[tokio::test]
    async fn dispatch_once_prefers_workflow_node_processor_name() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: Some("job.default".to_owned()),
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let workflow = workflows
            .create_workflow(scheduler_storage::CreateWorkflow {
                name: "processor override".to_owned(),
                created_by: "test".to_owned(),
                definition: scheduler_storage::WorkflowDefinition {
                    nodes: vec![scheduler_storage::WorkflowNodeSpec {
                        key: "job-a".to_owned(),
                        name: Some("Job A".to_owned()),
                        kind: Some("job".to_owned()),
                        job_id: Some(job.id.clone()),
                        processor_name: Some("workflow.override".to_owned()),
                        child_workflow_id: None,
                        map_items: None,
                        config: None,
                    }],
                    edges: Vec::new(),
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"));
        workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("workflow node should materialize: {error}"));

        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(1);
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &workflows,
            &registry,
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.processor_name, "workflow.override");
            }
            other => panic!("unexpected server message: {other:?}"),
        }
    }
}
