//! `TiKV` raft-rs bootstrap and runtime integration.
//!
//! This module validates the crate/config/storage boundary and hosts the first runtime
//! ticker. It deliberately does not campaign or grant scheduler ownership until real
//! consensus leadership and fencing are implemented.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use protobuf::Message as PbMessage;
use raft::{
    Config, StateRole,
    eraftpb::{Entry, EntryType, HardState, Message, Snapshot},
    raw_node::RawNode,
    storage::MemStorage,
};
use scheduler_config::ClusterConfig;
use scheduler_storage::{
    RaftRepository, UpsertRaftLogEntry, UpsertRaftMetadata, UpsertRaftSnapshot,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, warn};

use super::{
    ClusterMode, ClusterRole, ClusterStatus, RaftMessageSubmission, SharedClusterCoordinator,
};

/// Crate-level runtime library label exposed in diagnostics and design docs.
pub const RAFT_RS_LIBRARY: &str = "tikv/raft-rs crate raft 0.7.0";

const CLUSTER_ID: &str = "default";
const TICK_INTERVAL: Duration = Duration::from_millis(100);
const INBOX_CAPACITY: usize = 256;

/// Safe bootstrap status produced by constructing a raft-rs `RawNode` without driving it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftRsBootstrapStatus {
    /// Scheduler string node id from config.
    pub node_id: String,
    /// Deterministic raft-rs numeric node id derived from `node_id`.
    pub raft_node_id: u64,
    /// Numeric voter ids derived from configured peers.
    pub voter_ids: Vec<u64>,
    /// Initial raft-rs role after construction; must not be treated as scheduler ownership.
    pub initial_role: String,
    /// Whether a freshly constructed node reports ready work before the event loop starts.
    pub has_ready: bool,
}

#[derive(Debug, Clone)]
struct RaftPeerTransport {
    client: reqwest::Client,
    endpoints: Arc<BTreeMap<u64, String>>,
    token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RaftWireMessage {
    from: u64,
    to: u64,
    term: i64,
    message_type: String,
    index: i64,
    log_term: i64,
    commit: i64,
    snapshot_index: Option<i64>,
    snapshot_term: Option<i64>,
    entries: Vec<RaftWireLogEntry>,
    context: Option<String>,
    reject: bool,
    reject_hint: Option<i64>,
    leader_fencing_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RaftWireLogEntry {
    entry_type: String,
    index: i64,
    term: i64,
    data: String,
    context: Option<String>,
}

impl RaftPeerTransport {
    fn new(endpoints: BTreeMap<u64, String>, token: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoints: Arc::new(endpoints),
            token: token.filter(|value| !value.is_empty()),
        }
    }

    fn dispatch_ready_messages(&self, messages: &[Message]) {
        for message in messages {
            let Some(endpoint) = self.endpoints.get(&message.to).cloned() else {
                debug!(
                    to = message.to,
                    message_type = ?message.get_msg_type(),
                    "skip raft-rs outbound message without configured peer endpoint"
                );
                continue;
            };
            let request = raft_message_to_wire_request(message);
            let client = self.client.clone();
            let token = self.token.clone();
            tokio::spawn(async move {
                let url = raft_append_entries_url(&endpoint);
                let mut builder = client.post(&url).json(&request);
                if let Some(token) = token {
                    builder = builder.header("x-scheduler-raft-token", token);
                }
                match builder.send().await {
                    Ok(response) if response.status().is_success() => {
                        debug!(%url, "raft-rs outbound message delivered");
                    }
                    Ok(response) => {
                        warn!(%url, status = %response.status(), "raft-rs outbound message rejected by peer");
                    }
                    Err(error) => {
                        warn!(%url, %error, "raft-rs outbound message delivery failed");
                    }
                }
            });
        }
    }
}

/// Runtime coordinator backed by a raft-rs `RawNode` ticker.
#[derive(Debug)]
pub struct RaftRuntimeCoordinator {
    status: Arc<RwLock<ClusterStatus>>,
    inbox: mpsc::Sender<Message>,
}

impl RaftRuntimeCoordinator {
    /// Start the raft-rs ticker runtime and return a coordinator handle.
    ///
    /// # Errors
    ///
    /// Returns a storage error if the initial runtime status cannot be persisted.
    pub async fn start(
        config: &ClusterConfig,
        repository: RaftRepository,
    ) -> Result<SharedClusterCoordinator, scheduler_storage::DbErr> {
        let initial = build_runtime(config);
        let (role, detail, node, transport) = match initial {
            Ok((node, bootstrap, transport)) => (
                ClusterRole::Follower,
                format!(
                    "{RAFT_RS_LIBRARY} runtime ticker started: raft_node_id={}, voters={}, initial_role={}, has_ready={}; no campaign, no leader fencing yet",
                    bootstrap.raft_node_id,
                    bootstrap.voter_ids.len(),
                    bootstrap.initial_role,
                    bootstrap.has_ready
                ),
                Some(node),
                Some(transport),
            ),
            Err(error) => (
                ClusterRole::Unknown,
                format!(
                    "{RAFT_RS_LIBRARY} runtime bootstrap failed: {error}; raft ticker not started"
                ),
                None,
                None,
            ),
        };
        let status = Arc::new(RwLock::new(ClusterStatus {
            mode: ClusterMode::Raft,
            role,
            node_id: config.node_id.clone(),
            nodes: u32::try_from(config.peers.len()).unwrap_or(u32::MAX).max(1),
            can_schedule: false,
            leader_fencing_token: None,
            detail,
        }));
        let (tx, rx) = mpsc::channel(INBOX_CAPACITY);
        if let (Some(node), Some(transport)) = (node, transport) {
            persist_role_metadata(&repository, &config.node_id, role).await?;
            spawn_runtime_loop(
                config.node_id.clone(),
                status.clone(),
                repository,
                node,
                transport,
                rx,
            );
        }
        Ok(Arc::new(Self { status, inbox: tx }))
    }
}

#[async_trait::async_trait]
impl super::ClusterCoordinator for RaftRuntimeCoordinator {
    async fn status(&self) -> ClusterStatus {
        self.status.read().await.clone()
    }

    async fn submit_raft_message(&self, message: Message) -> RaftMessageSubmission {
        let message_type = message.get_msg_type();
        match self.inbox.try_send(message) {
            Ok(()) => RaftMessageSubmission::accepted(message_type),
            Err(mpsc::error::TrySendError::Full(_)) => RaftMessageSubmission::unavailable(
                "raft-rs runtime inbox is full; retry after the local node drains messages",
            ),
            Err(mpsc::error::TrySendError::Closed(_)) => RaftMessageSubmission::unavailable(
                "raft-rs runtime inbox is closed because bootstrap failed or runtime stopped",
            ),
        }
    }
}

/// Validate that the current cluster config can construct a raft-rs `RawNode`.
///
/// # Errors
///
/// Returns a human-readable error when the raft-rs config or bootstrap storage is invalid.
pub fn validate_raft_rs_bootstrap(config: &ClusterConfig) -> Result<RaftRsBootstrapStatus, String> {
    let (node, status, _transport) = build_runtime(config)?;
    let _node = node;
    Ok(status)
}

/// Deterministically map existing string node ids to raft-rs `u64` node ids.
#[must_use]
pub fn raft_numeric_id(node_id: &str) -> u64 {
    let digest = Sha256::digest(node_id.as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    let id = u64::from_be_bytes(bytes);
    if id == 0 { 1 } else { id }
}

fn build_runtime(
    config: &ClusterConfig,
) -> Result<
    (
        RawNode<MemStorage>,
        RaftRsBootstrapStatus,
        RaftPeerTransport,
    ),
    String,
> {
    let raft_node_id = raft_numeric_id(&config.node_id);
    let voter_ids = voter_ids(config, raft_node_id)?;
    let endpoints = peer_endpoints(config);
    let mut raft_config = Config::new(raft_node_id);
    raft_config.heartbeat_tick = 2;
    raft_config.election_tick = 20;
    raft_config.check_quorum = true;
    raft_config.pre_vote = true;
    raft_config
        .validate()
        .map_err(|error| format!("raft-rs config invalid: {error}"))?;

    let storage = MemStorage::new_with_conf_state((voter_ids.clone(), Vec::new()));
    let node = RawNode::with_default_logger(&raft_config, storage)
        .map_err(|error| format!("raft-rs RawNode bootstrap failed: {error}"))?;
    let raft_status = node.status();
    let initial_role = raft_role_name(raft_status.ss.raft_state).to_owned();
    let has_ready = node.has_ready();

    Ok((
        node,
        RaftRsBootstrapStatus {
            node_id: config.node_id.clone(),
            raft_node_id,
            voter_ids,
            initial_role,
            has_ready,
        },
        RaftPeerTransport::new(endpoints, config.transport_token.clone()),
    ))
}

fn spawn_runtime_loop(
    node_id: String,
    status: Arc<RwLock<ClusterStatus>>,
    repository: RaftRepository,
    node: RawNode<MemStorage>,
    transport: RaftPeerTransport,
    inbox: mpsc::Receiver<Message>,
) {
    tokio::spawn(async move {
        let node = Arc::new(Mutex::new(node));
        run_runtime_loop(node_id, status, repository, node, transport, inbox).await;
    });
}

#[allow(clippy::significant_drop_tightening)]
async fn run_runtime_loop(
    node_id: String,
    status: Arc<RwLock<ClusterStatus>>,
    repository: RaftRepository,
    node: Arc<Mutex<RawNode<MemStorage>>>,
    transport: RaftPeerTransport,
    mut inbox: mpsc::Receiver<Message>,
) {
    let mut ticker = tokio::time::interval(TICK_INTERVAL);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let mut guard = node.lock().await;
                guard.tick();
                if let Err(error) = process_ready(&node_id, &repository, &mut guard, &status, &transport).await {
                    warn!(%error, "raft-rs Ready processing failed");
                }
            }
            message = inbox.recv() => {
                let Some(message) = message else { break; };
                let mut guard = node.lock().await;
                if let Err(error) = guard.step(message) {
                    warn!(%error, "raft-rs message step failed");
                }
                if let Err(error) = process_ready(&node_id, &repository, &mut guard, &status, &transport).await {
                    warn!(%error, "raft-rs Ready processing failed");
                }
            }
        }
    }
}

async fn process_ready(
    node_id: &str,
    repository: &RaftRepository,
    node: &mut RawNode<MemStorage>,
    status: &Arc<RwLock<ClusterStatus>>,
    transport: &RaftPeerTransport,
) -> Result<(), scheduler_storage::DbErr> {
    if !node.has_ready() {
        update_runtime_status(node_id, repository, node, status).await?;
        return Ok(());
    }

    let ready = node.ready();
    if let Some(hard_state) = ready.hs() {
        persist_hard_state(node_id, repository, hard_state).await?;
    }
    for entry in ready.entries() {
        persist_entry(node_id, repository, entry).await?;
    }
    if !ready.snapshot().is_empty() {
        persist_snapshot(node_id, repository, ready.snapshot()).await?;
    }
    transport.dispatch_ready_messages(ready.messages());
    transport.dispatch_ready_messages(ready.persisted_messages());
    let applied = apply_committed_entries(node_id, repository, ready.committed_entries()).await?;
    let light_ready = node.advance_append(ready);
    if let Some(applied) = applied {
        node.advance_apply_to(applied);
    }
    transport.dispatch_ready_messages(light_ready.messages());
    let light_applied =
        apply_committed_entries(node_id, repository, light_ready.committed_entries()).await?;
    if let Some(applied) = light_applied {
        node.advance_apply_to(applied);
    }
    update_runtime_status(node_id, repository, node, status).await?;
    Ok(())
}

async fn persist_hard_state(
    node_id: &str,
    repository: &RaftRepository,
    hard_state: &HardState,
) -> Result<(), scheduler_storage::DbErr> {
    let existing = repository.get_metadata(node_id).await?;
    repository
        .upsert_metadata(UpsertRaftMetadata {
            cluster_id: CLUSTER_ID.to_owned(),
            node_id: node_id.to_owned(),
            current_term: i64::try_from(hard_state.term).unwrap_or(i64::MAX),
            voted_for: if hard_state.vote == 0 {
                None
            } else {
                Some(hard_state.vote.to_string())
            },
            commit_index: i64::try_from(hard_state.commit).unwrap_or(i64::MAX),
            applied_index: existing.as_ref().map_or(0, |item| item.applied_index),
            leader_fencing_token: existing.and_then(|item| item.leader_fencing_token),
        })
        .await
        .map(|_| ())
}

async fn persist_entry(
    node_id: &str,
    repository: &RaftRepository,
    entry: &Entry,
) -> Result<(), scheduler_storage::DbErr> {
    repository
        .upsert_log_entry(UpsertRaftLogEntry {
            cluster_id: CLUSTER_ID.to_owned(),
            node_id: node_id.to_owned(),
            log_index: i64::try_from(entry.get_index()).unwrap_or(i64::MAX),
            term: i64::try_from(entry.get_term()).unwrap_or(i64::MAX),
            entry_type: format!("{:?}", entry.get_entry_type()),
            data: STANDARD.encode(entry.get_data()),
            context: if entry.get_context().is_empty() {
                None
            } else {
                Some(STANDARD.encode(entry.get_context()))
            },
            sync_status: "persisted".to_owned(),
        })
        .await
        .map(|_| ())
}

async fn persist_snapshot(
    node_id: &str,
    repository: &RaftRepository,
    snapshot: &Snapshot,
) -> Result<(), scheduler_storage::DbErr> {
    let metadata = snapshot.get_metadata();
    repository
        .upsert_snapshot(UpsertRaftSnapshot {
            cluster_id: CLUSTER_ID.to_owned(),
            node_id: node_id.to_owned(),
            snapshot_index: i64::try_from(metadata.index).unwrap_or(i64::MAX),
            term: i64::try_from(metadata.term).unwrap_or(i64::MAX),
            conf_state: metadata
                .get_conf_state()
                .write_to_bytes()
                .ok()
                .map(|bytes| STANDARD.encode(bytes)),
            data: if snapshot.get_data().is_empty() {
                None
            } else {
                Some(STANDARD.encode(snapshot.get_data()))
            },
        })
        .await
        .map(|_| ())
}

async fn apply_committed_entries(
    node_id: &str,
    repository: &RaftRepository,
    entries: &[Entry],
) -> Result<Option<u64>, scheduler_storage::DbErr> {
    let mut applied = None;
    for entry in entries {
        match entry.get_entry_type() {
            EntryType::EntryNormal => {
                applied = Some(entry.get_index());
            }
            EntryType::EntryConfChange | EntryType::EntryConfChangeV2 => {
                warn!(
                    index = entry.get_index(),
                    term = entry.get_term(),
                    entry_type = ?entry.get_entry_type(),
                    "raft-rs config-change entry reached apply path; dynamic membership is gated"
                );
                break;
            }
        }
    }
    if let Some(applied) = applied {
        repository
            .update_applied_index(node_id, i64::try_from(applied).unwrap_or(i64::MAX))
            .await?;
    }
    Ok(applied)
}

async fn update_runtime_status(
    node_id: &str,
    repository: &RaftRepository,
    node: &RawNode<MemStorage>,
    status: &Arc<RwLock<ClusterStatus>>,
) -> Result<(), scheduler_storage::DbErr> {
    let raft_status = node.status();
    let role = cluster_role_from_raft(raft_status.ss.raft_state);
    let leader_fencing_token = leader_fencing_token_for_role(role, node_id, raft_status.hs.term);
    let persisted = repository
        .update_leader_fencing_token(node_id, leader_fencing_token.clone())
        .await?;
    let persisted_token = persisted.and_then(|metadata| metadata.leader_fencing_token);
    let mut writable = status.write().await;
    writable.role = role;
    writable.can_schedule = role == ClusterRole::Leader
        && leader_fencing_token.is_some()
        && persisted_token == leader_fencing_token;
    writable.leader_fencing_token = persisted_token;
    writable.detail = format!(
        "{RAFT_RS_LIBRARY} runtime active: raft_role={}, term={}, commit={}, applied={}; scheduling requires persisted leader fencing token",
        raft_role_name(raft_status.ss.raft_state),
        raft_status.hs.term,
        raft_status.hs.commit,
        raft_status.applied
    );
    Ok(())
}

async fn persist_role_metadata(
    repository: &RaftRepository,
    node_id: &str,
    _role: ClusterRole,
) -> Result<(), scheduler_storage::DbErr> {
    repository
        .upsert_metadata(UpsertRaftMetadata {
            cluster_id: CLUSTER_ID.to_owned(),
            node_id: node_id.to_owned(),
            current_term: 0,
            voted_for: None,
            commit_index: 0,
            applied_index: 0,
            leader_fencing_token: None,
        })
        .await
        .map(|_| ())
}

fn peer_endpoints(config: &ClusterConfig) -> BTreeMap<u64, String> {
    config
        .peers
        .iter()
        .map(|peer| (raft_numeric_id(&peer.node_id), peer.endpoint.clone()))
        .collect()
}

fn raft_append_entries_url(endpoint: &str) -> String {
    const PATH: &str = "/api/v1/raft/append-entries";
    if endpoint.ends_with(PATH) {
        endpoint.to_owned()
    } else {
        format!("{}{}", endpoint.trim_end_matches('/'), PATH)
    }
}

fn raft_message_to_wire_request(message: &Message) -> RaftWireMessage {
    let snapshot = message.get_snapshot();
    let snapshot_metadata = snapshot.get_metadata();
    RaftWireMessage {
        from: message.from,
        to: message.to,
        term: u64_to_i64(message.term),
        message_type: format!("{:?}", message.get_msg_type()),
        index: u64_to_i64(message.index),
        log_term: u64_to_i64(message.log_term),
        commit: u64_to_i64(message.commit),
        snapshot_index: (!snapshot.is_empty()).then_some(u64_to_i64(snapshot_metadata.index)),
        snapshot_term: (!snapshot.is_empty()).then_some(u64_to_i64(snapshot_metadata.term)),
        entries: message
            .get_entries()
            .iter()
            .map(raft_entry_to_wire_entry)
            .collect(),
        context: if message.get_context().is_empty() {
            None
        } else {
            Some(STANDARD.encode(message.get_context()))
        },
        reject: message.reject,
        reject_hint: (message.reject_hint != 0).then_some(u64_to_i64(message.reject_hint)),
        leader_fencing_token: None,
    }
}

fn raft_entry_to_wire_entry(entry: &Entry) -> RaftWireLogEntry {
    RaftWireLogEntry {
        entry_type: format!("{:?}", entry.get_entry_type()),
        index: u64_to_i64(entry.get_index()),
        term: u64_to_i64(entry.get_term()),
        data: STANDARD.encode(entry.get_data()),
        context: if entry.get_context().is_empty() {
            None
        } else {
            Some(STANDARD.encode(entry.get_context()))
        },
    }
}

fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn leader_fencing_token_for_role(role: ClusterRole, node_id: &str, term: u64) -> Option<String> {
    if role == ClusterRole::Leader && term > 0 {
        Some(format!("raft:term:{term}:node:{node_id}"))
    } else {
        None
    }
}

fn voter_ids(config: &ClusterConfig, local_id: u64) -> Result<Vec<u64>, String> {
    let mut voters = BTreeSet::from([local_id]);
    for peer in &config.peers {
        let peer_id = raft_numeric_id(&peer.node_id);
        if !voters.insert(peer_id) && peer.node_id != config.node_id {
            return Err(format!(
                "raft-rs numeric node id collision for configured peer {}",
                peer.node_id
            ));
        }
    }
    Ok(voters.into_iter().collect())
}

const fn cluster_role_from_raft(role: StateRole) -> ClusterRole {
    match role {
        StateRole::Leader => ClusterRole::Leader,
        StateRole::Follower | StateRole::Candidate | StateRole::PreCandidate => {
            ClusterRole::Follower
        }
    }
}

const fn raft_role_name(role: StateRole) -> &'static str {
    match role {
        StateRole::Follower => "follower",
        StateRole::Candidate => "candidate",
        StateRole::Leader => "leader",
        StateRole::PreCandidate => "pre_candidate",
    }
}

#[cfg(test)]
mod tests {
    use raft::eraftpb::EntryType;
    use scheduler_config::{ClusterConfig, ClusterModeConfig, ClusterPeerConfig};
    use scheduler_storage::{RaftRepository, UpsertRaftMetadata, connect_and_migrate};

    use std::time::Duration;

    use super::{
        CLUSTER_ID, RaftRuntimeCoordinator, apply_committed_entries, leader_fencing_token_for_role,
        raft_append_entries_url, raft_message_to_wire_request, raft_numeric_id,
        validate_raft_rs_bootstrap,
    };
    use crate::cluster::{ClusterMode, ClusterRole};

    #[test]
    fn raft_numeric_id_is_stable_non_zero() {
        let first = raft_numeric_id("scheduler-0");
        let second = raft_numeric_id("scheduler-0");

        assert_ne!(first, 0);
        assert_eq!(first, second);
    }

    #[test]
    fn raft_rs_bootstrap_constructs_raw_node_without_leadership() {
        let config = test_raft_config();

        let status = validate_raft_rs_bootstrap(&config)
            .unwrap_or_else(|error| panic!("raft-rs bootstrap should validate: {error}"));

        assert_eq!(status.node_id, "scheduler-0");
        assert_eq!(status.voter_ids.len(), 2);
        assert_eq!(status.initial_role, "follower");
    }

    #[test]
    fn raft_outbound_wire_request_preserves_message_fields() {
        let mut entry = raft::eraftpb::Entry::new();
        entry.set_entry_type(raft::eraftpb::EntryType::EntryNormal);
        entry.index = 5;
        entry.term = 3;
        entry.data = b"payload".to_vec().into();
        entry.context = b"entry-context".to_vec().into();
        let mut message = raft::eraftpb::Message::new();
        message.set_msg_type(raft::eraftpb::MessageType::MsgAppend);
        message.from = 1;
        message.to = 2;
        message.term = 3;
        message.index = 4;
        message.log_term = 3;
        message.commit = 4;
        message.context = b"message-context".to_vec().into();
        message.set_entries(vec![entry].into());

        let wire = raft_message_to_wire_request(&message);

        assert_eq!(wire.from, 1);
        assert_eq!(wire.to, 2);
        assert_eq!(wire.message_type, "MsgAppend");
        assert_eq!(wire.entries[0].entry_type, "EntryNormal");
        assert_eq!(wire.entries[0].data, "cGF5bG9hZA==");
        assert_eq!(
            wire.entries[0].context.as_deref(),
            Some("ZW50cnktY29udGV4dA==")
        );
        assert_eq!(wire.context.as_deref(), Some("bWVzc2FnZS1jb250ZXh0"));
        assert_eq!(wire.leader_fencing_token, None);
    }

    #[test]
    fn raft_peer_endpoint_adds_append_entries_path_once() {
        assert_eq!(
            raft_append_entries_url("http://scheduler-1:9998"),
            "http://scheduler-1:9998/api/v1/raft/append-entries"
        );
        assert_eq!(
            raft_append_entries_url("http://scheduler-1:9998/api/v1/raft/append-entries"),
            "http://scheduler-1:9998/api/v1/raft/append-entries"
        );
    }

    #[tokio::test]
    async fn raft_apply_committed_entries_updates_applied_index() {
        let repository = test_raft_repository().await;
        let mut first = raft::eraftpb::Entry::new();
        first.set_entry_type(EntryType::EntryNormal);
        first.index = 1;
        first.term = 1;
        let mut second = raft::eraftpb::Entry::new();
        second.set_entry_type(EntryType::EntryNormal);
        second.index = 3;
        second.term = 1;

        let applied = apply_committed_entries("scheduler-0", &repository, &[first, second])
            .await
            .unwrap_or_else(|error| panic!("committed entries should apply: {error}"));
        let metadata = repository
            .get_metadata("scheduler-0")
            .await
            .unwrap_or_else(|error| panic!("metadata should load: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));

        assert_eq!(applied, Some(3));
        assert_eq!(metadata.applied_index, 3);
        assert_eq!(metadata.leader_fencing_token, None);
    }

    #[tokio::test]
    async fn raft_apply_committed_entries_gates_config_changes() {
        let repository = test_raft_repository().await;
        let mut normal = raft::eraftpb::Entry::new();
        normal.set_entry_type(EntryType::EntryNormal);
        normal.index = 4;
        normal.term = 2;
        let mut conf_change = raft::eraftpb::Entry::new();
        conf_change.set_entry_type(EntryType::EntryConfChange);
        conf_change.index = 5;
        conf_change.term = 2;
        let mut after_conf_change = raft::eraftpb::Entry::new();
        after_conf_change.set_entry_type(EntryType::EntryNormal);
        after_conf_change.index = 6;
        after_conf_change.term = 2;

        let applied = apply_committed_entries(
            "scheduler-0",
            &repository,
            &[normal, conf_change, after_conf_change],
        )
        .await
        .unwrap_or_else(|error| panic!("committed entries should gate config changes: {error}"));
        let metadata = repository
            .get_metadata("scheduler-0")
            .await
            .unwrap_or_else(|error| panic!("metadata should load: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));

        assert_eq!(applied, Some(4));
        assert_eq!(metadata.applied_index, 4);
    }

    #[test]
    fn leader_fencing_token_requires_real_leader_role_and_term() {
        assert_eq!(
            leader_fencing_token_for_role(ClusterRole::Leader, "scheduler-0", 7).as_deref(),
            Some("raft:term:7:node:scheduler-0")
        );
        assert_eq!(
            leader_fencing_token_for_role(ClusterRole::Leader, "scheduler-0", 0),
            None
        );
        assert_eq!(
            leader_fencing_token_for_role(ClusterRole::Follower, "scheduler-0", 7),
            None
        );
    }

    #[tokio::test]
    async fn raft_runtime_starts_ticker_without_granting_scheduler_ownership() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should initialize: {error}"));
        let repository = RaftRepository::new(db);
        let coordinator = RaftRuntimeCoordinator::start(&test_raft_config(), repository)
            .await
            .unwrap_or_else(|error| panic!("raft runtime should start: {error}"));

        tokio::time::sleep(Duration::from_millis(150)).await;
        let status = coordinator.status().await;

        assert_eq!(status.mode, ClusterMode::Raft);
        assert_eq!(status.role, ClusterRole::Follower);
        assert!(!status.can_schedule);
        assert_eq!(status.leader_fencing_token, None);
        assert!(status.detail.contains("runtime active"));
    }

    #[tokio::test]
    async fn raft_runtime_accepts_inbound_messages_into_inbox_only() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should initialize: {error}"));
        let repository = RaftRepository::new(db);
        let coordinator = RaftRuntimeCoordinator::start(&test_raft_config(), repository)
            .await
            .unwrap_or_else(|error| panic!("raft runtime should start: {error}"));

        let mut message = raft::eraftpb::Message::new();
        message.set_msg_type(raft::eraftpb::MessageType::MsgHeartbeat);
        message.from = raft_numeric_id("scheduler-1");
        message.to = raft_numeric_id("scheduler-0");
        message.term = 1;
        let submission = coordinator.submit_raft_message(message).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        let status = coordinator.status().await;

        assert!(submission.accepted);
        assert!(submission.reason.contains("enqueued"));
        assert!(!status.can_schedule);
        assert_eq!(status.leader_fencing_token, None);
    }

    fn test_raft_config() -> ClusterConfig {
        ClusterConfig {
            mode: ClusterModeConfig::Raft,
            node_id: "scheduler-0".to_owned(),
            peers: vec![
                ClusterPeerConfig {
                    node_id: "scheduler-0".to_owned(),
                    endpoint: "http://scheduler-0.scheduler-headless:9999".to_owned(),
                },
                ClusterPeerConfig {
                    node_id: "scheduler-1".to_owned(),
                    endpoint: "http://scheduler-1.scheduler-headless:9999".to_owned(),
                },
            ],
            transport_token: None,
        }
    }

    async fn test_raft_repository() -> RaftRepository {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should initialize: {error}"));
        let repository = RaftRepository::new(db);
        repository
            .upsert_metadata(UpsertRaftMetadata {
                cluster_id: CLUSTER_ID.to_owned(),
                node_id: "scheduler-0".to_owned(),
                current_term: 1,
                voted_for: None,
                commit_index: 0,
                applied_index: 0,
                leader_fencing_token: None,
            })
            .await
            .unwrap_or_else(|error| panic!("metadata should upsert: {error}"));
        repository
    }
}
