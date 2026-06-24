use std::collections::HashMap;

use sha2::{Digest, Sha256};
use tikeo_storage::WorkerSessionSnapshotUpdate;
use uuid::Uuid;

use super::{
    RegisteredWorker, WorkerSessionStatus, registry_capabilities::worker_capabilities_json,
};

pub(super) fn logical_instance_id(
    namespace: &str,
    app: &str,
    cluster: &str,
    region: &str,
    client_instance_id: Option<&str>,
    worker_id: &str,
) -> String {
    let instance = client_instance_id.unwrap_or(worker_id);
    [namespace, app, cluster, region, instance].join("/")
}

pub(super) fn stable_worker_id(
    namespace: &str,
    app: &str,
    cluster: &str,
    region: &str,
    client_instance_id: Option<&str>,
) -> String {
    if let Some(client_instance_id) = client_instance_id {
        let digest = Sha256::digest(
            [namespace, app, cluster, region, client_instance_id]
                .join("/")
                .as_bytes(),
        );
        return format!("wrk-stable-{digest:x}");
    }
    format!("wrk-{}", Uuid::now_v7())
}

pub(super) fn session_snapshots<'a>(
    workers: impl IntoIterator<Item = &'a RegisteredWorker>,
) -> Vec<WorkerSessionSnapshotUpdate> {
    workers
        .into_iter()
        .filter(|worker| worker.is_current())
        .map(|worker| WorkerSessionSnapshotUpdate {
            worker_id: worker.worker_id.clone(),
            capabilities_json: json_or_empty_array(&worker.capabilities),
            structured_capabilities_json: worker_capabilities_json(Some(
                &worker.structured_capabilities,
            )),
            labels_json: json_or_empty_object(&worker.labels),
            master_json: json_or_empty_object(&worker.master),
        })
        .collect()
}

pub(super) fn next_generation(
    workers: &HashMap<String, RegisteredWorker>,
    logical_instance_id: &str,
) -> u64 {
    workers
        .values()
        .filter(|worker| worker.logical_instance_id == logical_instance_id)
        .map(|worker| worker.generation)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

pub(super) fn replace_previous_generations(
    workers: &mut HashMap<String, RegisteredWorker>,
    logical_instance_id: &str,
    replacement_worker_id: &str,
) {
    for worker in workers
        .values_mut()
        .filter(|worker| worker.logical_instance_id == logical_instance_id && worker.is_current())
    {
        worker.status = WorkerSessionStatus::Replaced;
        worker.status_reason = Some("replaced_by_new_generation".to_owned());
        worker.status_evidence =
            Some("same logical instance registered a newer generation".to_owned());
        worker.replaced_by_worker_id = Some(replacement_worker_id.to_owned());
    }
}

pub(super) fn empty_to_none(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn json_or_empty_array<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_owned())
}

fn json_or_empty_object<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_owned())
}
