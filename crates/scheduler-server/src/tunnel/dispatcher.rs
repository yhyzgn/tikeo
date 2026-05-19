//! Minimal pending-instance dispatcher for Worker Tunnel sessions.

use std::time::Duration;

use scheduler_core::InstanceStatus;
use scheduler_proto::worker::v1::DispatchTask;
use scheduler_storage::JobInstanceRepository;
use tokio::time;
use tracing::{debug, warn};

use super::WorkerRegistry;

const DISPATCH_INTERVAL: Duration = Duration::from_millis(500);
const DISPATCH_BATCH_SIZE: u64 = 16;

/// Run the minimal single-node dispatch loop forever.
pub async fn run(instances: JobInstanceRepository, registry: WorkerRegistry) {
    let mut ticker = time::interval(DISPATCH_INTERVAL);
    loop {
        ticker.tick().await;
        if let Err(error) = dispatch_once(&instances, &registry).await {
            warn!(%error, "worker dispatch iteration failed");
        }
    }
}

async fn dispatch_once(
    instances: &JobInstanceRepository,
    registry: &WorkerRegistry,
) -> Result<(), scheduler_storage::DbErr> {
    let pending = instances.list_pending(DISPATCH_BATCH_SIZE).await?;

    for instance in pending {
        let task = DispatchTask {
            instance_id: instance.id.clone(),
            job_id: instance.job_id.clone(),
            payload: Vec::new(),
        };

        if let Some(worker_id) = registry.dispatch_to_first(task).await {
            instances
                .update_status(&instance.id, InstanceStatus::Running)
                .await?;
            debug!(%worker_id, instance_id = %instance.id, "dispatched instance to worker");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use scheduler_core::{InstanceStatus, TriggerType};
    use scheduler_proto::worker::v1::{RegisterWorker, server_message};
    use scheduler_storage::{
        CreateJob, CreateJobInstance, JobInstanceRepository, JobRepository, connect_and_migrate,
    };
    use tokio::sync::mpsc;

    use super::{WorkerRegistry, dispatch_once};

    #[tokio::test]
    async fn dispatch_once_sends_pending_instance_to_registered_worker() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db);
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

        dispatch_once(&instances, &registry)
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
}
