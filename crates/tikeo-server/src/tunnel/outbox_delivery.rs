//! Durable Worker dispatch outbox delivery loop.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use tikeo_proto::worker::v1::DispatchTask;
use tikeo_storage::{WorkerDispatchOutboxRepository, WorkerDispatchOutboxSummary};
use tokio::time::{self, Duration};
use tonic_prost::prost::Message as _;
use tracing::{debug, info, warn};

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
    info!(gateway_node_id = %gateway_node_id, interval_ms = OUTBOX_DELIVERY_INTERVAL.as_millis(), "starting worker dispatch outbox delivery loop");
    loop {
        ticker.tick().await;
        if let Err(error) = outbox
            .requeue_expired_delivered(OUTBOX_DELIVERY_RETRY_SECONDS)
            .await
        {
            warn!(%error, %gateway_node_id, "worker dispatch outbox visibility scan failed");
        }
        if let Err(error) = reroute_stale_logical_rows(&outbox, &registry).await {
            warn!(%error, %gateway_node_id, "worker dispatch outbox reroute scan failed");
        }
        if let Err(error) = deliver_once(&outbox, &registry, &gateway_node_id).await {
            warn!(%error, %gateway_node_id, "worker dispatch outbox delivery iteration failed");
        }
    }
}

/// Reroute non-terminal rows when the same logical Worker has a newer online session.
///
/// This is intentionally global rather than gateway-local: when a gateway pod is
/// hard-killed, that old gateway cannot scan its own rows. Any surviving Server
/// may observe the newer logical Worker session and move durable outbox rows to
/// the new gateway before delivery resumes.
///
/// # Errors
///
/// Returns an error when repository access fails.
pub async fn reroute_stale_logical_rows(
    outbox: &WorkerDispatchOutboxRepository,
    registry: &WorkerRegistry,
) -> Result<u64, tikeo_storage::DbErr> {
    let mut moved = 0_u64;
    for row in outbox.list_reroute_candidates(100).await? {
        if let Some(current) = registry
            .current_logical_dispatch_target(&row.logical_instance_id)
            .await
            && current.generation > row.gateway_generation
            && outbox
                .reroute(
                    &row.id,
                    &current.gateway_node_id,
                    &current.worker_id,
                    current.generation,
                )
                .await?
                .is_some()
        {
            debug!(outbox_id = %row.id, worker_id = %current.worker_id, gateway_node_id = %current.gateway_node_id, "rerouted worker dispatch outbox row to current worker session");
            moved = moved.saturating_add(1);
        }
    }
    Ok(moved)
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
        debug!(outbox_id = %row.id, worker_id = %row.worker_id, gateway_node_id = %gateway_node_id, "delivered worker dispatch outbox row to local worker");
        return outbox
            .mark_delivered(&row.id, OUTBOX_DELIVERY_VISIBILITY_SECONDS)
            .await;
    }
    if let Some(current) = registry
        .current_logical_dispatch_target(&row.logical_instance_id)
        .await
        && current.generation > row.gateway_generation
    {
        return outbox
            .reroute(
                &row.id,
                &current.gateway_node_id,
                &current.worker_id,
                current.generation,
            )
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
                shard_map_version: 1,
                shard_count: 64,
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

#[cfg(test)]
mod reroute_tests {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
    use tikeo_proto::worker::v1::DispatchTask;
    use tikeo_storage::{
        CreateWorkerDispatchOutbox, RegisterWorkerSession, WorkerDispatchOutboxRepository,
        WorkerLifecycleRepository, connect_and_migrate,
    };
    use tonic_prost::prost::Message as _;

    use super::{deliver_once, reroute_stale_logical_rows};
    use crate::tunnel::WorkerRegistry;

    #[tokio::test]
    async fn global_reroute_scan_moves_rows_when_old_gateway_is_gone() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let lifecycle = WorkerLifecycleRepository::new(db.clone());
        let old = lifecycle
            .register_session(RegisterWorkerSession {
                worker_id: "worker-old-hard-killed-gateway".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "worker-global-reroute".to_owned(),
                connection_id: "conn-old".to_owned(),
                gateway_node_id: "gateway-hard-killed".to_owned(),
                fencing_token: "token-old".to_owned(),
                lease_seconds: 30,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("old worker should register: {error}"));
        let new = lifecycle
            .register_session(RegisterWorkerSession {
                worker_id: "worker-new-surviving-gateway".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "worker-global-reroute".to_owned(),
                connection_id: "conn-new".to_owned(),
                gateway_node_id: "gateway-survivor".to_owned(),
                fencing_token: "token-new".to_owned(),
                lease_seconds: 30,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("new worker should register: {error}"));
        let outbox = WorkerDispatchOutboxRepository::new(db);
        let task = DispatchTask {
            instance_id: "inst-global-reroute".to_owned(),
            job_id: "job-global-reroute".to_owned(),
            assignment_token: "asg-global-reroute".to_owned(),
            ..DispatchTask::default()
        };
        let created = outbox
            .create(CreateWorkerDispatchOutbox {
                instance_id: "inst-global-reroute".to_owned(),
                attempt_id: "attempt-global-reroute".to_owned(),
                worker_id: old.worker_id,
                logical_instance_id: old.logical_instance_id,
                gateway_node_id: "gateway-hard-killed".to_owned(),
                gateway_generation: old.generation,
                assignment_token: "asg-global-reroute".to_owned(),
                dispatch_payload: BASE64_STANDARD.encode(task.encode_to_vec()),
                shard_id: 0,
                shard_map_version: 1,
                shard_count: 64,
                owner_node_id: "owner".to_owned(),
                owner_epoch: 0,
                owner_fencing_token: "fence".to_owned(),
                next_delivery_at: None,
            })
            .await
            .unwrap_or_else(|error| panic!("outbox should create: {error}"));

        let moved = reroute_stale_logical_rows(&outbox, &WorkerRegistry::with_lifecycle(lifecycle))
            .await
            .unwrap_or_else(|error| panic!("reroute scan should not fail: {error}"));

        assert_eq!(moved, 1);
        let updated = outbox
            .get(&created.id)
            .await
            .unwrap_or_else(|error| panic!("outbox get should not fail: {error}"))
            .unwrap_or_else(|| panic!("outbox row should exist"));
        assert_eq!(updated.status, "queued");
        assert_eq!(updated.worker_id, new.worker_id);
        assert_eq!(updated.gateway_node_id, "gateway-survivor");
        assert_eq!(updated.gateway_generation, new.generation);
    }

    #[tokio::test]
    async fn global_reroute_scan_moves_acked_rows_when_result_channel_is_lost() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let lifecycle = WorkerLifecycleRepository::new(db.clone());
        let old = lifecycle
            .register_session(RegisterWorkerSession {
                worker_id: "worker-old-acked-gateway".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "worker-acked-reroute".to_owned(),
                connection_id: "conn-old".to_owned(),
                gateway_node_id: "gateway-old-acked".to_owned(),
                fencing_token: "token-old".to_owned(),
                lease_seconds: 30,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("old worker should register: {error}"));
        let new = lifecycle
            .register_session(RegisterWorkerSession {
                worker_id: "worker-new-acked-gateway".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "worker-acked-reroute".to_owned(),
                connection_id: "conn-new".to_owned(),
                gateway_node_id: "gateway-new-acked".to_owned(),
                fencing_token: "token-new".to_owned(),
                lease_seconds: 30,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("new worker should register: {error}"));
        let outbox = WorkerDispatchOutboxRepository::new(db);
        let task = DispatchTask {
            instance_id: "inst-acked-reroute".to_owned(),
            job_id: "job-acked-reroute".to_owned(),
            assignment_token: "asg-acked-reroute".to_owned(),
            ..DispatchTask::default()
        };
        let created = outbox
            .create(CreateWorkerDispatchOutbox {
                instance_id: "inst-acked-reroute".to_owned(),
                attempt_id: "attempt-acked-reroute".to_owned(),
                worker_id: old.worker_id,
                logical_instance_id: old.logical_instance_id,
                gateway_node_id: "gateway-old-acked".to_owned(),
                gateway_generation: old.generation,
                assignment_token: "asg-acked-reroute".to_owned(),
                dispatch_payload: BASE64_STANDARD.encode(task.encode_to_vec()),
                shard_id: 0,
                shard_map_version: 1,
                shard_count: 64,
                owner_node_id: "owner".to_owned(),
                owner_epoch: 0,
                owner_fencing_token: "fence".to_owned(),
                next_delivery_at: None,
            })
            .await
            .unwrap_or_else(|error| panic!("outbox should create: {error}"));
        outbox
            .mark_hint_delivered(&created.id, 30)
            .await
            .unwrap_or_else(|error| panic!("outbox should mark delivered: {error}"));
        assert!(
            outbox
                .mark_acked_by_assignment(
                    "inst-acked-reroute",
                    "worker-old-acked-gateway",
                    "asg-acked-reroute",
                )
                .await
                .unwrap_or_else(|error| panic!("outbox should ack: {error}"))
        );

        let moved = reroute_stale_logical_rows(&outbox, &WorkerRegistry::with_lifecycle(lifecycle))
            .await
            .unwrap_or_else(|error| panic!("reroute scan should not fail: {error}"));

        assert_eq!(moved, 1);
        let updated = outbox
            .get(&created.id)
            .await
            .unwrap_or_else(|error| panic!("outbox get should not fail: {error}"))
            .unwrap_or_else(|| panic!("outbox row should exist"));
        assert_eq!(updated.status, "queued");
        assert_eq!(updated.worker_id, new.worker_id);
        assert_eq!(updated.gateway_node_id, "gateway-new-acked");
        assert_eq!(updated.gateway_generation, new.generation);
    }

    #[tokio::test]
    async fn deliver_once_reroutes_outbox_when_worker_reconnected_to_new_gateway() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let lifecycle = WorkerLifecycleRepository::new(db.clone());
        let old = lifecycle
            .register_session(RegisterWorkerSession {
                worker_id: "worker-old-gateway".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "worker-reroute".to_owned(),
                connection_id: "conn-old".to_owned(),
                gateway_node_id: "gateway-old".to_owned(),
                fencing_token: "token-old".to_owned(),
                lease_seconds: 30,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("old worker should register: {error}"));
        let new = lifecycle
            .register_session(RegisterWorkerSession {
                worker_id: "worker-new-gateway".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "worker-reroute".to_owned(),
                connection_id: "conn-new".to_owned(),
                gateway_node_id: "gateway-new".to_owned(),
                fencing_token: "token-new".to_owned(),
                lease_seconds: 30,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("new worker should register: {error}"));
        let outbox = WorkerDispatchOutboxRepository::new(db);
        let task = DispatchTask {
            instance_id: "inst-reroute".to_owned(),
            job_id: "job-reroute".to_owned(),
            assignment_token: "asg-reroute".to_owned(),
            ..DispatchTask::default()
        };
        let created = outbox
            .create(CreateWorkerDispatchOutbox {
                instance_id: "inst-reroute".to_owned(),
                attempt_id: "attempt-reroute".to_owned(),
                worker_id: old.worker_id,
                logical_instance_id: old.logical_instance_id,
                gateway_node_id: "gateway-old".to_owned(),
                gateway_generation: old.generation,
                assignment_token: "asg-reroute".to_owned(),
                dispatch_payload: BASE64_STANDARD.encode(task.encode_to_vec()),
                shard_id: 0,
                shard_map_version: 1,
                shard_count: 64,
                owner_node_id: "owner".to_owned(),
                owner_epoch: 0,
                owner_fencing_token: "fence".to_owned(),
                next_delivery_at: None,
            })
            .await
            .unwrap_or_else(|error| panic!("outbox should create: {error}"));

        let delivered = deliver_once(
            &outbox,
            &WorkerRegistry::with_lifecycle(lifecycle),
            "gateway-old",
        )
        .await
        .unwrap_or_else(|error| panic!("delivery should not fail: {error}"))
        .unwrap_or_else(|| panic!("outbox row should be rerouted"));

        assert_eq!(delivered.id, created.id);
        assert_eq!(delivered.status, "queued");
        assert_eq!(delivered.worker_id, new.worker_id);
        assert_eq!(delivered.gateway_node_id, "gateway-new");
        assert_eq!(delivered.gateway_generation, new.generation);
    }
}
