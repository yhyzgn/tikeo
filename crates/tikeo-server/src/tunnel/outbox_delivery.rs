//! Durable Worker dispatch outbox delivery loop.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use tikeo_proto::worker::v1::DispatchTask;
use tikeo_storage::{WorkerDispatchOutboxRepository, WorkerDispatchOutboxSummary};
use tokio::time::{self, Duration};
use tonic_prost::prost::Message as _;
use tracing::warn;

use super::WorkerRegistry;

const OUTBOX_DELIVERY_VISIBILITY_SECONDS: i64 = 30;
const OUTBOX_DELIVERY_RETRY_SECONDS: i64 = 2;
const OUTBOX_DELIVERY_INTERVAL: Duration = Duration::from_millis(200);

/// Decode a base64 protobuf `DispatchTask` payload stored in the outbox.
fn decode_dispatch_payload(payload: &str) -> Result<DispatchTask, String> {
    let bytes = BASE64_STANDARD
        .decode(payload)
        .map_err(|error| format!("invalid dispatch payload base64: {error}"))?;
    DispatchTask::decode(bytes.as_slice())
        .map_err(|error| format!("invalid dispatch payload protobuf: {error}"))
}

/// Run the durable outbox gateway delivery loop forever.
pub async fn run(
    outbox: WorkerDispatchOutboxRepository,
    registry: WorkerRegistry,
    gateway_node_id: String,
) {
    let mut ticker = time::interval(OUTBOX_DELIVERY_INTERVAL);
    loop {
        ticker.tick().await;
        if let Err(error) = deliver_once(&outbox, &registry, &gateway_node_id).await {
            warn!(%error, %gateway_node_id, "worker dispatch outbox delivery iteration failed");
        }
    }
}

/// Deliver at most one queued outbox row for this gateway node.
///
/// # Errors
///
/// Returns an error when repository access fails.
pub async fn deliver_once(
    outbox: &WorkerDispatchOutboxRepository,
    registry: &WorkerRegistry,
    gateway_node_id: &str,
) -> Result<Option<WorkerDispatchOutboxSummary>, tikeo_storage::DbErr> {
    let Some(row) = outbox.claim_next_for_gateway(gateway_node_id, 1).await? else {
        return Ok(None);
    };
    let task = match decode_dispatch_payload(&row.dispatch_payload) {
        Ok(task) => task,
        Err(error) => {
            warn!(outbox_id = %row.id, %error, "worker dispatch outbox payload decode failed");
            return outbox
                .mark_delivery_failed(&row.id, &error, OUTBOX_DELIVERY_RETRY_SECONDS)
                .await;
        }
    };
    if registry
        .dispatch_relayed_task_to_local_worker(&row.worker_id, task)
        .await
    {
        return outbox
            .mark_delivered(&row.id, OUTBOX_DELIVERY_VISIBILITY_SECONDS)
            .await;
    }
    outbox
        .mark_delivery_failed(
            &row.id,
            "local worker stream is not currently schedulable",
            OUTBOX_DELIVERY_RETRY_SECONDS,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tikeo_proto::worker::v1::{RegisterWorker, WorkerCapabilities, server_message};
    use tikeo_storage::{CreateWorkerDispatchOutbox, connect_and_migrate};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn deliver_once_sends_queued_outbox_to_local_worker_and_marks_delivered() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let outbox = WorkerDispatchOutboxRepository::new(db);
        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(4);
        let worker = registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-local".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    labels: HashMap::default(),
                    structured_capabilities: Some(WorkerCapabilities::default()),
                    election: None,
                },
                tx,
            )
            .await;
        let task = DispatchTask {
            instance_id: "inst-deliver".to_owned(),
            job_id: "job-deliver".to_owned(),
            assignment_token: "asg-deliver".to_owned(),
            ..DispatchTask::default()
        };
        let payload = BASE64_STANDARD.encode(task.encode_to_vec());
        let row = outbox
            .create(CreateWorkerDispatchOutbox {
                instance_id: "inst-deliver".to_owned(),
                attempt_id: "attempt-deliver".to_owned(),
                worker_id: worker.worker_id.clone(),
                logical_instance_id: worker.worker_id.clone(),
                gateway_node_id: "standalone".to_owned(),
                gateway_generation: 0,
                assignment_token: "asg-deliver".to_owned(),
                dispatch_payload: payload,
                shard_id: 0,
                owner_node_id: "owner".to_owned(),
                owner_epoch: 0,
                owner_fencing_token: "fence".to_owned(),
                next_delivery_at: None,
            })
            .await
            .unwrap_or_else(|error| panic!("outbox should create: {error}"));

        let delivered = deliver_once(&outbox, &registry, "standalone")
            .await
            .unwrap_or_else(|error| panic!("delivery should not fail: {error}"))
            .unwrap_or_else(|| panic!("outbox row should deliver"));

        assert_eq!(delivered.id, row.id);
        assert_eq!(delivered.status, "delivered");
        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker should receive dispatch"))
            .unwrap_or_else(|error| panic!("worker stream should not error: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, "inst-deliver");
                assert_eq!(task.assignment_token, "asg-deliver");
            }
            other => panic!("expected dispatch task, got {other:?}"),
        }
    }
}
