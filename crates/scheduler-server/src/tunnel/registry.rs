//! In-memory Worker Tunnel connection registry.

use std::{collections::HashMap, sync::Arc, time::SystemTime};

use scheduler_proto::worker::v1::{DispatchTask, RegisterWorker, ServerMessage, server_message};
use tokio::sync::{RwLock, mpsc};
use tonic::Status;

/// Shared worker registry handle.
#[derive(Debug, Clone, Default)]
pub struct WorkerRegistry {
    workers: Arc<RwLock<HashMap<String, RegisteredWorker>>>,
}

impl WorkerRegistry {
    /// Register or replace a worker record.
    pub async fn register(
        &self,
        worker: RegisterWorker,
        outbound: mpsc::Sender<Result<ServerMessage, Status>>,
    ) -> RegisteredWorker {
        let record = RegisteredWorker {
            worker_id: worker.worker_id.clone(),
            app: worker.app,
            namespace: worker.namespace,
            cluster: worker.cluster,
            region: worker.region,
            capabilities: worker.capabilities,
            labels: worker.labels,
            outbound,
            registered_at: SystemTime::now(),
            last_heartbeat_at: SystemTime::now(),
            last_sequence: 0,
        };

        self.workers
            .write()
            .await
            .insert(record.worker_id.clone(), record.clone());

        record
    }

    /// Record a heartbeat for a known worker.
    #[allow(clippy::significant_drop_tightening)]
    pub async fn heartbeat(&self, worker_id: &str, sequence: u64) -> Option<RegisteredWorker> {
        let mut workers = self.workers.write().await;
        let worker = workers.get_mut(worker_id)?;
        worker.last_heartbeat_at = SystemTime::now();
        worker.last_sequence = sequence;
        Some(worker.clone())
    }

    /// Return a worker by id.
    pub async fn get(&self, worker_id: &str) -> Option<RegisteredWorker> {
        self.workers.read().await.get(worker_id).cloned()
    }

    /// Return currently connected worker ids.
    pub async fn worker_ids(&self) -> Vec<String> {
        self.workers.read().await.keys().cloned().collect()
    }

    /// Dispatch one task to a specific currently registered worker.
    ///
    /// # Errors
    ///
    /// Returns `None` when the worker is not connected or the worker stream is closed.
    pub async fn dispatch_to_worker(&self, worker_id: &str, task: DispatchTask) -> Option<String> {
        let worker = self.workers.read().await.get(worker_id).cloned()?;
        let worker_id = worker.worker_id.clone();
        worker
            .outbound
            .send(Ok(ServerMessage {
                kind: Some(server_message::Kind::DispatchTask(task)),
            }))
            .await
            .ok()?;
        Some(worker_id)
    }

    /// Dispatch one task to the first currently registered worker.
    ///
    /// # Errors
    ///
    /// Returns `None` when no worker is connected or the worker stream is closed.
    pub async fn dispatch_to_first(&self, task: DispatchTask) -> Option<String> {
        let worker = self.workers.read().await.values().next().cloned()?;
        let worker_id = worker.worker_id.clone();
        worker
            .outbound
            .send(Ok(ServerMessage {
                kind: Some(server_message::Kind::DispatchTask(task)),
            }))
            .await
            .ok()?;
        Some(worker_id)
    }
}

/// Registered worker metadata.
#[derive(Debug, Clone)]
pub struct RegisteredWorker {
    /// Worker identity.
    pub worker_id: String,
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
    /// Registration timestamp.
    pub registered_at: SystemTime,
    /// Last heartbeat timestamp.
    pub last_heartbeat_at: SystemTime,
    /// Last heartbeat sequence.
    pub last_sequence: u64,
}

#[cfg(test)]
mod tests {
    use scheduler_proto::worker::v1::RegisterWorker;
    use tokio::sync::mpsc;

    use super::WorkerRegistry;

    #[tokio::test]
    async fn registry_tracks_registration_and_heartbeat() {
        let registry = WorkerRegistry::default();
        registry
            .register(
                RegisterWorker {
                    worker_id: "worker-1".to_owned(),
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

        let updated = registry
            .heartbeat("worker-1", 7)
            .await
            .unwrap_or_else(|| panic!("registered worker should exist"));

        assert_eq!(updated.worker_id, "worker-1");
        assert_eq!(updated.last_sequence, 7);
    }
}
