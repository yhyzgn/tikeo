use std::collections::HashSet;

use sha2::{Digest, Sha256};
use tikeo_storage::PersistedOnlineWorkerSummary;

use super::{
    BroadcastSelector, RegisteredWorker,
    registry_capabilities::{parse_persisted_capabilities, parse_persisted_labels},
};
use crate::tunnel::capability::{WorkerRequirement, structured_capabilities_match};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LassoDispatchScore {
    local_gateway: bool,
    worker_master: bool,
    master_domain: String,
    spread_score: u64,
    worker_id: String,
}

pub(in crate::tunnel) fn is_match(worker_val: &str, job_val: &str) -> bool {
    worker_val == job_val
        || worker_val == "*"
        || worker_val.is_empty()
        || job_val == "*"
        || job_val.is_empty()
}

pub(super) fn broadcast_selector_matches(
    worker: &RegisteredWorker,
    selector: &BroadcastSelector,
) -> bool {
    if selector
        .region
        .as_deref()
        .is_some_and(|region| !is_match(&worker.region, region))
    {
        return false;
    }
    if selector
        .cluster
        .as_deref()
        .is_some_and(|cluster| !is_match(&worker.cluster, cluster))
    {
        return false;
    }
    if !selector
        .labels
        .iter()
        .all(|(key, value)| worker.labels.get(key).is_some_and(|actual| actual == value))
    {
        return false;
    }
    if selector.tags.is_empty() {
        return true;
    }
    let tags: HashSet<&str> = worker
        .structured_capabilities
        .tags
        .iter()
        .map(String::as_str)
        .collect();
    selector.tags.iter().all(|tag| tags.contains(tag.as_str()))
}

pub(super) fn worker_satisfies(worker: &RegisteredWorker, requirement: &WorkerRequirement) -> bool {
    structured_capabilities_match(&worker.structured_capabilities, requirement)
}

pub(super) fn persisted_lasso_dispatch_score(
    worker: &PersistedOnlineWorkerSummary,
    local_gateway_node_id: &str,
    dispatch_key: &str,
) -> LassoDispatchScore {
    let (worker_master, master_domain) = persisted_master_order(worker);
    LassoDispatchScore {
        local_gateway: worker.gateway_node_id == local_gateway_node_id,
        worker_master,
        master_domain,
        spread_score: rendezvous_spread_score(dispatch_key, &worker.worker_id),
        worker_id: worker.worker_id.clone(),
    }
}

pub(super) fn registered_lasso_dispatch_score(
    worker: &RegisteredWorker,
    dispatch_key: &str,
) -> LassoDispatchScore {
    LassoDispatchScore {
        local_gateway: true,
        worker_master: worker.master.is_master,
        master_domain: worker.master.domain.clone(),
        spread_score: rendezvous_spread_score(dispatch_key, &worker.worker_id),
        worker_id: worker.worker_id.clone(),
    }
}

impl Ord for LassoDispatchScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .local_gateway
            .cmp(&self.local_gateway)
            .then_with(|| other.worker_master.cmp(&self.worker_master))
            .then_with(|| self.master_domain.cmp(&other.master_domain))
            .then_with(|| other.spread_score.cmp(&self.spread_score))
            .then_with(|| self.worker_id.cmp(&other.worker_id))
    }
}

impl PartialOrd for LassoDispatchScore {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn persisted_master_order(worker: &PersistedOnlineWorkerSummary) -> (bool, String) {
    let value = serde_json::from_str::<serde_json::Value>(&worker.master_json).unwrap_or_default();
    (
        value
            .get("isMaster")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        value
            .get("domain")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned(),
    )
}

fn rendezvous_spread_score(dispatch_key: &str, worker_id: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(dispatch_key.as_bytes());
    hasher.update([0]);
    hasher.update(worker_id.as_bytes());
    let digest = hasher.finalize();
    let mut prefix = [0_u8; 8];
    prefix.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(prefix)
}

pub(super) fn persisted_worker_matches(
    worker: &PersistedOnlineWorkerSummary,
    namespace: &str,
    app: &str,
    requirement: Option<&WorkerRequirement>,
) -> bool {
    is_match(&worker.namespace_name, namespace)
        && is_match(&worker.app_name, app)
        && requirement.is_none_or(|requirement| persisted_worker_satisfies(worker, requirement))
}

pub(super) fn persisted_broadcast_worker_matches(
    worker: &PersistedOnlineWorkerSummary,
    namespace: &str,
    app: &str,
    selector: &BroadcastSelector,
) -> bool {
    if !is_match(&worker.namespace_name, namespace) || !is_match(&worker.app_name, app) {
        return false;
    }
    if selector
        .region
        .as_deref()
        .is_some_and(|region| !is_match(&worker.region, region))
    {
        return false;
    }
    if selector
        .cluster
        .as_deref()
        .is_some_and(|cluster| !is_match(&worker.cluster, cluster))
    {
        return false;
    }
    let labels = parse_persisted_labels(&worker.labels_json);
    if !selector
        .labels
        .iter()
        .all(|(key, value)| labels.get(key).is_some_and(|actual| actual == value))
    {
        return false;
    }
    if selector.tags.is_empty() {
        return true;
    }
    let capabilities = parse_persisted_capabilities(&worker.structured_capabilities_json);
    let tags = capabilities
        .tags
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    selector.tags.iter().all(|tag| tags.contains(tag.as_str()))
}

pub(super) fn persisted_worker_satisfies(
    worker: &PersistedOnlineWorkerSummary,
    requirement: &WorkerRequirement,
) -> bool {
    structured_capabilities_match(
        &parse_persisted_capabilities(&worker.structured_capabilities_json),
        requirement,
    )
}
