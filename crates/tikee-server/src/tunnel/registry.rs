//! In-memory Worker Tunnel connection registry.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime},
};

use sha2::{Digest, Sha256};
use tikee_proto::worker::v1::{
    DispatchTask, RegisterWorker, ServerMessage, WorkerCapabilities, WorkerClusterElection,
    server_message,
};
use tikee_storage::{RegisterWorkerSession, WorkerHeartbeat, WorkerLifecycleRepository};
use tokio::sync::{RwLock, mpsc};
use tonic::Status;
use uuid::Uuid;

use super::capability::{WorkerRequirement, structured_capabilities_match};

const DEFAULT_LEASE_SECONDS: u64 = 30;

/// Shared worker registry handle.
#[derive(Debug, Clone, Default)]
pub struct WorkerRegistry {
    workers: Arc<RwLock<HashMap<String, RegisteredWorker>>>,
    lifecycle: Option<WorkerLifecycleRepository>,
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
        }
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
        let (record, worker_count) = {
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
                active_assignment_tokens: Vec::new(),
            };
            replace_previous_generations(&mut workers, &logical_instance_id, &worker_id);
            workers.insert(record.worker_id.clone(), record.clone());
            recompute_worker_master_states(&mut workers);
            let worker_count = workers
                .values()
                .filter(|worker| worker.is_schedulable())
                .count();
            drop(workers);
            (record, worker_count)
        };
        metrics::gauge!("tikee_worker_connected_total")
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
                fencing_token: fencing_token.to_owned(),
                lease_seconds: i64::try_from(DEFAULT_LEASE_SECONDS).unwrap_or(i64::MAX),
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
                metrics::counter!("tikee_worker_stale_messages_total", "kind" => "heartbeat")
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
                metrics::counter!("tikee_worker_stale_messages_total", "kind" => "unregister")
                    .increment(1);
                return None;
            }
            worker.status = WorkerSessionStatus::Stopped;
            worker.status_reason = Some("graceful_shutdown".to_owned());
            worker.status_evidence = Some("worker sent graceful unregister".to_owned());
            worker.active_assignment_tokens.clear();
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
            worker.active_assignment_tokens.clear();
            recompute_worker_master_states(&mut workers);
            let offline = workers.get(worker_id).cloned()?;
            drop(workers);
            offline
        };
        if let Some(lifecycle) = self.lifecycle.as_ref() {
            let _ = lifecycle.mark_transport_error(worker_id, evidence).await;
        }
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
            return self.find_eligible_workers_with_requirement(namespace, app, None).await;
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

    /// Return true when a worker message carries a currently accepted assignment token.
    pub async fn accepts_worker_assignment(&self, worker_id: &str, assignment_token: &str) -> bool {
        if assignment_token.is_empty() {
            return false;
        }
        self.workers
            .read()
            .await
            .get(worker_id)
            .is_some_and(|worker| {
                worker.is_schedulable()
                    && worker
                        .active_assignment_tokens
                        .iter()
                        .any(|token| token == assignment_token)
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
        let worker = {
            let mut workers = self.workers.write().await;
            let worker = workers.get_mut(worker_id)?;
            if !worker.is_schedulable() {
                return None;
            }
            worker
                .active_assignment_tokens
                .push(assignment_token.clone());
            let worker = worker.clone();
            drop(workers);
            worker
        };
        let worker_id = worker.worker_id.clone();
        if worker
            .outbound
            .send(Ok(ServerMessage {
                kind: Some(server_message::Kind::DispatchTask(task)),
            }))
            .await
            .is_ok()
        {
            metrics::counter!("tikee_worker_dispatch_total", "result" => "sent").increment(1);
            Some(assignment_token)
        } else {
            metrics::counter!("tikee_worker_dispatch_total", "result" => "closed").increment(1);
            self.revoke_assignment_token(&worker_id, &assignment_token)
                .await;
            None
        }
    }

    async fn revoke_assignment_token(&self, worker_id: &str, assignment_token: &str) {
        {
            let mut workers = self.workers.write().await;
            let Some(worker) = workers.get_mut(worker_id) else {
                return;
            };
            worker
                .active_assignment_tokens
                .retain(|token| token != assignment_token);
            drop(workers);
        }
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
        worker.active_assignment_tokens.clear();
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
    /// Active assignment tokens currently accepted for task logs/results.
    pub active_assignment_tokens: Vec<String>,
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
mod tests {
    use tikee_proto::worker::v1::{
        DispatchTask, PluginProcessorCapability, RegisterWorker, ScriptRunnerCapability,
        SdkProcessorCapability, WorkerCapabilities, WorkerClusterElection,
    };
    use tokio::sync::mpsc;

    use tikee_storage::WorkerLifecycleRepository;

    use super::{BroadcastSelector, WorkerRegistry, WorkerSessionStatus};
    use crate::tunnel::capability::WorkerRequirement;

    #[tokio::test]
    async fn registry_elects_single_master_per_worker_domain_and_fails_over() {
        let registry = WorkerRegistry::default();
        let first = registry
            .register(election_worker("pod-a", 10), mpsc::channel(1).0)
            .await;
        let second = registry
            .register(election_worker("pod-b", 1), mpsc::channel(1).0)
            .await;

        let first_after_election = registry
            .get(&first.worker_id)
            .await
            .unwrap_or_else(|| panic!("first worker should exist"));
        let second_after_election = registry
            .get(&second.worker_id)
            .await
            .unwrap_or_else(|| panic!("second worker should exist"));

        assert!(!first_after_election.master.is_master);
        assert!(second_after_election.master.is_master);
        assert_eq!(
            first_after_election.master.master_worker_id.as_deref(),
            Some(second.worker_id.as_str())
        );
        assert_eq!(
            second_after_election.master.fencing_token,
            first_after_election.master.fencing_token
        );

        registry
            .mark_transport_error(&second.worker_id, "test disconnect")
            .await
            .unwrap_or_else(|| panic!("second worker should be marked offline"));
        let promoted = registry
            .get(&first.worker_id)
            .await
            .unwrap_or_else(|| panic!("first worker should remain"));

        assert!(promoted.master.is_master);
        assert_eq!(
            promoted.master.master_worker_id.as_deref(),
            Some(first.worker_id.as_str())
        );
    }

    #[tokio::test]
    async fn registry_orders_dispatch_candidates_by_domain_master_first() {
        let registry = WorkerRegistry::default();
        let follower = registry
            .register(election_worker("pod-a", 10), mpsc::channel(1).0)
            .await;
        let master = registry
            .register(election_worker("pod-b", 1), mpsc::channel(1).0)
            .await;

        let candidates = registry
            .find_ordered_dispatch_workers("finance", "billing", None)
            .await;

        assert_eq!(
            candidates.first().map(String::as_str),
            Some(master.worker_id.as_str())
        );
        assert!(candidates.contains(&follower.worker_id));
    }

    #[tokio::test]
    async fn registry_tracks_registration_and_heartbeat() {
        let registry = WorkerRegistry::default();
        let worker = registry
            .register(
                RegisterWorker {
                    client_instance_id: "pod-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "finance".to_owned(),
                    cluster: "prod".to_owned(),
                    region: "cn".to_owned(),
                    capabilities: vec!["http".to_owned()],
                    structured_capabilities: None,
                    election: None,
                    labels: [("runtime".to_owned(), "rust".to_owned())].into(),
                },
                mpsc::channel(1).0,
            )
            .await;

        let worker_id = registry
            .worker_ids()
            .await
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("worker id should exist"));
        let updated = registry
            .heartbeat(&worker_id, 7, 1, &worker.fencing_token)
            .await
            .unwrap_or_else(|| panic!("registered worker should exist"));

        assert!(updated.worker_id.starts_with("wrk-"));
        assert_eq!(updated.client_instance_id.as_deref(), Some("pod-1"));
        assert_eq!(updated.last_sequence, 7);
    }

    #[tokio::test]
    async fn registry_replaces_same_logical_instance_with_new_generation_and_fencing() {
        let registry = WorkerRegistry::default();
        let first = registry
            .register(register_worker("pod-1"), mpsc::channel(1).0)
            .await;
        let second = registry
            .register(register_worker("pod-1"), mpsc::channel(1).0)
            .await;

        assert_eq!(first.generation, 1);
        assert_eq!(second.generation, 2);
        assert_eq!(first.worker_id, second.worker_id);
        assert_ne!(first.fencing_token, second.fencing_token);

        assert!(
            registry
                .heartbeat(&first.worker_id, 9, first.generation, &first.fencing_token)
                .await
                .is_none(),
            "older generation heartbeat should be fenced"
        );
        let renewed = registry
            .heartbeat(
                &second.worker_id,
                10,
                second.generation,
                &second.fencing_token,
            )
            .await
            .unwrap_or_else(|| panic!("new generation heartbeat should renew"));
        assert_eq!(renewed.last_sequence, 10);
        assert_eq!(registry.worker_ids().await, vec![second.worker_id]);
    }

    #[tokio::test]
    async fn registry_dispatch_assignment_token_validates_current_worker_session() {
        let registry = WorkerRegistry::default();
        let (outbound, _inbound) = mpsc::channel(8);
        let worker = registry
            .register(register_worker("pod-assign"), outbound)
            .await;
        let task = DispatchTask {
            instance_id: "inst-assign".to_owned(),
            job_id: "job-assign".to_owned(),
            payload: Vec::new(),
            processor_name: "demo.echo".to_owned(),
            processor_binding: None,
            assignment_token: String::new(),
        };

        let token = registry
            .dispatch_to_worker(&worker.worker_id, task)
            .await
            .unwrap_or_else(|| panic!("dispatch should assign token"));

        assert!(
            registry
                .accepts_worker_assignment(&worker.worker_id, &token)
                .await
        );
        assert!(
            !registry
                .accepts_worker_assignment(&worker.worker_id, "wrong-token")
                .await
        );
    }

    #[tokio::test]
    async fn registry_matches_broadcast_selector_region_tags_cluster_and_labels() {
        let registry = WorkerRegistry::default();
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "pod-java-cn".to_owned(),
                    region: "cn".to_owned(),
                    cluster: "v2".to_owned(),
                    structured_capabilities: Some(WorkerCapabilities {
                        tags: vec!["java".to_owned(), "blue".to_owned()],
                        ..WorkerCapabilities::default()
                    }),
                    labels: [("tier".to_owned(), "gold".to_owned())].into(),
                    ..register_worker("pod-java-cn")
                },
                mpsc::channel(1).0,
            )
            .await;
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "pod-rust-us".to_owned(),
                    region: "us".to_owned(),
                    cluster: "v1".to_owned(),
                    structured_capabilities: Some(WorkerCapabilities {
                        tags: vec!["rust".to_owned()],
                        ..WorkerCapabilities::default()
                    }),
                    labels: [("tier".to_owned(), "silver".to_owned())].into(),
                    ..register_worker("pod-rust-us")
                },
                mpsc::channel(1).0,
            )
            .await;

        let workers = registry
            .find_eligible_workers_with_broadcast_selector(
                "finance",
                "billing",
                Some(&BroadcastSelector {
                    tags: vec!["java".to_owned(), "blue".to_owned()],
                    region: Some("cn".to_owned()),
                    cluster: Some("v2".to_owned()),
                    labels: [("tier".to_owned(), "gold".to_owned())].into(),
                }),
            )
            .await;

        assert_eq!(workers.len(), 1);
    }

    #[tokio::test]
    async fn registry_requires_structured_script_runner_capabilities() {
        let registry = WorkerRegistry::default();
        registry
            .register(
                RegisterWorker {
                    capabilities: vec!["script".to_owned()],
                    ..register_worker("pod-script")
                },
                mpsc::channel(1).0,
            )
            .await;
        registry
            .register(
                RegisterWorker {
                    capabilities: vec!["legacy-script-python".to_owned()],
                    ..register_worker("pod-python")
                },
                mpsc::channel(1).0,
            )
            .await;
        registry
            .register(
                RegisterWorker {
                    structured_capabilities: Some(WorkerCapabilities {
                        script_runners: vec![ScriptRunnerCapability {
                            language: "python".to_owned(),
                            sandbox_backend: "srt".to_owned(),
                        }],
                        ..WorkerCapabilities::default()
                    }),
                    ..register_worker("pod-python-structured")
                },
                mpsc::channel(1).0,
            )
            .await;

        let legacy_script_workers = registry
            .find_eligible_workers_with_capability("finance", "billing", Some("script"))
            .await;
        assert!(legacy_script_workers.is_empty());
        let legacy_python_workers = registry
            .find_eligible_workers_with_capability("finance", "billing", Some("legacy-script-python"))
            .await;
        assert!(legacy_python_workers.is_empty());
        let python_workers = registry
            .find_eligible_workers_with_requirement(
                "finance",
                "billing",
                Some(&WorkerRequirement::ScriptRunner {
                    language: "python".to_owned(),
                }),
            )
            .await;
        assert_eq!(python_workers.len(), 1);
    }

    #[tokio::test]
    async fn registry_matches_structured_sdk_script_and_plugin_capabilities() {
        let registry = WorkerRegistry::default();
        registry
            .register(
                RegisterWorker {
                    structured_capabilities: Some(WorkerCapabilities {
                        tags: vec!["java".to_owned()],
                        sdk_processors: vec![SdkProcessorCapability {
                            name: "demo.echo".to_owned(),
                        }],
                        script_runners: vec![ScriptRunnerCapability {
                            language: "python".to_owned(),
                            sandbox_backend: "srt".to_owned(),
                        }],
                        plugin_processors: vec![PluginProcessorCapability {
                            r#type: "sql".to_owned(),
                            processor_names: vec!["billing.sql-sync".to_owned()],
                        }],
                    }),
                    ..register_worker("pod-structured")
                },
                mpsc::channel(1).0,
            )
            .await;

        assert_eq!(
            registry
                .find_eligible_workers_with_requirement(
                    "finance",
                    "billing",
                    Some(&WorkerRequirement::SdkProcessor {
                        name: "demo.echo".to_owned()
                    })
                )
                .await
                .len(),
            1
        );
        assert_eq!(
            registry
                .find_eligible_workers_with_requirement(
                    "finance",
                    "billing",
                    Some(&WorkerRequirement::ScriptRunner {
                        language: "python".to_owned()
                    })
                )
                .await
                .len(),
            1
        );
        assert_eq!(
            registry
                .find_eligible_workers_with_requirement(
                    "finance",
                    "billing",
                    Some(&WorkerRequirement::PluginProcessor {
                        processor_type: "sql".to_owned(),
                        processor_name: "billing.sql-sync".to_owned()
                    })
                )
                .await
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn registry_marks_transport_error_offline_and_persists_evidence() {
        let db = tikee_storage::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let lifecycle = WorkerLifecycleRepository::new(db);
        let registry = WorkerRegistry::with_lifecycle(lifecycle.clone());
        let worker = registry
            .register(register_worker("pod-transport"), mpsc::channel(1).0)
            .await;

        let offline = registry
            .mark_transport_error(&worker.worker_id, "worker tunnel stream ended")
            .await
            .unwrap_or_else(|| panic!("current worker should be marked offline"));

        assert_eq!(offline.status, WorkerSessionStatus::Offline);
        assert!(registry.worker_ids().await.is_empty());
        assert!(
            !registry.accepts_worker_message(&worker.worker_id).await,
            "offline transport session must not stay schedulable"
        );
        let persisted = lifecycle
            .get_session(&worker.worker_id)
            .await
            .unwrap_or_else(|error| panic!("persisted session should load: {error}"))
            .unwrap_or_else(|| panic!("persisted session should exist"));
        assert_eq!(persisted.status, "offline");
        assert_eq!(persisted.status_reason.as_deref(), Some("transport_error"));
    }

    #[tokio::test]
    async fn registry_persists_reconnect_as_same_worker_id_with_new_generation() {
        let db = tikee_storage::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let lifecycle = WorkerLifecycleRepository::new(db);
        let registry = WorkerRegistry::with_lifecycle(lifecycle.clone());

        let first = registry
            .register(register_worker("pod-1"), mpsc::channel(1).0)
            .await;
        let second = registry
            .register(register_worker("pod-1"), mpsc::channel(1).0)
            .await;

        assert_eq!(first.worker_id, second.worker_id);
        assert_ne!(first.fencing_token, second.fencing_token);
        let persisted_second = lifecycle
            .get_session(&second.worker_id)
            .await
            .unwrap_or_else(|error| panic!("persisted reconnected session should load: {error}"))
            .unwrap_or_else(|| panic!("persisted reconnected session should exist"));

        assert_eq!(persisted_second.status, "online");
        assert_eq!(persisted_second.generation, 2);

        registry
            .heartbeat(
                &second.worker_id,
                11,
                second.generation,
                &second.fencing_token,
            )
            .await
            .unwrap_or_else(|| panic!("current heartbeat should renew"));
        let renewed = lifecycle
            .get_session(&second.worker_id)
            .await
            .unwrap_or_else(|error| panic!("renewed session should load: {error}"))
            .unwrap_or_else(|| panic!("renewed session should exist"));
        assert_eq!(renewed.last_sequence, 11);
    }

    fn election_worker(client_instance_id: &str, priority: u32) -> RegisterWorker {
        RegisterWorker {
            election: Some(WorkerClusterElection {
                enabled: true,
                domain: String::new(),
                priority,
            }),
            ..register_worker(client_instance_id)
        }
    }

    fn register_worker(client_instance_id: &str) -> RegisterWorker {
        RegisterWorker {
            client_instance_id: client_instance_id.to_owned(),
            app: "billing".to_owned(),
            namespace: "finance".to_owned(),
            cluster: "prod".to_owned(),
            region: "cn".to_owned(),
            capabilities: vec!["http".to_owned()],
            structured_capabilities: None,
            election: None,
            labels: [("runtime".to_owned(), "rust".to_owned())].into(),
        }
    }
}
