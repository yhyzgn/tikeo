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

const DISPATCH_INTERVAL: Duration = Duration::from_millis(500);
const DISPATCH_BATCH_SIZE: u64 = 16;

/// Run the minimal single-node dispatch loop forever.
pub async fn run(
    jobs: JobRepository,
    instances: JobInstanceRepository,
    attempts: JobInstanceAttemptRepository,
    workflows: WorkflowRepository,
    registry: WorkerRegistry,
) {
    let mut ticker = time::interval(DISPATCH_INTERVAL);
    loop {
        ticker.tick().await;
        if let Err(error) = dispatch_once(&jobs, &instances, &attempts, &workflows, &registry).await
        {
            warn!(%error, "worker dispatch iteration failed");
        }
    }
}

async fn dispatch_once(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    workflows: &WorkflowRepository,
    registry: &WorkerRegistry,
) -> Result<(), scheduler_storage::DbErr> {
    let _ = workflows.materialize_next_queued_node().await?;
    dispatch_single_instances(jobs, instances, registry).await?;
    dispatch_broadcast_attempts(instances, attempts, registry).await
}

async fn dispatch_single_instances(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    registry: &WorkerRegistry,
) -> Result<(), scheduler_storage::DbErr> {
    let pending = instances.list_pending_single(DISPATCH_BATCH_SIZE).await?;

    for instance in pending {
        let Some(job) = jobs.get(&instance.job_id).await? else {
            continue;
        };

        let task = DispatchTask {
            instance_id: instance.id.clone(),
            job_id: instance.job_id.clone(),
            payload: Vec::new(),
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
            debug!(%worker_id, instance_id = %instance.id, "dispatched instance to worker");
        }
    }

    Ok(())
}

async fn dispatch_broadcast_attempts(
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    registry: &WorkerRegistry,
) -> Result<(), scheduler_storage::DbErr> {
    let pending = attempts.list_pending(DISPATCH_BATCH_SIZE).await?;

    for attempt in pending {
        let Some(instance) = instances.get(&attempt.instance_id).await? else {
            continue;
        };
        let task = DispatchTask {
            instance_id: attempt.instance_id.clone(),
            job_id: instance.job_id.clone(),
            payload: Vec::new(),
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use scheduler_core::{ExecutionMode, InstanceStatus, TriggerType};
    use scheduler_proto::worker::v1::{RegisterWorker, server_message};
    use scheduler_storage::{
        CreateJob, CreateJobInstance, JobInstanceAttemptRepository, JobInstanceRepository,
        JobRepository, WorkflowRepository, connect_and_migrate,
    };
    use tokio::sync::mpsc;

    use super::{WorkerRegistry, dispatch_once};

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
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
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
                    worker_id: "worker-1".to_owned(),
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

        dispatch_once(&jobs, &instances, &attempts, &workflows, &registry)
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
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
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
                    worker_id: "worker-1".to_owned(),
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
                    worker_id: "worker-2".to_owned(),
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

        dispatch_once(&jobs, &instances, &attempts, &workflows, &registry)
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
}
