//! Worker Tunnel transport registry.
//!
//! Persistent worker lifecycle storage is the authority for online state, capability
//! snapshots, and dispatch eligibility. This registry intentionally keeps only the
//! per-process live stream handles required to send gRPC messages.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tikeo_proto::worker::v1::{
    DispatchTask, RegisterWorker, ServerMessage, WorkerCapabilities, WorkerClusterElection,
    server_message,
};
use tikeo_storage::{
    PersistedOnlineWorkerSummary, RegisterWorkerSession, WorkerHeartbeat,
    WorkerLifecycleRepository, WorkerSessionSnapshotUpdate,
};
use tokio::sync::{RwLock, mpsc};
use tonic::Status;
use uuid::Uuid;

use super::{
    capability::{WorkerRequirement, structured_capabilities_match},
    relay::SharedWorkerRelayDispatch,
};

const DEFAULT_LEASE_SECONDS: u64 = 30;

/// Shared worker registry handle.
#[derive(Debug, Clone)]
pub struct WorkerRegistry {
    workers: Arc<RwLock<HashMap<String, RegisteredWorker>>>,
    lifecycle: Option<WorkerLifecycleRepository>,
    gateway_node_id: String,
    relay: Option<SharedWorkerRelayDispatch>,
}

impl Default for WorkerRegistry {
    fn default() -> Self {
        Self {
            workers: Arc::new(RwLock::const_new(HashMap::new())),
            lifecycle: None,
            gateway_node_id: "standalone".to_owned(),
            relay: None,
        }
    }
}

/// Broadcast fan-out selector over connected worker metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BroadcastSelector {
    /// Required structured capability tags.
    pub tags: Vec<String>,
    /// Optional exact region match.
    pub region: Option<String>,
    /// Optional exact cluster/version match.
    pub cluster: Option<String>,
    /// Optional worker labels that must all match.
    pub labels: HashMap<String, String>,
}

/// Worker-side master election outcome for a namespace/app/cluster/region domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerMasterState {
    /// Deterministic election domain.
    pub domain: String,
    /// Whether this worker is the elected master for the domain.
    pub is_master: bool,
    /// Current elected master worker id, when one exists.
    pub master_worker_id: Option<String>,
    /// Monotonic-ish term derived from domain membership generations.
    pub term: u64,
    /// Fencing token bound to domain, term, and elected master.
    pub fencing_token: Option<String>,
}

impl WorkerMasterState {
    const fn follower(domain: String) -> Self {
        Self {
            domain,
            is_master: false,
            master_worker_id: None,
            term: 0,
            fencing_token: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkerElectionRegistration {
    enabled: bool,
    domain: String,
    priority: u32,
}

impl WorkerRegistry {
    /// Create a registry backed by persistent worker lifecycle storage.
    #[must_use]
    pub fn with_lifecycle(lifecycle: WorkerLifecycleRepository) -> Self {
        Self {
            workers: Arc::new(RwLock::const_new(HashMap::new())),
            lifecycle: Some(lifecycle),
            gateway_node_id: "standalone".to_owned(),
            relay: None,
        }
    }

    /// Bind this registry to the server node that owns its live Worker Tunnel streams.
    #[must_use]
    pub fn with_gateway_node_id(mut self, gateway_node_id: impl Into<String>) -> Self {
        let gateway_node_id = gateway_node_id.into();
        self.gateway_node_id = if gateway_node_id.trim().is_empty() {
            "standalone".to_owned()
        } else {
            gateway_node_id
        };
        self
    }

    /// Return the server node id owning this registry's live streams.
    #[must_use]
    pub fn gateway_node_id(&self) -> &str {
        &self.gateway_node_id
    }

    /// Attach the server-to-server relay used for workers connected through other Pods.
    #[must_use]
    pub fn with_relay(mut self, relay: SharedWorkerRelayDispatch) -> Self {
        self.relay = Some(relay);
        self
    }

    /// Register or replace a worker record.
    pub async fn register(
        &self,
        worker: RegisterWorker,
        outbound: mpsc::Sender<Result<ServerMessage, Status>>,
    ) -> RegisteredWorker {
        let now = SystemTime::now();
        let client_instance_id = empty_to_none(worker.client_instance_id.clone());
        let worker_id = stable_worker_id(
            &worker.namespace,
            &worker.app,
            &worker.cluster,
            &worker.region,
            client_instance_id.as_deref(),
        );
        let logical_instance_id = logical_instance_id(
            &worker.namespace,
            &worker.app,
            &worker.cluster,
            &worker.region,
            client_instance_id.as_deref(),
            &worker_id,
        );
        let connection_id = format!("conn-{}", Uuid::now_v7());
        let fencing_token = format!("wft-{}", Uuid::now_v7());
        let persisted_generation = self
            .persist_registration(&worker, &worker_id, &connection_id, &fencing_token)
            .await;
        let election = worker_election_registration(&worker);
        let master = WorkerMasterState::follower(election.domain.clone());
        let (record, worker_count, snapshots) = {
            let mut workers = self.workers.write().await;
            let generation = persisted_generation
                .unwrap_or_else(|| next_generation(&workers, &logical_instance_id));
            let record = RegisteredWorker {
                worker_id: worker_id.clone(),
                logical_instance_id: logical_instance_id.clone(),
                client_instance_id,
                app: worker.app,
                namespace: worker.namespace,
                cluster: worker.cluster,
                region: worker.region,
                capabilities: worker.capabilities,
                structured_capabilities: worker.structured_capabilities.clone().unwrap_or_default(),
                election,
                master,
                labels: worker.labels,
                outbound,
                generation,
                fencing_token,
                status: WorkerSessionStatus::Online,
                status_reason: None,
                status_evidence: None,
                lease_expires_at: now + Duration::from_secs(DEFAULT_LEASE_SECONDS),
                replaced_by_worker_id: None,
                registered_at: now,
                last_heartbeat_at: now,
                last_sequence: 0,
            };
            replace_previous_generations(&mut workers, &logical_instance_id, &worker_id);
            workers.insert(record.worker_id.clone(), record.clone());
            recompute_worker_master_states(&mut workers);
            let record = workers.get(&worker_id).cloned().unwrap_or(record);
            let worker_count = workers
                .values()
                .filter(|worker| worker.is_schedulable())
                .count();
            let snapshots = session_snapshots(workers.values());
            drop(workers);
            (record, worker_count, snapshots)
        };
        self.persist_worker_snapshots(snapshots).await;
        metrics::gauge!("tikeo_worker_connected_total")
            .set(u32::try_from(worker_count).map_or_else(|_| f64::from(u32::MAX), f64::from));

        record
    }

    async fn persist_registration(
        &self,
        worker: &RegisterWorker,
        worker_id: &str,
        connection_id: &str,
        fencing_token: &str,
    ) -> Option<u64> {
        let lifecycle = self.lifecycle.as_ref()?;
        let persisted = lifecycle
            .register_session(RegisterWorkerSession {
                worker_id: worker_id.to_owned(),
                namespace_name: worker.namespace.clone(),
                app_name: worker.app.clone(),
                cluster: worker.cluster.clone(),
                region: worker.region.clone(),
                client_instance_id: empty_to_none(worker.client_instance_id.clone())
                    .unwrap_or_else(|| worker_id.to_owned()),
                connection_id: connection_id.to_owned(),
                gateway_node_id: self.gateway_node_id().to_owned(),
                fencing_token: fencing_token.to_owned(),
                lease_seconds: i64::try_from(DEFAULT_LEASE_SECONDS).unwrap_or(i64::MAX),
                capabilities_json: json_or_empty_array(&worker.capabilities),
                structured_capabilities_json: worker_capabilities_json(
                    worker.structured_capabilities.as_ref(),
                ),
                labels_json: json_or_empty_object(&worker.labels),
                master_json: json_or_empty_object(&WorkerMasterState::follower(
                    worker_election_registration(worker).domain,
                )),
            })
            .await
            .ok()?;
        u64::try_from(persisted.generation).ok()
    }

    /// Record a heartbeat for a known worker.
    pub async fn heartbeat(
        &self,
        worker_id: &str,
        sequence: u64,
        generation: u64,
        fencing_token: &str,
    ) -> Option<RegisteredWorker> {
        let updated = self
            .record_in_memory_heartbeat(worker_id, sequence, generation, fencing_token)
            .await?;
        self.persist_heartbeat(&updated, sequence, generation, fencing_token)
            .await;
        self.persist_current_snapshots().await;
        Some(updated)
    }

    async fn record_in_memory_heartbeat(
        &self,
        worker_id: &str,
        sequence: u64,
        generation: u64,
        fencing_token: &str,
    ) -> Option<RegisteredWorker> {
        let now = SystemTime::now();
        let updated = {
            let mut workers = self.workers.write().await;
            let worker = workers.get_mut(worker_id)?;
            if !worker.accepts_heartbeat(generation, fencing_token) {
                metrics::counter!("tikeo_worker_stale_messages_total", "kind" => "heartbeat")
                    .increment(1);
                return None;
            }
            worker.last_heartbeat_at = now;
            worker.lease_expires_at = now + Duration::from_secs(DEFAULT_LEASE_SECONDS);
            worker.last_sequence = sequence;
            recompute_worker_master_states(&mut workers);
            let updated = workers.get(worker_id).cloned()?;
            drop(workers);
            updated
        };
        Some(updated)
    }

    async fn persist_heartbeat(
        &self,
        worker: &RegisteredWorker,
        sequence: u64,
        generation: u64,
        fencing_token: &str,
    ) {
        let Some(lifecycle) = self.lifecycle.as_ref() else {
            return;
        };
        let _ = lifecycle
            .heartbeat(WorkerHeartbeat {
                worker_id: worker.worker_id.clone(),
                generation: i64::try_from(generation).unwrap_or(i64::MAX),
                fencing_token: fencing_token.to_owned(),
                sequence: i64::try_from(sequence).unwrap_or(i64::MAX),
                lease_seconds: i64::try_from(DEFAULT_LEASE_SECONDS).unwrap_or(i64::MAX),
            })
            .await;
    }

    /// Gracefully unregister a current worker session.
    pub async fn unregister(
        &self,
        worker_id: &str,
        generation: u64,
        fencing_token: &str,
    ) -> Option<RegisteredWorker> {
        let stopped = self
            .stop_in_memory_worker(worker_id, generation, fencing_token)
            .await?;
        self.persist_unregister(&stopped, generation, fencing_token)
            .await;
        self.persist_current_snapshots().await;
        Some(stopped)
    }

    async fn stop_in_memory_worker(
        &self,
        worker_id: &str,
        generation: u64,
        fencing_token: &str,
    ) -> Option<RegisteredWorker> {
        let stopped = {
            let mut workers = self.workers.write().await;
            let worker = workers.get_mut(worker_id)?;
            if !worker.accepts_heartbeat(generation, fencing_token) {
                metrics::counter!("tikeo_worker_stale_messages_total", "kind" => "unregister")
                    .increment(1);
                return None;
            }
            worker.status = WorkerSessionStatus::Stopped;
            worker.status_reason = Some("graceful_shutdown".to_owned());
            worker.status_evidence = Some("worker sent graceful unregister".to_owned());
            recompute_worker_master_states(&mut workers);
            let stopped = workers.get(worker_id).cloned()?;
            drop(workers);
            stopped
        };
        Some(stopped)
    }

    async fn persist_unregister(
        &self,
        worker: &RegisteredWorker,
        generation: u64,
        fencing_token: &str,
    ) {
        let Some(lifecycle) = self.lifecycle.as_ref() else {
            return;
        };
        let _ = lifecycle
            .graceful_unregister(
                &worker.worker_id,
                i64::try_from(generation).unwrap_or(i64::MAX),
                fencing_token,
            )
            .await;
    }

    /// Mark a server-observed stream close/error for a current worker session.
    pub async fn mark_transport_error(
        &self,
        worker_id: &str,
        evidence: &str,
    ) -> Option<RegisteredWorker> {
        let offline = {
            let mut workers = self.workers.write().await;
            let worker = workers.get_mut(worker_id)?;
            if !worker.is_current() {
                return None;
            }
            worker.status = WorkerSessionStatus::Offline;
            worker.status_reason = Some("transport_error".to_owned());
            worker.status_evidence = Some(evidence.to_owned());
            recompute_worker_master_states(&mut workers);
            let offline = workers.get(worker_id).cloned()?;
            drop(workers);
            offline
        };
        if let Some(lifecycle) = self.lifecycle.as_ref() {
            let _ = lifecycle.mark_transport_error(worker_id, evidence).await;
        }
        self.persist_current_snapshots().await;
        Some(offline)
    }

    /// Return a worker by id.
    pub async fn get(&self, worker_id: &str) -> Option<RegisteredWorker> {
        self.workers.read().await.get(worker_id).cloned()
    }

    /// Return currently connected workers.
    pub async fn workers(&self) -> Vec<RegisteredWorker> {
        self.workers
            .read()
            .await
            .values()
            .filter(|worker| worker.is_schedulable())
            .cloned()
            .collect()
    }

    /// Return all known sessions including replaced/history entries.
    pub async fn sessions(&self) -> Vec<RegisteredWorker> {
        self.workers.read().await.values().cloned().collect()
    }

    /// Return currently connected worker ids.
    pub async fn worker_ids(&self) -> Vec<String> {
        self.workers()
            .await
            .into_iter()
            .map(|worker| worker.worker_id)
            .collect()
    }

    /// Return worker ids matching the given namespace and app.
    pub async fn find_eligible_workers(&self, namespace: &str, app: &str) -> Vec<String> {
        self.find_eligible_workers_with_capability(namespace, app, None)
            .await
    }

    /// Return worker ids matching namespace/app plus broadcast selector constraints.
    pub async fn find_eligible_workers_with_broadcast_selector(
        &self,
        namespace: &str,
        app: &str,
        selector: Option<&BroadcastSelector>,
    ) -> Vec<String> {
        let selector = selector.cloned().unwrap_or_default();
        self.workers
            .read()
            .await
            .values()
            .filter(|worker| {
                worker.is_schedulable()
                    && is_match(&worker.namespace, namespace)
                    && is_match(&worker.app, app)
                    && broadcast_selector_matches(worker, &selector)
            })
            .map(|worker| worker.worker_id.clone())
            .collect()
    }

    /// Return worker ids matching namespace/app and an optional required capability.
    pub async fn find_eligible_workers_with_capability(
        &self,
        namespace: &str,
        app: &str,
        required_capability: Option<&str>,
    ) -> Vec<String> {
        let Some(required_capability) = required_capability else {
            return self
                .find_eligible_workers_with_requirement(namespace, app, None)
                .await;
        };
        let Some(requirement) = WorkerRequirement::from_legacy(required_capability) else {
            return Vec::new();
        };
        self.find_eligible_workers_with_requirement(namespace, app, Some(&requirement))
            .await
    }

    /// Return worker ids matching namespace/app and an optional structured requirement.
    pub async fn find_eligible_workers_with_requirement(
        &self,
        namespace: &str,
        app: &str,
        requirement: Option<&WorkerRequirement>,
    ) -> Vec<String> {
        self.workers
            .read()
            .await
            .values()
            .filter(|w| {
                w.is_schedulable()
                    && is_match(&w.namespace, namespace)
                    && is_match(&w.app, app)
                    && requirement.is_none_or(|requirement| worker_satisfies(w, requirement))
            })
            .map(|w| w.worker_id.clone())
            .collect()
    }

    /// Return worker ids matching namespace/app/requirement, preferring each domain master for ordered single dispatch.
    pub async fn find_ordered_dispatch_workers(
        &self,
        namespace: &str,
        app: &str,
        requirement: Option<&WorkerRequirement>,
    ) -> Vec<String> {
        let mut workers = self
            .workers
            .read()
            .await
            .values()
            .filter(|worker| {
                worker.is_schedulable()
                    && is_match(&worker.namespace, namespace)
                    && is_match(&worker.app, app)
                    && requirement.is_none_or(|requirement| worker_satisfies(worker, requirement))
            })
            .cloned()
            .collect::<Vec<_>>();
        workers.sort_by(|left, right| {
            right
                .master
                .is_master
                .cmp(&left.master.is_master)
                .then_with(|| left.master.domain.cmp(&right.master.domain))
                .then_with(|| left.election.priority.cmp(&right.election.priority))
                .then_with(|| left.worker_id.cmp(&right.worker_id))
        });
        workers.into_iter().map(|worker| worker.worker_id).collect()
    }

    /// Return true when a connected worker advertises the required capability.
    pub async fn worker_supports_capability(
        &self,
        worker_id: &str,
        required_capability: Option<&str>,
    ) -> bool {
        let Some(required_capability) = required_capability else {
            return self.worker_supports_requirement(worker_id, None).await;
        };
        let Some(requirement) = WorkerRequirement::from_legacy(required_capability) else {
            return false;
        };
        self.worker_supports_requirement(worker_id, Some(&requirement))
            .await
    }

    /// Return true when a connected worker satisfies the structured requirement.
    pub async fn worker_supports_requirement(
        &self,
        worker_id: &str,
        requirement: Option<&WorkerRequirement>,
    ) -> bool {
        let Some(requirement) = requirement else {
            return true;
        };
        self.workers
            .read()
            .await
            .get(worker_id)
            .is_some_and(|worker| worker.is_schedulable() && worker_satisfies(worker, requirement))
    }

    /// Return true when a worker session is still current and can write task messages.
    pub async fn accepts_worker_message(&self, worker_id: &str) -> bool {
        self.workers
            .read()
            .await
            .get(worker_id)
            .is_some_and(RegisteredWorker::is_schedulable)
    }

    /// Return persisted online workers matching namespace/app/requirement, preferring each domain master.
    ///
    /// When lifecycle storage is unavailable, falls back to the legacy in-process view for tests and
    /// embedded single-process use. Production server construction wires lifecycle storage.
    pub async fn find_ordered_persisted_dispatch_workers(
        &self,
        namespace: &str,
        app: &str,
        requirement: Option<&WorkerRequirement>,
    ) -> Vec<String> {
        let Some(lifecycle) = self.lifecycle.as_ref() else {
            return self
                .find_ordered_dispatch_workers(namespace, app, requirement)
                .await;
        };
        let Ok(mut workers) = lifecycle.list_online_workers(500).await else {
            return Vec::new();
        };
        workers.retain(|worker| persisted_worker_matches(worker, namespace, app, requirement));
        workers.sort_by(persisted_dispatch_order);
        workers.into_iter().map(|worker| worker.worker_id).collect()
    }

    /// Return persisted online workers matching namespace/app plus broadcast selector constraints.
    ///
    /// When lifecycle storage is unavailable, falls back to the legacy in-process view for tests and
    /// embedded single-process use. Production server construction wires lifecycle storage.
    pub async fn find_persisted_broadcast_workers(
        &self,
        namespace: &str,
        app: &str,
        selector: Option<&BroadcastSelector>,
    ) -> Vec<String> {
        let Some(lifecycle) = self.lifecycle.as_ref() else {
            return self
                .find_eligible_workers_with_broadcast_selector(namespace, app, selector)
                .await;
        };
        let selector = selector.cloned().unwrap_or_default();
        let Ok(workers) = lifecycle.list_online_workers(500).await else {
            return Vec::new();
        };
        workers
            .into_iter()
            .filter(|worker| persisted_broadcast_worker_matches(worker, namespace, app, &selector))
            .map(|worker| worker.worker_id)
            .collect()
    }

    /// Return true when durable worker lifecycle storage says the worker is current and capability-compatible.
    pub async fn persisted_worker_supports_requirement(
        &self,
        worker_id: &str,
        requirement: Option<&WorkerRequirement>,
    ) -> bool {
        let Some(lifecycle) = self.lifecycle.as_ref() else {
            return self
                .worker_supports_requirement(worker_id, requirement)
                .await;
        };
        lifecycle
            .get_online_current_worker(worker_id)
            .await
            .ok()
            .flatten()
            .is_some_and(|worker| {
                requirement
                    .is_none_or(|requirement| persisted_worker_satisfies(&worker, requirement))
            })
    }

    /// Dispatch one task to a specific currently registered worker.
    ///
    /// # Errors
    ///
    /// Returns `None` when the worker is not connected or the worker stream is closed.
    pub async fn dispatch_to_worker(
        &self,
        worker_id: &str,
        mut task: DispatchTask,
    ) -> Option<String> {
        let assignment_token = format!("asg-{}", Uuid::now_v7());
        task.assignment_token = assignment_token.clone();
        self.dispatch_tokened_to_worker(worker_id, task)
            .await
            .then_some(assignment_token)
    }

    /// Dispatch one task that already carries a persisted assignment token.
    pub async fn dispatch_tokened_to_worker(&self, worker_id: &str, task: DispatchTask) -> bool {
        if task.assignment_token.trim().is_empty() {
            metrics::counter!("tikeo_worker_dispatch_total", "result" => "missing_assignment_token")
                .increment(1);
            return false;
        }
        let persisted = match self.lifecycle.as_ref() {
            Some(lifecycle) => {
                let persisted = lifecycle
                    .get_online_current_worker(worker_id)
                    .await
                    .ok()
                    .flatten();
                if persisted.is_none() {
                    metrics::counter!("tikeo_worker_dispatch_total", "result" => "not_online")
                        .increment(1);
                    return false;
                }
                persisted
            }
            None => None,
        };
        let gateway_node_id = persisted.as_ref().map_or_else(
            || self.gateway_node_id.clone(),
            |worker| worker.gateway_node_id.clone(),
        );
        if gateway_node_id == self.gateway_node_id {
            return self.dispatch_to_local_worker(worker_id, task).await;
        }
        let Some(relay) = self.relay.as_ref() else {
            metrics::counter!("tikeo_worker_dispatch_total", "result" => "relay_unavailable")
                .increment(1);
            return false;
        };
        match relay
            .dispatch_to_gateway(&gateway_node_id, worker_id, task)
            .await
        {
            Ok(()) => {
                metrics::counter!("tikeo_worker_dispatch_total", "result" => "relayed")
                    .increment(1);
                true
            }
            Err(error) => {
                metrics::counter!("tikeo_worker_dispatch_total", "result" => "relay_failed")
                    .increment(1);
                if error.mark_worker_offline {
                    let _ = self.mark_transport_error(worker_id, &error.message).await;
                }
                false
            }
        }
    }

    /// Dispatch a task already authorized and tokened by the scheduling leader to a local stream.
    pub async fn dispatch_relayed_task_to_local_worker(
        &self,
        worker_id: &str,
        task: DispatchTask,
    ) -> bool {
        let workers = self.workers.read().await;
        let Some(worker) = workers.get(worker_id) else {
            return false;
        };
        if !worker.is_schedulable() {
            return false;
        }
        let worker = worker.clone();
        drop(workers);
        worker
            .outbound
            .send(Ok(ServerMessage {
                kind: Some(server_message::Kind::DispatchTask(task)),
            }))
            .await
            .is_ok()
    }

    async fn dispatch_to_local_worker(&self, worker_id: &str, task: DispatchTask) -> bool {
        let workers = self.workers.read().await;
        let Some(worker) = workers.get(worker_id) else {
            return false;
        };
        if !worker.is_schedulable() {
            return false;
        }
        let worker = worker.clone();
        drop(workers);
        if worker
            .outbound
            .send(Ok(ServerMessage {
                kind: Some(server_message::Kind::DispatchTask(task)),
            }))
            .await
            .is_ok()
        {
            metrics::counter!("tikeo_worker_dispatch_total", "result" => "sent").increment(1);
            true
        } else {
            metrics::counter!("tikeo_worker_dispatch_total", "result" => "closed").increment(1);
            false
        }
    }

    async fn persist_current_snapshots(&self) {
        if self.lifecycle.is_none() {
            return;
        }
        let snapshots = {
            let workers = self.workers.read().await;
            session_snapshots(workers.values())
        };
        self.persist_worker_snapshots(snapshots).await;
    }

    async fn persist_worker_snapshots(&self, snapshots: Vec<WorkerSessionSnapshotUpdate>) {
        let Some(lifecycle) = self.lifecycle.as_ref() else {
            return;
        };
        let _ = lifecycle.update_session_snapshots(snapshots).await;
    }
}

fn logical_instance_id(
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

fn stable_worker_id(
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

fn worker_election_registration(worker: &RegisterWorker) -> WorkerElectionRegistration {
    let election = worker.election.clone().unwrap_or_default();
    WorkerElectionRegistration {
        enabled: election.enabled,
        domain: normalized_election_domain(worker, &election),
        priority: election.priority,
    }
}

fn normalized_election_domain(worker: &RegisterWorker, election: &WorkerClusterElection) -> String {
    let configured = election.domain.trim();
    if configured.is_empty() {
        worker_domain(
            &worker.namespace,
            &worker.app,
            &worker.cluster,
            &worker.region,
        )
    } else {
        configured.to_owned()
    }
}

fn worker_domain(namespace: &str, app: &str, cluster: &str, region: &str) -> String {
    format!("{namespace}/{app}/{cluster}/{region}")
}

fn recompute_worker_master_states(workers: &mut HashMap<String, RegisteredWorker>) {
    let now = SystemTime::now();
    let mut winners = HashMap::<String, (String, u64, String)>::new();
    for worker in workers.values() {
        if !worker.election.enabled || !worker.is_current() || worker.lease_expires_at <= now {
            continue;
        }
        let term = workers
            .values()
            .filter(|candidate| candidate.election.domain == worker.election.domain)
            .map(|candidate| candidate.generation)
            .max()
            .unwrap_or(worker.generation);
        let candidate = (
            worker.worker_id.clone(),
            term,
            worker_master_fencing_token(&worker.election.domain, term, &worker.worker_id),
        );
        let replace = winners
            .get(&worker.election.domain)
            .is_none_or(|(winner_id, _, _)| {
                let winner = workers.get(winner_id);
                winner.is_none_or(|winner| {
                    worker.election.priority < winner.election.priority
                        || (worker.election.priority == winner.election.priority
                            && worker.worker_id < winner.worker_id)
                })
            });
        if replace {
            winners.insert(worker.election.domain.clone(), candidate);
        }
    }

    for worker in workers.values_mut() {
        if !worker.election.enabled {
            worker.master = WorkerMasterState::follower(worker.election.domain.clone());
            continue;
        }
        if let Some((master_worker_id, term, fencing_token)) = winners.get(&worker.election.domain)
        {
            worker.master = WorkerMasterState {
                domain: worker.election.domain.clone(),
                is_master: master_worker_id == &worker.worker_id,
                master_worker_id: Some(master_worker_id.clone()),
                term: *term,
                fencing_token: Some(fencing_token.clone()),
            };
        } else {
            worker.master = WorkerMasterState::follower(worker.election.domain.clone());
        }
    }
}

fn worker_master_fencing_token(domain: &str, term: u64, worker_id: &str) -> String {
    let digest = Sha256::digest(format!("{domain}:{term}:{worker_id}").as_bytes());
    format!("wmf-{term}-{}", hex_prefix(&digest, 16))
}

fn hex_prefix(bytes: &[u8], len: usize) -> String {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .take(len)
        .map(|nibble| char::from_digit(u32::from(nibble), 16).unwrap_or('0'))
        .collect()
}

fn json_or_empty_array<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_owned())
}

fn json_or_empty_object<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_owned())
}

fn worker_capabilities_json(capabilities: Option<&WorkerCapabilities>) -> String {
    let Some(capabilities) = capabilities else {
        return "{}".to_owned();
    };
    serde_json::to_string(&serde_json::json!({
        "tags": capabilities.tags,
        "sdkProcessors": capabilities.sdk_processors.iter().map(|processor| processor.name.as_str()).collect::<Vec<_>>(),
        "scriptRunners": capabilities.script_runners.iter().map(|runner| serde_json::json!({
            "language": runner.language,
            "sandboxBackend": runner.sandbox_backend,
        })).collect::<Vec<_>>(),
        "pluginProcessors": capabilities.plugin_processors.iter().map(|processor| serde_json::json!({
            "type": processor.r#type,
            "processorNames": processor.processor_names,
        })).collect::<Vec<_>>(),
    }))
    .unwrap_or_else(|_| "{}".to_owned())
}

fn session_snapshots<'a>(
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

fn next_generation(workers: &HashMap<String, RegisteredWorker>, logical_instance_id: &str) -> u64 {
    workers
        .values()
        .filter(|worker| worker.logical_instance_id == logical_instance_id)
        .map(|worker| worker.generation)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn replace_previous_generations(
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

fn empty_to_none(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn is_match(worker_val: &str, job_val: &str) -> bool {
    worker_val == job_val
        || worker_val == "*"
        || worker_val.is_empty()
        || job_val == "*"
        || job_val.is_empty()
}

fn broadcast_selector_matches(worker: &RegisteredWorker, selector: &BroadcastSelector) -> bool {
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

fn worker_satisfies(worker: &RegisteredWorker, requirement: &WorkerRequirement) -> bool {
    structured_capabilities_match(&worker.structured_capabilities, requirement)
}

fn persisted_dispatch_order(
    left: &PersistedOnlineWorkerSummary,
    right: &PersistedOnlineWorkerSummary,
) -> std::cmp::Ordering {
    let left_master = persisted_master_order(left);
    let right_master = persisted_master_order(right);
    right_master
        .0
        .cmp(&left_master.0)
        .then_with(|| left_master.1.cmp(&right_master.1))
        .then_with(|| left.worker_id.cmp(&right.worker_id))
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

fn persisted_worker_matches(
    worker: &PersistedOnlineWorkerSummary,
    namespace: &str,
    app: &str,
    requirement: Option<&WorkerRequirement>,
) -> bool {
    is_match(&worker.namespace_name, namespace)
        && is_match(&worker.app_name, app)
        && requirement.is_none_or(|requirement| persisted_worker_satisfies(worker, requirement))
}

fn persisted_broadcast_worker_matches(
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

fn persisted_worker_satisfies(
    worker: &PersistedOnlineWorkerSummary,
    requirement: &WorkerRequirement,
) -> bool {
    structured_capabilities_match(
        &parse_persisted_capabilities(&worker.structured_capabilities_json),
        requirement,
    )
}

fn parse_persisted_capabilities(value: &str) -> WorkerCapabilities {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(value) else {
        return WorkerCapabilities::default();
    };
    WorkerCapabilities {
        tags: value
            .get("tags")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_owned)
            .collect(),
        sdk_processors: value
            .get("sdkProcessors")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(|name| tikeo_proto::worker::v1::SdkProcessorCapability {
                name: name.to_owned(),
            })
            .collect(),
        script_runners: value
            .get("scriptRunners")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|runner| {
                Some(tikeo_proto::worker::v1::ScriptRunnerCapability {
                    language: runner.get("language")?.as_str()?.to_owned(),
                    sandbox_backend: runner
                        .get("sandboxBackend")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                })
            })
            .collect(),
        plugin_processors: value
            .get("pluginProcessors")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|processor| {
                Some(tikeo_proto::worker::v1::PluginProcessorCapability {
                    r#type: processor.get("type")?.as_str()?.to_owned(),
                    processor_names: processor
                        .get("processorNames")
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_owned)
                        .collect(),
                })
            })
            .collect(),
    }
}

fn parse_persisted_labels(value: &str) -> HashMap<String, String> {
    serde_json::from_str::<HashMap<String, String>>(value).unwrap_or_default()
}

/// Worker session status used by scheduling and UI grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerSessionStatus {
    /// Tunnel exists, lease is valid, and this generation is current.
    Online,
    /// Session was superseded by a newer generation for the same logical instance.
    Replaced,
    /// Worker sent a graceful unregister before closing the tunnel.
    Stopped,
    /// Server observed the stream close/error without graceful unregister.
    Offline,
}

impl WorkerSessionStatus {
    /// Stable wire value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Replaced => "replaced",
            Self::Stopped => "stopped",
            Self::Offline => "offline",
        }
    }
}

/// Registered worker metadata.
#[derive(Debug, Clone)]
pub struct RegisteredWorker {
    /// Worker session identity.
    pub worker_id: String,
    /// Stable logical instance key derived from namespace/app/cluster/region/client instance.
    pub logical_instance_id: String,
    /// Optional client-side stable instance hint.
    pub client_instance_id: Option<String>,
    /// Application name.
    pub app: String,
    /// Namespace.
    pub namespace: String,
    /// Cluster name.
    pub cluster: String,
    /// Region.
    pub region: String,
    /// Runtime capabilities.
    pub capabilities: Vec<String>,
    /// Structured runtime capabilities used by new dispatch routing.
    pub structured_capabilities: WorkerCapabilities,
    /// Worker cluster election registration.
    election: WorkerElectionRegistration,
    /// Current worker-side master election state.
    pub master: WorkerMasterState,
    /// Worker labels.
    pub labels: HashMap<String, String>,
    /// Outbound stream sender for server-to-worker commands.
    pub outbound: mpsc::Sender<Result<ServerMessage, Status>>,
    /// Monotonic generation for this logical instance.
    pub generation: u64,
    /// Fencing token assigned to this session.
    pub fencing_token: String,
    /// Current session status.
    pub status: WorkerSessionStatus,
    /// Machine-readable reason for the status.
    pub status_reason: Option<String>,
    /// Human-readable evidence for the status reason.
    pub status_evidence: Option<String>,
    /// Lease expiry timestamp.
    pub lease_expires_at: SystemTime,
    /// Replacement session id when status is replaced.
    pub replaced_by_worker_id: Option<String>,
    /// Registration timestamp.
    pub registered_at: SystemTime,
    /// Last heartbeat timestamp.
    pub last_heartbeat_at: SystemTime,
    /// Last heartbeat sequence.
    pub last_sequence: u64,
}

impl RegisteredWorker {
    const fn is_current(&self) -> bool {
        matches!(self.status, WorkerSessionStatus::Online)
    }

    fn is_schedulable(&self) -> bool {
        self.is_current() && self.lease_expires_at > SystemTime::now()
    }

    fn accepts_heartbeat(&self, generation: u64, fencing_token: &str) -> bool {
        self.is_current() && self.generation == generation && self.fencing_token == fencing_token
    }
}

#[cfg(test)]
mod registry_tests;
