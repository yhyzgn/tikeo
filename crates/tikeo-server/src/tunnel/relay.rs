//! Internal server-to-server Worker Tunnel relay.
//!
//! Worker gRPC streams are process-local transport handles. In Raft mode a Worker may
//! connect to any Server Pod, while the scheduling leader may run on another Pod. The
//! relay lets the leader dispatch to the Pod that owns the live stream without making
//! the Worker expose any inbound port.

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use async_trait::async_trait;
use tikeo_config::ClusterPeerConfig;
use tikeo_proto::worker::v1::DispatchTask;
use tonic_prost::prost::Message as _;

/// Error returned by an internal worker relay attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerRelayError {
    /// Human-readable diagnostic evidence.
    pub message: String,
    /// Whether this failure proves the target worker transport should be marked offline.
    pub mark_worker_offline: bool,
}

impl WorkerRelayError {
    /// Transient or configuration failure that does not prove the worker stream is gone.
    #[must_use]
    pub fn transient(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            mark_worker_offline: false,
        }
    }

    /// Gateway confirmed it does not own a usable stream for this worker.
    #[must_use]
    pub fn worker_offline(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            mark_worker_offline: true,
        }
    }
}

/// Server-to-server relay used by the scheduling leader to reach remote gateway Pods.
#[async_trait]
pub trait WorkerRelayDispatch: Send + Sync + Debug {
    /// Dispatch a fully assignment-tokened task to the gateway that owns `worker_id`.
    async fn dispatch_to_gateway(
        &self,
        gateway_node_id: &str,
        worker_id: &str,
        task: DispatchTask,
    ) -> Result<(), WorkerRelayError>;
}

/// Shared relay handle.
pub type SharedWorkerRelayDispatch = Arc<dyn WorkerRelayDispatch>;

/// HTTP implementation backed by `cluster.peers[].endpoint`.
#[derive(Debug, Clone)]
pub struct HttpWorkerRelayDispatch {
    client: reqwest::Client,
    peer_endpoints: Arc<HashMap<String, String>>,
    transport_token: Option<String>,
}

impl HttpWorkerRelayDispatch {
    /// Build a relay from configured cluster peers.
    #[must_use]
    pub fn from_peers(peers: &[ClusterPeerConfig], transport_token: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            peer_endpoints: Arc::new(
                peers
                    .iter()
                    .map(|peer| (peer.node_id.clone(), peer.endpoint.clone()))
                    .collect(),
            ),
            transport_token: transport_token.filter(|value| !value.is_empty()),
        }
    }

    fn dispatch_url(endpoint: &str, worker_id: &str) -> String {
        format!(
            "{}/api/v1/internal/worker-tunnel/dispatch/{}",
            endpoint.trim_end_matches('/'),
            worker_id
        )
    }
}

#[async_trait]
impl WorkerRelayDispatch for HttpWorkerRelayDispatch {
    async fn dispatch_to_gateway(
        &self,
        gateway_node_id: &str,
        worker_id: &str,
        task: DispatchTask,
    ) -> Result<(), WorkerRelayError> {
        let Some(endpoint) = self.peer_endpoints.get(gateway_node_id) else {
            return Err(WorkerRelayError::transient(format!(
                "no configured peer endpoint for worker gateway node {gateway_node_id}"
            )));
        };
        let Some(token) = self.transport_token.as_deref() else {
            return Err(WorkerRelayError::transient(
                "worker relay requires cluster.transport_token to be configured",
            ));
        };
        let mut body = Vec::new();
        task.encode(&mut body).map_err(|error| {
            WorkerRelayError::transient(format!("failed to encode worker dispatch task: {error}"))
        })?;
        let url = Self::dispatch_url(endpoint, worker_id);
        let response = self
            .client
            .post(&url)
            .header("x-tikeo-raft-token", token)
            .header("content-type", "application/x-protobuf")
            .body(body)
            .send()
            .await
            .map_err(|error| {
                WorkerRelayError::transient(format!(
                    "worker relay request to {gateway_node_id} failed: {error}"
                ))
            })?;
        if response.status().is_success() {
            return Ok(());
        }
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::GONE {
            Err(WorkerRelayError::worker_offline(format!(
                "worker gateway {gateway_node_id} does not own worker {worker_id}: http {status} {text}"
            )))
        } else {
            Err(WorkerRelayError::transient(format!(
                "worker relay to {gateway_node_id} rejected dispatch for {worker_id}: http {status} {text}"
            )))
        }
    }
}
