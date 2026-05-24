//! In-memory Worker Tunnel connection registry.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use tikee_proto::worker::v1::{DispatchTask, RegisterWorker, ServerMessage, server_message};
use tikee_storage::{RegisterWorkerSession, WorkerHeartbeat, WorkerLifecycleRepository};
use tokio::sync::{RwLock, mpsc};
use tonic::Status;
use uuid::Uuid;

const DEFAULT_LEASE_SECONDS: u64 = 30;

/// Shared worker registry handle.
#[derive(Debug, Clone, Default)]
pub struct WorkerRegistry {
    workers: Arc<RwLock<HashMap<String, RegisteredWorker>>>,
    lifecycle: Option<WorkerLifecycleRepository>,
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
        let worker_id = format!("wrk-{}", Uuid::now_v7());
        let client_instance_id = empty_to_none(worker.client_instance_id.clone());
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
            let updated = worker.clone();
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

    /// Return worker ids matching namespace/app and an optional required capability.
    pub async fn find_eligible_workers_with_capability(
        &self,
        namespace: &str,
        app: &str,
        required_capability: Option<&str>,
    ) -> Vec<String> {
        self.workers
            .read()
            .await
            .values()
            .filter(|w| {
                w.is_schedulable()
                    && is_match(&w.namespace, namespace)
                    && is_match(&w.app, app)
                    && required_capability
                        .is_none_or(|capability| worker_has_capability(w, capability))
            })
            .map(|w| w.worker_id.clone())
            .collect()
    }

    /// Return true when a connected worker advertises the required capability.
    pub async fn worker_supports_capability(
        &self,
        worker_id: &str,
        required_capability: Option<&str>,
    ) -> bool {
        let Some(required_capability) = required_capability else {
            return true;
        };
        self.workers
            .read()
            .await
            .get(worker_id)
            .is_some_and(|worker| {
                worker.is_schedulable() && worker_has_capability(worker, required_capability)
            })
    }

    /// Return true when a worker session is still current and can write task messages.
    pub async fn accepts_worker_message(&self, worker_id: &str) -> bool {
        self.workers
            .read()
            .await
            .get(worker_id)
            .is_some_and(RegisteredWorker::is_schedulable)
    }

    /// Dispatch one task to a specific currently registered worker.
    ///
    /// # Errors
    ///
    /// Returns `None` when the worker is not connected or the worker stream is closed.
    pub async fn dispatch_to_worker(&self, worker_id: &str, task: DispatchTask) -> Option<String> {
        let worker = self.workers.read().await.get(worker_id).cloned()?;
        if !worker.is_schedulable() {
            return None;
        }
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
            Some(worker_id)
        } else {
            metrics::counter!("tikee_worker_dispatch_total", "result" => "closed").increment(1);
            None
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

fn worker_has_capability(worker: &RegisteredWorker, required: &str) -> bool {
    worker.capabilities.iter().any(|capability| {
        capability == required
            || capability == "*"
            || capability == "script:*" && required.starts_with("script:")
    })
}

/// Worker session status used by scheduling and UI grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerSessionStatus {
    /// Tunnel exists, lease is valid, and this generation is current.
    Online,
    /// Session was superseded by a newer generation for the same logical instance.
    Replaced,
}

impl WorkerSessionStatus {
    /// Stable wire value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Replaced => "replaced",
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
mod tests {
    use tikee_proto::worker::v1::RegisterWorker;
    use tokio::sync::mpsc;

    use tikee_storage::WorkerLifecycleRepository;

    use super::{WorkerRegistry, WorkerSessionStatus};

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
        assert_ne!(first.worker_id, second.worker_id);
        assert_ne!(first.fencing_token, second.fencing_token);

        let old = registry
            .get(&first.worker_id)
            .await
            .unwrap_or_else(|| panic!("old session should remain inspectable"));
        assert_eq!(old.status, WorkerSessionStatus::Replaced);
        assert_eq!(
            old.status_reason.as_deref(),
            Some("replaced_by_new_generation")
        );
        assert_eq!(
            old.replaced_by_worker_id.as_deref(),
            Some(second.worker_id.as_str())
        );

        assert!(
            registry
                .heartbeat(&first.worker_id, 9, first.generation, &first.fencing_token)
                .await
                .is_none(),
            "replaced session heartbeat should be fenced"
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
    async fn registry_persists_replaced_generations_when_lifecycle_store_is_configured() {
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

        let persisted_first = lifecycle
            .get_session(&first.worker_id)
            .await
            .unwrap_or_else(|error| panic!("persisted old session should load: {error}"))
            .unwrap_or_else(|| panic!("persisted old session should exist"));
        let persisted_second = lifecycle
            .get_session(&second.worker_id)
            .await
            .unwrap_or_else(|error| panic!("persisted new session should load: {error}"))
            .unwrap_or_else(|| panic!("persisted new session should exist"));

        assert_eq!(persisted_first.status, "replaced");
        assert_eq!(
            persisted_first.status_reason.as_deref(),
            Some("replaced_by_new_generation")
        );
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

    fn register_worker(client_instance_id: &str) -> RegisterWorker {
        RegisterWorker {
            client_instance_id: client_instance_id.to_owned(),
            app: "billing".to_owned(),
            namespace: "finance".to_owned(),
            cluster: "prod".to_owned(),
            region: "cn".to_owned(),
            capabilities: vec!["http".to_owned()],
            labels: [("runtime".to_owned(), "rust".to_owned())].into(),
        }
    }
}
