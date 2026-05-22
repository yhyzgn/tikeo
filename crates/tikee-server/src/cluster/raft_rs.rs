//! `TiKV` raft-rs bootstrap and runtime integration.
//!
//! This module validates the crate/config/storage boundary and hosts the first runtime
//! ticker. It deliberately does not campaign or grant tikee ownership until real
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
    eraftpb::{
        ConfChange, ConfChangeType, ConfChangeV2, ConfState, Entry, EntryType, HardState, Message,
        Snapshot,
    },
    raw_node::RawNode,
    storage::MemStorage,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tikee_config::ClusterConfig;
use tikee_storage::{
    RaftLogEntrySummary, RaftRepository, RecordRaftAppliedCommand, UpsertRaftLogEntry,
    UpsertRaftMember, UpsertRaftMetadata, UpsertRaftSnapshot,
};
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use tracing::{debug, warn};
use url::Url;

use super::{
    ClusterMode, ClusterRole, ClusterStatus, RaftMembershipProposal,
    RaftMembershipProposalSubmission, RaftMessageSubmission, SharedClusterCoordinator,
};

/// Crate-level runtime library label exposed in diagnostics and design docs.
pub const RAFT_RS_LIBRARY: &str = "tikv/raft-rs crate raft 0.7.0";

const CLUSTER_ID: &str = "default";
const TICK_INTERVAL: Duration = Duration::from_millis(100);
const INBOX_CAPACITY: usize = 256;

/// Safe bootstrap status produced by constructing a raft-rs `RawNode` without driving it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftRsBootstrapStatus {
    /// Tikee string node id from config.
    pub node_id: String,
    /// Deterministic raft-rs numeric node id derived from `node_id`.
    pub raft_node_id: u64,
    /// Numeric voter ids derived from configured peers.
    pub voter_ids: Vec<u64>,
    /// Initial raft-rs role after construction; must not be treated as tikee ownership.
    pub initial_role: String,
    /// Whether a freshly constructed node reports ready work before the event loop starts.
    pub has_ready: bool,
    /// Number of persisted log entries restored into `MemStorage` before runtime start.
    pub restored_entries: usize,
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

#[derive(Debug, Clone, Deserialize)]
struct TikeeRaftCommand {
    command_id: String,
    command_type: String,
    #[serde(default)]
    payload: serde_json::Value,
}

#[derive(Debug)]
enum RaftRuntimeCommand {
    Message(Message),
    MembershipProposal {
        proposal: RaftMembershipProposal,
        respond_to: oneshot::Sender<RaftMembershipProposalSubmission>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RaftMembershipProposalContext {
    proposal_id: String,
    action: String,
    node_id: String,
    endpoint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RaftMemberUpsertPayload {
    node_id: String,
    endpoint: String,
    status: String,
}

#[derive(Debug, Clone)]
enum RaftCommandApply {
    Noop {
        command_id: String,
        payload: Option<String>,
        message: String,
    },
    MemberUpsert {
        command_id: String,
        payload: RaftMemberUpsertPayload,
        payload_json: String,
    },
    DeferredUnsupported {
        command_id: String,
        command_type: String,
        payload: Option<String>,
    },
    Rejected {
        command_id: String,
        command_type: String,
        payload: Option<String>,
        message: String,
    },
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
                    builder = builder.header("x-tikee-raft-token", token);
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
    inbox: mpsc::Sender<RaftRuntimeCommand>,
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
    ) -> Result<SharedClusterCoordinator, tikee_storage::DbErr> {
        let initial = build_runtime_from_repository(config, &repository).await;
        let (role, detail, node, transport) = match initial {
            Ok((node, bootstrap, transport)) => (
                ClusterRole::Follower,
                format!(
                    "{RAFT_RS_LIBRARY} runtime ticker started: raft_node_id={}, voters={}, initial_role={}, has_ready={}, restored_entries={}; no campaign, stale leader fencing cleared until role is re-observed",
                    bootstrap.raft_node_id,
                    bootstrap.voter_ids.len(),
                    bootstrap.initial_role,
                    bootstrap.has_ready,
                    bootstrap.restored_entries
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
        match self.inbox.try_send(RaftRuntimeCommand::Message(message)) {
            Ok(()) => RaftMessageSubmission::accepted(message_type),
            Err(mpsc::error::TrySendError::Full(_)) => RaftMessageSubmission::unavailable(
                "raft-rs runtime inbox is full; retry after the local node drains messages",
            ),
            Err(mpsc::error::TrySendError::Closed(_)) => RaftMessageSubmission::unavailable(
                "raft-rs runtime inbox is closed because bootstrap failed or runtime stopped",
            ),
        }
    }

    async fn propose_membership_change(
        &self,
        proposal: RaftMembershipProposal,
    ) -> RaftMembershipProposalSubmission {
        let (tx, rx) = oneshot::channel();
        match self.inbox.try_send(RaftRuntimeCommand::MembershipProposal {
            proposal,
            respond_to: tx,
        }) {
            Ok(()) => rx.await.unwrap_or_else(|_| {
                RaftMembershipProposalSubmission::unavailable(
                    "raft-rs membership proposal runtime stopped before responding",
                )
            }),
            Err(mpsc::error::TrySendError::Full(_)) => {
                RaftMembershipProposalSubmission::unavailable(
                    "raft-rs runtime inbox is full; retry membership proposal later",
                )
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                RaftMembershipProposalSubmission::unavailable(
                    "raft-rs runtime inbox is closed because bootstrap failed or runtime stopped",
                )
            }
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
            restored_entries: 0,
        },
        RaftPeerTransport::new(endpoints, config.transport_token.clone()),
    ))
}

async fn build_runtime_from_repository(
    config: &ClusterConfig,
    repository: &RaftRepository,
) -> Result<
    (
        RawNode<MemStorage>,
        RaftRsBootstrapStatus,
        RaftPeerTransport,
    ),
    String,
> {
    let (mut node, mut status, transport) = build_runtime(config)?;
    status.restored_entries = restore_persisted_storage(&config.node_id, repository, &mut node)
        .await
        .map_err(|error| format!("raft-rs persisted storage restore failed: {error}"))?;
    // A process restart must not inherit stale tikee authority. The next observed
    // raft-rs role will regenerate/persist a token only if this node is still leader.
    repository
        .update_leader_fencing_token(&config.node_id, None)
        .await
        .map_err(|error| format!("raft-rs stale fencing clear failed: {error}"))?;
    Ok((node, status, transport))
}

async fn restore_persisted_storage(
    node_id: &str,
    repository: &RaftRepository,
    node: &mut RawNode<MemStorage>,
) -> Result<usize, tikee_storage::DbErr> {
    let Some(metadata) = repository.get_metadata(node_id).await? else {
        return Ok(0);
    };
    if let Some(conf_state) = metadata.conf_state.as_deref() {
        let conf_state = decode_conf_state(conf_state)?;
        node.raft.mut_store().wl().set_conf_state(conf_state);
    }
    let entries = repository.list_log_entries(node_id, 1, 10_000).await?;
    let restored = entries.len();
    let entries = entries
        .iter()
        .map(stored_log_entry_to_raft)
        .collect::<Result<Vec<_>, _>>()?;
    if !entries.is_empty() {
        node.raft
            .mut_store()
            .wl()
            .append(&entries)
            .map_err(|error| {
                tikee_storage::DbErr::Custom(format!(
                    "raft-rs persisted MemStorage append failed: {error}"
                ))
            })?;
    }
    let mut hard_state = HardState::new();
    hard_state.set_term(u64::try_from(metadata.current_term.max(0)).unwrap_or(u64::MAX));
    hard_state.set_vote(
        metadata
            .voted_for
            .as_deref()
            .and_then(|vote| vote.parse::<u64>().ok())
            .unwrap_or(0),
    );
    hard_state.set_commit(u64::try_from(metadata.commit_index.max(0)).unwrap_or(u64::MAX));
    node.raft.mut_store().wl().set_hardstate(hard_state);
    Ok(restored)
}

fn stored_log_entry_to_raft(row: &RaftLogEntrySummary) -> Result<Entry, tikee_storage::DbErr> {
    let mut entry = Entry::new();
    entry.set_index(u64::try_from(row.log_index.max(0)).unwrap_or(u64::MAX));
    entry.set_term(u64::try_from(row.term.max(0)).unwrap_or(u64::MAX));
    entry.set_entry_type(stored_entry_type(&row.entry_type)?);
    let data = STANDARD.decode(&row.data).map_err(|error| {
        tikee_storage::DbErr::Custom(format!("raft log entry data base64 invalid: {error}"))
    })?;
    entry.set_data(data.into());
    if let Some(context) = row.context.as_deref() {
        let context = STANDARD.decode(context).map_err(|error| {
            tikee_storage::DbErr::Custom(format!("raft log entry context base64 invalid: {error}"))
        })?;
        entry.set_context(context.into());
    }
    Ok(entry)
}

fn stored_entry_type(entry_type: &str) -> Result<EntryType, tikee_storage::DbErr> {
    match entry_type {
        "EntryNormal" => Ok(EntryType::EntryNormal),
        "EntryConfChange" => Ok(EntryType::EntryConfChange),
        "EntryConfChangeV2" => Ok(EntryType::EntryConfChangeV2),
        other => Err(tikee_storage::DbErr::Custom(format!(
            "unsupported persisted raft entry type: {other}"
        ))),
    }
}

fn decode_conf_state(conf_state: &str) -> Result<ConfState, tikee_storage::DbErr> {
    let bytes = STANDARD.decode(conf_state).map_err(|error| {
        tikee_storage::DbErr::Custom(format!("raft conf_state base64 invalid: {error}"))
    })?;
    let mut decoded = ConfState::new();
    decoded.merge_from_bytes(&bytes).map_err(|error| {
        tikee_storage::DbErr::Custom(format!("raft conf_state protobuf invalid: {error}"))
    })?;
    Ok(decoded)
}

fn spawn_runtime_loop(
    node_id: String,
    status: Arc<RwLock<ClusterStatus>>,
    repository: RaftRepository,
    node: RawNode<MemStorage>,
    transport: RaftPeerTransport,
    inbox: mpsc::Receiver<RaftRuntimeCommand>,
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
    mut inbox: mpsc::Receiver<RaftRuntimeCommand>,
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
            command = inbox.recv() => {
                let Some(command) = command else { break; };
                match command {
                    RaftRuntimeCommand::Message(message) => {
                        let mut guard = node.lock().await;
                        if let Err(error) = guard.step(message) {
                            warn!(%error, "raft-rs message step failed");
                        }
                        if let Err(error) = process_ready(&node_id, &repository, &mut guard, &status, &transport).await {
                            warn!(%error, "raft-rs Ready processing failed");
                        }
                    }
                    RaftRuntimeCommand::MembershipProposal { proposal, respond_to } => {
                        let mut guard = node.lock().await;
                        let response = propose_membership_change_to_runtime(
                            &node_id,
                            &repository,
                            &mut guard,
                            &status,
                            &transport,
                            proposal,
                        )
                        .await;
                        let _ = respond_to.send(response);
                    }
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
) -> Result<(), tikee_storage::DbErr> {
    if !node.has_ready() {
        update_runtime_status(node_id, repository, node, status).await?;
        return Ok(());
    }

    let ready = node.ready();
    if let Some(hard_state) = ready.hs() {
        persist_hard_state(node_id, repository, hard_state).await?;
        node.raft.mut_store().wl().set_hardstate(hard_state.clone());
    }
    for entry in ready.entries() {
        persist_entry(node_id, repository, entry).await?;
    }
    node.raft
        .mut_store()
        .wl()
        .append(ready.entries())
        .map_err(|error| {
            tikee_storage::DbErr::Custom(format!("raft-rs MemStorage append failed: {error}"))
        })?;
    if !ready.snapshot().is_empty() {
        persist_snapshot(node_id, repository, ready.snapshot()).await?;
        node.raft
            .mut_store()
            .wl()
            .apply_snapshot(ready.snapshot().clone())
            .map_err(|error| {
                tikee_storage::DbErr::Custom(format!(
                    "raft-rs MemStorage snapshot apply failed: {error}"
                ))
            })?;
    }
    transport.dispatch_ready_messages(ready.messages());
    transport.dispatch_ready_messages(ready.persisted_messages());
    let applied =
        apply_committed_entries(node_id, repository, Some(node), ready.committed_entries()).await?;
    let light_ready = node.advance_append(ready);
    if let Some(commit) = light_ready.commit_index() {
        node.raft
            .mut_store()
            .wl()
            .mut_hard_state()
            .set_commit(commit);
    }
    if let Some(applied) = applied {
        node.advance_apply_to(applied);
    }
    transport.dispatch_ready_messages(light_ready.messages());
    let light_applied = apply_committed_entries(
        node_id,
        repository,
        Some(node),
        light_ready.committed_entries(),
    )
    .await?;
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
) -> Result<(), tikee_storage::DbErr> {
    let existing = repository.get_metadata(node_id).await?;
    let applied_index = existing.as_ref().map_or(0, |item| item.applied_index);
    let leader_fencing_token = existing
        .as_ref()
        .and_then(|item| item.leader_fencing_token.clone());
    let conf_state = existing.and_then(|item| item.conf_state);
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
            applied_index,
            leader_fencing_token,
            conf_state,
        })
        .await
        .map(|_| ())
}

async fn persist_entry(
    node_id: &str,
    repository: &RaftRepository,
    entry: &Entry,
) -> Result<(), tikee_storage::DbErr> {
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
) -> Result<(), tikee_storage::DbErr> {
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
    mut node: Option<&mut RawNode<MemStorage>>,
    entries: &[Entry],
) -> Result<Option<u64>, tikee_storage::DbErr> {
    let mut applied = None;
    for entry in entries {
        match entry.get_entry_type() {
            EntryType::EntryNormal => {
                record_normal_command(node_id, repository, entry).await?;
                applied = Some(entry.get_index());
            }
            EntryType::EntryConfChange | EntryType::EntryConfChangeV2 => {
                if apply_config_change_entry(node_id, repository, node.as_deref_mut(), entry)
                    .await?
                {
                    applied = Some(entry.get_index());
                } else {
                    break;
                }
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

async fn record_normal_command(
    node_id: &str,
    repository: &RaftRepository,
    entry: &Entry,
) -> Result<(), tikee_storage::DbErr> {
    let command = parse_entry_command(entry);
    if let Some(existing) = repository
        .get_applied_command_by_command_id(CLUSTER_ID, command.command_id())
        .await?
    {
        debug!(
            command_id = existing.command_id,
            original_log_index = existing.log_index,
            replay_log_index = entry.get_index(),
            "raft-rs command replay skipped by command_id idempotency"
        );
        return Ok(());
    }

    let (command_id, command_type, payload, status, message) = match command {
        RaftCommandApply::Noop {
            command_id,
            payload,
            message,
        } => (
            command_id,
            "noop".to_owned(),
            payload,
            "applied".to_owned(),
            message,
        ),
        RaftCommandApply::MemberUpsert {
            command_id,
            payload,
            payload_json,
        } => {
            repository
                .upsert_member(UpsertRaftMember {
                    node_id: payload.node_id,
                    endpoint: payload.endpoint,
                    status: payload.status,
                })
                .await?;
            (
                command_id,
                "raft_member_upsert".to_owned(),
                Some(payload_json),
                "applied".to_owned(),
                "raft member catalog metadata upserted idempotently; raft ConfChange remains gated"
                    .to_owned(),
            )
        }
        RaftCommandApply::DeferredUnsupported {
            command_id,
            command_type,
            payload,
        } => (
            command_id,
            command_type,
            payload,
            "deferred_unsupported".to_owned(),
            "command envelope recorded but business apply is not implemented for this type"
                .to_owned(),
        ),
        RaftCommandApply::Rejected {
            command_id,
            command_type,
            payload,
            message,
        } => (
            command_id,
            command_type,
            payload,
            "rejected".to_owned(),
            message,
        ),
    };
    repository
        .record_applied_command(RecordRaftAppliedCommand {
            cluster_id: CLUSTER_ID.to_owned(),
            node_id: node_id.to_owned(),
            log_index: i64::try_from(entry.get_index()).unwrap_or(i64::MAX),
            term: i64::try_from(entry.get_term()).unwrap_or(i64::MAX),
            command_id,
            command_type,
            payload,
            status,
            message,
        })
        .await
        .map(|_| ())
}

fn build_membership_conf_change(proposal: &RaftMembershipProposal) -> Result<ConfChange, String> {
    let context = RaftMembershipProposalContext {
        proposal_id: proposal.proposal_id.clone(),
        action: proposal.action.clone(),
        node_id: proposal.node_id.clone(),
        endpoint: proposal.endpoint.clone(),
    };
    let context_bytes = serde_json::to_vec(&context)
        .map_err(|error| format!("membership proposal context serialization failed: {error}"))?;
    let change_type = membership_action_to_conf_change_type(&proposal.action)
        .ok_or_else(|| format!("unsupported membership action: {}", proposal.action))?;
    let mut conf_change = ConfChange::new();
    conf_change.set_change_type(change_type);
    conf_change.node_id = raft_numeric_id(&proposal.node_id);
    conf_change.context = context_bytes.into();
    Ok(conf_change)
}

async fn propose_membership_change_to_runtime(
    node_id: &str,
    repository: &RaftRepository,
    node: &mut RawNode<MemStorage>,
    status: &Arc<RwLock<ClusterStatus>>,
    transport: &RaftPeerTransport,
    proposal: RaftMembershipProposal,
) -> RaftMembershipProposalSubmission {
    let runtime_status = status.read().await.clone();
    if runtime_status.role != ClusterRole::Leader
        || !runtime_status.can_schedule
        || runtime_status
            .leader_fencing_token
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return RaftMembershipProposalSubmission::unavailable(
            "membership proposals require a real raft leader with a persisted fencing token",
        );
    }

    let conf_change = match build_membership_conf_change(&proposal) {
        Ok(conf_change) => conf_change,
        Err(error) => {
            return RaftMembershipProposalSubmission::unavailable(format!(
                "membership proposal conversion failed: {error}"
            ));
        }
    };

    if let Err(error) =
        node.propose_conf_change(conf_change.get_context().to_vec(), conf_change.clone())
    {
        return RaftMembershipProposalSubmission::unavailable(format!(
            "raft-rs propose_conf_change rejected membership proposal: {error}"
        ));
    }
    if let Err(error) = process_ready(node_id, repository, node, status, transport).await {
        return RaftMembershipProposalSubmission::unavailable(format!(
            "raft-rs Ready processing failed after membership proposal: {error}"
        ));
    }
    RaftMembershipProposalSubmission::accepted(
        "membership proposal submitted to raft-rs propose_conf_change",
    )
}

async fn apply_config_change_entry(
    node_id: &str,
    repository: &RaftRepository,
    node: Option<&mut RawNode<MemStorage>>,
    entry: &Entry,
) -> Result<bool, tikee_storage::DbErr> {
    let Some(node) = node else {
        warn!(
            index = entry.get_index(),
            term = entry.get_term(),
            entry_type = ?entry.get_entry_type(),
            "raft-rs config-change entry reached apply path without runtime node; dynamic membership remains gated"
        );
        return Ok(false);
    };

    let decoded = match decode_config_change_entry(entry) {
        Ok(decoded) => decoded,
        Err(message) => {
            warn!(
                index = entry.get_index(),
                term = entry.get_term(),
                %message,
                "raft-rs config-change entry rejected before apply"
            );
            return Ok(true);
        }
    };
    let Some(context) = decoded.context else {
        warn!(
            index = entry.get_index(),
            term = entry.get_term(),
            "raft-rs config-change entry rejected because proposal context is missing or invalid"
        );
        return Ok(true);
    };

    let conf_state = match decoded.change {
        DecodedConfChange::V1(conf_change) => match node.apply_conf_change(&conf_change) {
            Ok(conf_state) => conf_state,
            Err(error) => {
                mark_membership_proposal_rejected(repository, &context, &format!("{error}"))
                    .await?;
                return Ok(true);
            }
        },
        DecodedConfChange::V2(conf_change) => match node.apply_conf_change(&conf_change) {
            Ok(conf_state) => conf_state,
            Err(error) => {
                mark_membership_proposal_rejected(repository, &context, &format!("{error}"))
                    .await?;
                return Ok(true);
            }
        },
    };
    let conf_state_bytes = conf_state.write_to_bytes().unwrap_or_default();
    repository
        .update_conf_state(node_id, STANDARD.encode(conf_state_bytes))
        .await?;
    node.raft.mut_store().wl().set_conf_state(conf_state);
    apply_membership_context(repository, &context).await?;
    repository
        .update_membership_proposal_status(
            CLUSTER_ID,
            &context.proposal_id,
            "applied",
            "committed raft-rs ConfChange applied and ConfState persisted",
        )
        .await?;
    Ok(true)
}

#[derive(Debug)]
enum DecodedConfChange {
    V1(ConfChange),
    V2(ConfChangeV2),
}

#[derive(Debug)]
struct DecodedConfChangeEntry {
    change: DecodedConfChange,
    context: Option<RaftMembershipProposalContext>,
}

fn decode_config_change_entry(entry: &Entry) -> Result<DecodedConfChangeEntry, String> {
    match entry.get_entry_type() {
        EntryType::EntryConfChange => {
            let mut conf_change = ConfChange::new();
            conf_change
                .merge_from_bytes(entry.get_data())
                .map_err(|error| format!("invalid ConfChange payload: {error}"))?;
            let context = decode_membership_context(conf_change.get_context())
                .or_else(|| decode_membership_context(entry.get_context()));
            Ok(DecodedConfChangeEntry {
                change: DecodedConfChange::V1(conf_change),
                context,
            })
        }
        EntryType::EntryConfChangeV2 => {
            let mut conf_change = ConfChangeV2::new();
            conf_change
                .merge_from_bytes(entry.get_data())
                .map_err(|error| format!("invalid ConfChangeV2 payload: {error}"))?;
            if conf_change.get_changes().len() != 1 {
                return Err(
                    "only one-at-a-time ConfChangeV2 membership entries are supported".to_owned(),
                );
            }
            let context = decode_membership_context(conf_change.get_context())
                .or_else(|| decode_membership_context(entry.get_context()));
            Ok(DecodedConfChangeEntry {
                change: DecodedConfChange::V2(conf_change),
                context,
            })
        }
        EntryType::EntryNormal => Err("normal entries are not config changes".to_owned()),
    }
}

fn decode_membership_context(bytes: &[u8]) -> Option<RaftMembershipProposalContext> {
    if bytes.is_empty() {
        return None;
    }
    serde_json::from_slice(bytes).ok()
}

async fn apply_membership_context(
    repository: &RaftRepository,
    context: &RaftMembershipProposalContext,
) -> Result<(), tikee_storage::DbErr> {
    match context.action.as_str() {
        "add_voter" => {
            if let Some(endpoint) = &context.endpoint {
                repository
                    .upsert_member(UpsertRaftMember {
                        node_id: context.node_id.clone(),
                        endpoint: endpoint.clone(),
                        status: "active".to_owned(),
                    })
                    .await?;
            } else if let Some(existing) = repository.get_member(&context.node_id).await? {
                repository
                    .upsert_member(UpsertRaftMember {
                        node_id: context.node_id.clone(),
                        endpoint: existing.endpoint,
                        status: "active".to_owned(),
                    })
                    .await?;
            }
        }
        "remove_voter" => {
            let endpoint = repository
                .get_member(&context.node_id)
                .await?
                .and_then(|member| (!member.endpoint.is_empty()).then_some(member.endpoint))
                .or_else(|| context.endpoint.clone())
                .unwrap_or_default();
            repository
                .upsert_member(UpsertRaftMember {
                    node_id: context.node_id.clone(),
                    endpoint,
                    status: "removed".to_owned(),
                })
                .await?;
        }
        _ => {}
    }
    Ok(())
}

async fn mark_membership_proposal_rejected(
    repository: &RaftRepository,
    context: &RaftMembershipProposalContext,
    reason: &str,
) -> Result<(), tikee_storage::DbErr> {
    repository
        .update_membership_proposal_status(
            CLUSTER_ID,
            &context.proposal_id,
            "rejected",
            &format!("committed raft-rs ConfChange rejected: {reason}"),
        )
        .await?;
    Ok(())
}

fn membership_action_to_conf_change_type(action: &str) -> Option<ConfChangeType> {
    match action {
        "add_voter" => Some(ConfChangeType::AddNode),
        "remove_voter" => Some(ConfChangeType::RemoveNode),
        _ => None,
    }
}

fn parse_entry_command(entry: &Entry) -> RaftCommandApply {
    if entry.get_data().is_empty() {
        return RaftCommandApply::Noop {
            command_id: format!("raft-noop-{}", entry.get_index()),
            payload: None,
            message: "empty raft entry treated as noop".to_owned(),
        };
    }

    match serde_json::from_slice::<TikeeRaftCommand>(entry.get_data()) {
        Ok(command) => parse_tikee_command(command, entry.get_index()),
        Err(error) => RaftCommandApply::Rejected {
            command_id: format!("raft-invalid-{}", entry.get_index()),
            command_type: "invalid_json".to_owned(),
            payload: Some(STANDARD.encode(entry.get_data())),
            message: format!("invalid command envelope JSON: {error}"),
        },
    }
}

fn parse_tikee_command(command: TikeeRaftCommand, log_index: u64) -> RaftCommandApply {
    let command_id = command.command_id.trim().to_owned();
    let command_type = command.command_type.trim().to_owned();
    if command_id.is_empty() {
        return RaftCommandApply::Rejected {
            command_id: format!("raft-rejected-{log_index}"),
            command_type: command_type_or_invalid(&command_type),
            payload: Some(command.payload.to_string()),
            message: "command_id must not be empty".to_owned(),
        };
    }
    if command_type.is_empty() {
        return RaftCommandApply::Rejected {
            command_id,
            command_type: "invalid_command_type".to_owned(),
            payload: Some(command.payload.to_string()),
            message: "command_type must not be empty".to_owned(),
        };
    }

    match command_type.as_str() {
        "noop" => RaftCommandApply::Noop {
            command_id,
            payload: Some(command.payload.to_string()),
            message: "noop command applied idempotently".to_owned(),
        },
        "raft_member_upsert" => {
            let payload_json = command.payload.to_string();
            match parse_member_upsert_payload(command.payload) {
                Ok((payload, payload_json)) => RaftCommandApply::MemberUpsert {
                    command_id,
                    payload,
                    payload_json,
                },
                Err(message) => RaftCommandApply::Rejected {
                    command_id,
                    command_type,
                    payload: Some(payload_json),
                    message,
                },
            }
        }
        _ => RaftCommandApply::DeferredUnsupported {
            command_id,
            command_type,
            payload: Some(command.payload.to_string()),
        },
    }
}

fn parse_member_upsert_payload(
    payload: serde_json::Value,
) -> Result<(RaftMemberUpsertPayload, String), String> {
    let payload_json = payload.to_string();
    let payload = serde_json::from_value::<RaftMemberUpsertPayload>(payload)
        .map_err(|error| format!("raft_member_upsert payload is invalid: {error}"))?;
    validate_member_node_id(&payload.node_id)?;
    validate_member_endpoint(&payload.endpoint)?;
    validate_member_status(&payload.status)?;
    Ok((payload, payload_json))
}

fn validate_member_node_id(node_id: &str) -> Result<(), String> {
    if node_id.trim().is_empty() {
        return Err("raft_member_upsert node_id must not be empty".to_owned());
    }
    Ok(())
}

fn validate_member_endpoint(endpoint: &str) -> Result<(), String> {
    let parsed = Url::parse(endpoint)
        .map_err(|error| format!("raft_member_upsert endpoint must be an absolute URL: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("raft_member_upsert endpoint must use http or https".to_owned());
    }
    if parsed.host_str().is_none() {
        return Err("raft_member_upsert endpoint must include a host".to_owned());
    }
    Ok(())
}

fn validate_member_status(status: &str) -> Result<(), String> {
    if matches!(
        status,
        "configured" | "joining" | "active" | "leaving" | "removed"
    ) {
        Ok(())
    } else {
        Err(
            "raft_member_upsert status must be one of configured, joining, active, leaving, removed"
                .to_owned(),
        )
    }
}

fn command_type_or_invalid(command_type: &str) -> String {
    if command_type.is_empty() {
        "invalid_command_type".to_owned()
    } else {
        command_type.to_owned()
    }
}

impl RaftCommandApply {
    fn command_id(&self) -> &str {
        match self {
            Self::Noop { command_id, .. }
            | Self::MemberUpsert { command_id, .. }
            | Self::DeferredUnsupported { command_id, .. }
            | Self::Rejected { command_id, .. } => command_id,
        }
    }
}

async fn update_runtime_status(
    node_id: &str,
    repository: &RaftRepository,
    node: &RawNode<MemStorage>,
    status: &Arc<RwLock<ClusterStatus>>,
) -> Result<(), tikee_storage::DbErr> {
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
) -> Result<(), tikee_storage::DbErr> {
    if repository.get_metadata(node_id).await?.is_some() {
        repository
            .update_leader_fencing_token(node_id, None)
            .await?;
        return Ok(());
    }
    repository
        .upsert_metadata(UpsertRaftMetadata {
            cluster_id: CLUSTER_ID.to_owned(),
            node_id: node_id.to_owned(),
            current_term: 0,
            voted_for: None,
            commit_index: 0,
            applied_index: 0,
            leader_fencing_token: None,
            conf_state: None,
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
    use base64::Engine as _;
    use protobuf::Message as _;
    use raft::{GetEntriesContext, Storage, eraftpb::EntryType};
    use tikee_config::{ClusterConfig, ClusterModeConfig, ClusterPeerConfig};
    use tikee_storage::{
        RaftRepository, UpsertRaftLogEntry, UpsertRaftMetadata, connect_and_migrate,
    };

    use std::{collections::BTreeMap, sync::Arc, time::Duration};
    use tokio::sync::RwLock;

    use super::{
        CLUSTER_ID, RaftMembershipProposalContext, RaftRuntimeCoordinator, STANDARD, StateRole,
        apply_committed_entries, build_membership_conf_change, build_runtime_from_repository,
        leader_fencing_token_for_role, persist_entry, persist_hard_state, raft_append_entries_url,
        raft_message_to_wire_request, raft_numeric_id, update_runtime_status,
        validate_raft_rs_bootstrap,
    };
    use crate::cluster::{ClusterMode, ClusterRole, ClusterStatus, RaftMembershipProposal};

    #[test]
    fn raft_numeric_id_is_stable_non_zero() {
        let first = raft_numeric_id("tikee-0");
        let second = raft_numeric_id("tikee-0");

        assert_ne!(first, 0);
        assert_eq!(first, second);
    }

    #[test]
    fn raft_rs_bootstrap_constructs_raw_node_without_leadership() {
        let config = test_raft_config();

        let status = validate_raft_rs_bootstrap(&config)
            .unwrap_or_else(|error| panic!("raft-rs bootstrap should validate: {error}"));

        assert_eq!(status.node_id, "tikee-0");
        assert_eq!(status.voter_ids.len(), 2);
        assert_eq!(status.initial_role, "follower");
    }

    #[tokio::test]
    async fn raft_runtime_restore_replays_persisted_metadata_and_clears_stale_fencing() {
        let repository = test_raft_repository_for("tikee-0").await;
        repository
            .upsert_metadata(UpsertRaftMetadata {
                cluster_id: CLUSTER_ID.to_owned(),
                node_id: "tikee-0".to_owned(),
                current_term: 7,
                voted_for: Some("42".to_owned()),
                commit_index: 2,
                applied_index: 1,
                leader_fencing_token: Some("raft:term:7:node:tikee-0".to_owned()),
                conf_state: None,
            })
            .await
            .unwrap_or_else(|error| panic!("metadata should persist: {error}"));
        for index in 1..=2 {
            repository
                .upsert_log_entry(UpsertRaftLogEntry {
                    cluster_id: CLUSTER_ID.to_owned(),
                    node_id: "tikee-0".to_owned(),
                    log_index: index,
                    term: 7,
                    entry_type: "EntryNormal".to_owned(),
                    data: STANDARD.encode(format!("entry-{index}")),
                    context: None,
                    sync_status: "persisted".to_owned(),
                })
                .await
                .unwrap_or_else(|error| panic!("log entry should persist: {error}"));
        }

        let (node, bootstrap, _transport) =
            build_runtime_from_repository(&test_raft_config(), &repository)
                .await
                .unwrap_or_else(|error| panic!("runtime should restore persisted state: {error}"));
        let initial_state = node
            .store()
            .initial_state()
            .unwrap_or_else(|error| panic!("initial state should load: {error}"));
        let entries = node
            .store()
            .entries(1, 3, None, GetEntriesContext::empty(false))
            .unwrap_or_else(|error| panic!("entries should restore: {error}"));
        let metadata = repository
            .get_metadata("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("metadata should load: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));

        assert_eq!(bootstrap.restored_entries, 2);
        assert_eq!(initial_state.hard_state.term, 7);
        assert_eq!(initial_state.hard_state.vote, 42);
        assert_eq!(initial_state.hard_state.commit, 2);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].get_data(), b"entry-1");
        assert_eq!(metadata.leader_fencing_token, None);
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
            raft_append_entries_url("http://tikee-1:9998"),
            "http://tikee-1:9998/api/v1/raft/append-entries"
        );
        assert_eq!(
            raft_append_entries_url("http://tikee-1:9998/api/v1/raft/append-entries"),
            "http://tikee-1:9998/api/v1/raft/append-entries"
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

        let applied = apply_committed_entries("tikee-0", &repository, None, &[first, second])
            .await
            .unwrap_or_else(|error| panic!("committed entries should apply: {error}"));
        let metadata = repository
            .get_metadata("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("metadata should load: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));

        assert_eq!(applied, Some(3));
        assert_eq!(metadata.applied_index, 3);
        assert_eq!(metadata.leader_fencing_token, None);
        let commands = repository
            .list_applied_commands("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("applied commands should list: {error}"));
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command_type, "noop");
        assert_eq!(commands[0].status, "applied");
    }

    #[tokio::test]
    async fn raft_apply_committed_entries_records_noop_command_envelope() {
        let repository = test_raft_repository().await;
        let mut entry = raft::eraftpb::Entry::new();
        entry.set_entry_type(EntryType::EntryNormal);
        entry.index = 7;
        entry.term = 3;
        entry.data =
            br#"{"command_id":"cmd-noop-1","command_type":"noop","payload":{"source":"test"}}"#
                .to_vec()
                .into();

        let applied = apply_committed_entries("tikee-0", &repository, None, &[entry])
            .await
            .unwrap_or_else(|error| panic!("noop command should apply: {error}"));
        let commands = repository
            .list_applied_commands("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("applied commands should list: {error}"));

        assert_eq!(applied, Some(7));
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command_id, "cmd-noop-1");
        assert_eq!(commands[0].command_type, "noop");
        assert_eq!(commands[0].status, "applied");
        assert_eq!(commands[0].payload.as_deref(), Some(r#"{"source":"test"}"#));
    }

    #[tokio::test]
    async fn raft_apply_committed_entries_applies_member_upsert_once_by_command_id() {
        let repository = test_raft_repository().await;
        let mut first = raft::eraftpb::Entry::new();
        first.set_entry_type(EntryType::EntryNormal);
        first.index = 10;
        first.term = 4;
        first.data = br#"{"command_id":"cmd-member-1","command_type":"raft_member_upsert","payload":{"node_id":"tikee-2","endpoint":"http://tikee-2.tikee-headless:9998","status":"active"}}"#
            .to_vec()
            .into();
        let mut replay = raft::eraftpb::Entry::new();
        replay.set_entry_type(EntryType::EntryNormal);
        replay.index = 11;
        replay.term = 4;
        replay.data = br#"{"command_id":"cmd-member-1","command_type":"raft_member_upsert","payload":{"node_id":"tikee-2","endpoint":"http://tikee-2.example:9998","status":"removed"}}"#
            .to_vec()
            .into();

        let applied = apply_committed_entries("tikee-0", &repository, None, &[first, replay])
            .await
            .unwrap_or_else(|error| panic!("member upsert command should apply: {error}"));
        let commands = repository
            .list_applied_commands("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("applied commands should list: {error}"));
        let members = repository
            .list_members()
            .await
            .unwrap_or_else(|error| panic!("members should list: {error}"));
        let metadata = repository
            .get_metadata("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("metadata should load: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));

        assert_eq!(applied, Some(11));
        assert_eq!(metadata.applied_index, 11);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command_id, "cmd-member-1");
        assert_eq!(commands[0].command_type, "raft_member_upsert");
        assert_eq!(commands[0].status, "applied");
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].node_id, "tikee-2");
        assert_eq!(members[0].endpoint, "http://tikee-2.tikee-headless:9998");
        assert_eq!(members[0].status, "active");
    }

    #[tokio::test]
    async fn raft_apply_committed_entries_records_unsupported_and_rejected_payloads() {
        let repository = test_raft_repository().await;
        let mut unsupported = raft::eraftpb::Entry::new();
        unsupported.set_entry_type(EntryType::EntryNormal);
        unsupported.index = 12;
        unsupported.term = 4;
        unsupported.data =
            br#"{"command_id":"cmd-unknown-1","command_type":"future_command","payload":{"x":1}}"#
                .to_vec()
                .into();
        let mut rejected = raft::eraftpb::Entry::new();
        rejected.set_entry_type(EntryType::EntryNormal);
        rejected.index = 13;
        rejected.term = 4;
        rejected.data = br#"{"command_id":"cmd-member-bad","command_type":"raft_member_upsert","payload":{"node_id":"tikee-3","endpoint":"ftp://tikee-3","status":"active"}}"#
            .to_vec()
            .into();
        let mut invalid_json = raft::eraftpb::Entry::new();
        invalid_json.set_entry_type(EntryType::EntryNormal);
        invalid_json.index = 14;
        invalid_json.term = 4;
        invalid_json.data = b"not-json".to_vec().into();

        let applied = apply_committed_entries(
            "tikee-0",
            &repository,
            None,
            &[unsupported, rejected, invalid_json],
        )
        .await
        .unwrap_or_else(|error| panic!("non-applied command records should be stored: {error}"));
        let commands = repository
            .list_applied_commands("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("applied commands should list: {error}"));
        let members = repository
            .list_members()
            .await
            .unwrap_or_else(|error| panic!("members should list: {error}"));

        assert_eq!(applied, Some(14));
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].command_id, "cmd-unknown-1");
        assert_eq!(commands[0].status, "deferred_unsupported");
        assert_eq!(commands[1].command_id, "cmd-member-bad");
        assert_eq!(commands[1].status, "rejected");
        assert!(commands[1].message.contains("http or https"));
        assert_eq!(commands[2].command_id, "raft-invalid-14");
        assert_eq!(commands[2].command_type, "invalid_json");
        assert_eq!(commands[2].status, "rejected");
        assert!(members.is_empty());
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
            "tikee-0",
            &repository,
            None,
            &[normal, conf_change, after_conf_change],
        )
        .await
        .unwrap_or_else(|error| panic!("committed entries should gate config changes: {error}"));
        let metadata = repository
            .get_metadata("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("metadata should load: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));

        assert_eq!(applied, Some(4));
        assert_eq!(metadata.applied_index, 4);
    }

    #[tokio::test]
    async fn raft_apply_committed_conf_change_adds_member_after_commit() {
        let repository = test_raft_repository().await;
        let mut node = test_raw_node("tikee-0", &["tikee-0"]);
        let proposal = RaftMembershipProposal {
            proposal_id: "prop-add-3".to_owned(),
            action: "add_voter".to_owned(),
            node_id: "tikee-3".to_owned(),
            endpoint: Some("http://tikee-3.tikee-headless:9998".to_owned()),
        };
        repository
            .record_membership_proposal(tikee_storage::RecordRaftMembershipProposal {
                cluster_id: CLUSTER_ID.to_owned(),
                proposal_id: proposal.proposal_id.clone(),
                action: proposal.action.clone(),
                node_id: proposal.node_id.clone(),
                endpoint: proposal.endpoint.clone(),
                status: "proposed_conf_change".to_owned(),
                message: "test proposal".to_owned(),
                created_by: "test".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("proposal should record: {error}"));
        let conf_change = build_membership_conf_change(&proposal)
            .unwrap_or_else(|error| panic!("conf change should build: {error}"));
        let mut entry = raft::eraftpb::Entry::new();
        entry.set_entry_type(EntryType::EntryConfChange);
        entry.index = 20;
        entry.term = 5;
        entry.data = conf_change
            .write_to_bytes()
            .unwrap_or_else(|error| panic!("conf change should encode: {error}"))
            .into();

        let applied = apply_committed_entries("tikee-0", &repository, Some(&mut node), &[entry])
            .await
            .unwrap_or_else(|error| panic!("conf change should apply: {error}"));
        let metadata = repository
            .get_metadata("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("metadata should load: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));
        let member = repository
            .get_member("tikee-3")
            .await
            .unwrap_or_else(|error| panic!("member should load: {error}"))
            .unwrap_or_else(|| panic!("member should exist"));
        let stored = repository
            .get_membership_proposal(CLUSTER_ID, "prop-add-3")
            .await
            .unwrap_or_else(|error| panic!("proposal should load: {error}"))
            .unwrap_or_else(|| panic!("proposal should exist"));

        assert_eq!(applied, Some(20));
        assert_eq!(metadata.applied_index, 20);
        assert!(metadata.conf_state.is_some());
        assert_eq!(member.status, "active");
        assert_eq!(stored.status, "applied");
    }

    #[tokio::test]
    async fn raft_apply_committed_conf_change_rejects_malformed_payload_but_advances() {
        let repository = test_raft_repository().await;
        let mut node = test_raw_node("tikee-0", &["tikee-0"]);
        let mut entry = raft::eraftpb::Entry::new();
        entry.set_entry_type(EntryType::EntryConfChange);
        entry.index = 21;
        entry.term = 5;
        entry.data = b"bad-conf-change".to_vec().into();

        let applied = apply_committed_entries("tikee-0", &repository, Some(&mut node), &[entry])
            .await
            .unwrap_or_else(|error| {
                panic!("malformed conf change should be recorded as handled: {error}")
            });
        let metadata = repository
            .get_metadata("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("metadata should load: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));

        assert_eq!(applied, Some(21));
        assert_eq!(metadata.applied_index, 21);
        assert_eq!(metadata.conf_state, None);
    }

    #[tokio::test]
    async fn raft_apply_committed_conf_change_without_runtime_remains_gated() {
        let repository = test_raft_repository().await;
        let mut conf_change = raft::eraftpb::ConfChange::new();
        conf_change.set_change_type(raft::eraftpb::ConfChangeType::AddNode);
        conf_change.node_id = raft_numeric_id("tikee-4");
        conf_change.context = serde_json::to_vec(&RaftMembershipProposalContext {
            proposal_id: "prop-add-4".to_owned(),
            action: "add_voter".to_owned(),
            node_id: "tikee-4".to_owned(),
            endpoint: Some("http://tikee-4.tikee-headless:9998".to_owned()),
        })
        .unwrap_or_else(|error| panic!("context should encode: {error}"))
        .into();
        let mut entry = raft::eraftpb::Entry::new();
        entry.set_entry_type(EntryType::EntryConfChange);
        entry.index = 22;
        entry.term = 5;
        entry.data = conf_change
            .write_to_bytes()
            .unwrap_or_else(|error| panic!("conf change should encode: {error}"))
            .into();

        let applied = apply_committed_entries("tikee-0", &repository, None, &[entry])
            .await
            .unwrap_or_else(|error| panic!("conf change without runtime should gate: {error}"));
        let metadata = repository
            .get_metadata("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("metadata should load: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));

        assert_eq!(applied, None);
        assert_eq!(metadata.applied_index, 0);
    }

    #[test]
    fn leader_fencing_token_requires_real_leader_role_and_term() {
        assert_eq!(
            leader_fencing_token_for_role(ClusterRole::Leader, "tikee-0", 7).as_deref(),
            Some("raft:term:7:node:tikee-0")
        );
        assert_eq!(
            leader_fencing_token_for_role(ClusterRole::Leader, "tikee-0", 0),
            None
        );
        assert_eq!(
            leader_fencing_token_for_role(ClusterRole::Follower, "tikee-0", 7),
            None
        );
    }

    #[tokio::test]
    async fn raft_runtime_starts_ticker_without_granting_tikee_ownership() {
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
        message.from = raft_numeric_id("tikee-1");
        message.to = raft_numeric_id("tikee-0");
        message.term = 1;
        let submission = coordinator.submit_raft_message(message).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        let status = coordinator.status().await;

        assert!(submission.accepted);
        assert!(submission.reason.contains("enqueued"));
        assert!(!status.can_schedule);
        assert_eq!(status.leader_fencing_token, None);
    }

    #[tokio::test]
    async fn raft_inprocess_harness_elects_real_leader_and_persists_fencing() {
        let mut cluster = TestRaftCluster::new(&["tikee-0", "tikee-1", "tikee-2"]).await;
        cluster
            .nodes
            .get_mut("tikee-0")
            .unwrap_or_else(|| panic!("tikee-0 should exist"))
            .raw
            .campaign()
            .unwrap_or_else(|error| panic!("campaign should start: {error}"));
        cluster.drain().await;

        let leaders = cluster.leader_ids();
        let status = cluster.nodes["tikee-0"].status.read().await.clone();
        let metadata = cluster.nodes["tikee-0"]
            .repository
            .get_metadata("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("metadata should load: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));

        assert_eq!(leaders, vec!["tikee-0".to_owned()]);
        assert_eq!(status.role, ClusterRole::Leader);
        assert!(status.can_schedule);
        assert_eq!(
            metadata.leader_fencing_token.as_deref(),
            Some("raft:term:1:node:tikee-0")
        );
    }

    #[tokio::test]
    async fn raft_inprocess_membership_proposal_commits_and_applies_member() {
        let mut cluster = TestRaftCluster::new(&["tikee-0", "tikee-1", "tikee-2"]).await;
        cluster
            .nodes
            .get_mut("tikee-0")
            .unwrap_or_else(|| panic!("tikee-0 should exist"))
            .raw
            .campaign()
            .unwrap_or_else(|error| panic!("campaign should start: {error}"));
        cluster.drain().await;
        let proposal = RaftMembershipProposal {
            proposal_id: "prop-add-4-e2e".to_owned(),
            action: "add_voter".to_owned(),
            node_id: "tikee-4".to_owned(),
            endpoint: Some("http://tikee-4.tikee-headless:9998".to_owned()),
        };
        cluster.record_proposal("tikee-0", &proposal).await;
        let conf_change = build_membership_conf_change(&proposal)
            .unwrap_or_else(|error| panic!("conf change should build: {error}"));
        cluster
            .nodes
            .get_mut("tikee-0")
            .unwrap_or_else(|| panic!("tikee-0 should exist"))
            .raw
            .propose_conf_change(conf_change.get_context().to_vec(), conf_change)
            .unwrap_or_else(|error| panic!("conf change should propose: {error}"));
        cluster.drain().await;

        let leader = &cluster.nodes["tikee-0"];
        let member = leader
            .repository
            .get_member("tikee-4")
            .await
            .unwrap_or_else(|error| panic!("member should load: {error}"))
            .unwrap_or_else(|| panic!("member should exist"));
        let proposal = leader
            .repository
            .get_membership_proposal(CLUSTER_ID, "prop-add-4-e2e")
            .await
            .unwrap_or_else(|error| panic!("proposal should load: {error}"))
            .unwrap_or_else(|| panic!("proposal should exist"));
        let metadata = leader
            .repository
            .get_metadata("tikee-0")
            .await
            .unwrap_or_else(|error| panic!("metadata should load: {error}"))
            .unwrap_or_else(|| panic!("metadata should exist"));

        assert_eq!(member.status, "active");
        assert_eq!(proposal.status, "applied");
        assert!(metadata.conf_state.is_some());
    }

    fn test_raft_config() -> ClusterConfig {
        ClusterConfig {
            mode: ClusterModeConfig::Raft,
            node_id: "tikee-0".to_owned(),
            peers: vec![
                ClusterPeerConfig {
                    node_id: "tikee-0".to_owned(),
                    endpoint: "http://tikee-0.tikee-headless:9999".to_owned(),
                },
                ClusterPeerConfig {
                    node_id: "tikee-1".to_owned(),
                    endpoint: "http://tikee-1.tikee-headless:9999".to_owned(),
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
                node_id: "tikee-0".to_owned(),
                current_term: 1,
                voted_for: None,
                commit_index: 0,
                applied_index: 0,
                leader_fencing_token: None,
                conf_state: None,
            })
            .await
            .unwrap_or_else(|error| panic!("metadata should upsert: {error}"));
        repository
    }

    fn test_raw_node(
        local: &str,
        voters: &[&str],
    ) -> raft::raw_node::RawNode<raft::storage::MemStorage> {
        let local_id = raft_numeric_id(local);
        let voter_ids = voters.iter().map(|node_id| raft_numeric_id(node_id));
        let mut config = raft::Config::new(local_id);
        config.heartbeat_tick = 2;
        config.election_tick = 20;
        config
            .validate()
            .unwrap_or_else(|error| panic!("test raft config should validate: {error}"));
        let storage = raft::storage::MemStorage::new_with_conf_state((voter_ids, Vec::new()));
        raft::raw_node::RawNode::with_default_logger(&config, storage)
            .unwrap_or_else(|error| panic!("test raw node should build: {error}"))
    }

    struct TestRaftNode {
        raw: raft::raw_node::RawNode<raft::storage::MemStorage>,
        repository: RaftRepository,
        status: Arc<RwLock<ClusterStatus>>,
    }

    struct TestRaftCluster {
        nodes: BTreeMap<String, TestRaftNode>,
    }

    impl TestRaftCluster {
        async fn new(node_ids: &[&str]) -> Self {
            let mut nodes = BTreeMap::new();
            let voter_ids = node_ids
                .iter()
                .map(|node_id| raft_numeric_id(node_id))
                .collect::<Vec<_>>();
            for node_id in node_ids {
                nodes.insert(
                    (*node_id).to_owned(),
                    TestRaftNode {
                        raw: test_raw_node_from_ids(node_id, voter_ids.clone()),
                        repository: test_raft_repository_for(node_id).await,
                        status: Arc::new(RwLock::new(test_cluster_status(node_id, node_ids.len()))),
                    },
                );
            }
            Self { nodes }
        }

        async fn drain(&mut self) {
            for _ in 0..32 {
                let mut messages = Vec::new();
                for (node_id, node) in &mut self.nodes {
                    messages.extend(process_test_ready(node_id, node).await);
                }
                if messages.is_empty() {
                    continue;
                }
                for message in messages {
                    if let Some(target) = self
                        .nodes
                        .values_mut()
                        .find(|node| node.raw.raft.id == message.to)
                    {
                        target
                            .raw
                            .step(message)
                            .unwrap_or_else(|error| panic!("message should step: {error}"));
                    }
                }
            }
        }

        fn leader_ids(&self) -> Vec<String> {
            self.nodes
                .iter()
                .filter(|(_, node)| node.raw.raft.state == StateRole::Leader)
                .map(|(node_id, _)| node_id.clone())
                .collect()
        }

        async fn record_proposal(&self, node_id: &str, proposal: &RaftMembershipProposal) {
            self.nodes[node_id]
                .repository
                .record_membership_proposal(tikee_storage::RecordRaftMembershipProposal {
                    cluster_id: CLUSTER_ID.to_owned(),
                    proposal_id: proposal.proposal_id.clone(),
                    action: proposal.action.clone(),
                    node_id: proposal.node_id.clone(),
                    endpoint: proposal.endpoint.clone(),
                    status: "proposed_conf_change".to_owned(),
                    message: "test e2e proposal".to_owned(),
                    created_by: "test".to_owned(),
                })
                .await
                .unwrap_or_else(|error| panic!("proposal should record: {error}"));
        }
    }

    async fn process_test_ready(
        node_id: &str,
        node: &mut TestRaftNode,
    ) -> Vec<raft::eraftpb::Message> {
        if !node.raw.has_ready() {
            update_runtime_status(node_id, &node.repository, &node.raw, &node.status)
                .await
                .unwrap_or_else(|error| {
                    panic!("test raft storage/status operation should succeed: {error}")
                });
            return Vec::new();
        }
        let ready = node.raw.ready();
        if let Some(hard_state) = ready.hs() {
            persist_hard_state(node_id, &node.repository, hard_state)
                .await
                .unwrap_or_else(|error| {
                    panic!("test raft storage/status operation should succeed: {error}")
                });
            node.raw
                .raft
                .mut_store()
                .wl()
                .set_hardstate(hard_state.clone());
        }
        for entry in ready.entries() {
            persist_entry(node_id, &node.repository, entry)
                .await
                .unwrap_or_else(|error| {
                    panic!("test raft storage/status operation should succeed: {error}")
                });
        }
        node.raw
            .raft
            .mut_store()
            .wl()
            .append(ready.entries())
            .unwrap_or_else(|error| panic!("ready entries should append: {error}"));
        let mut messages = ready.messages().to_vec();
        messages.extend(ready.persisted_messages().iter().cloned());
        let applied = apply_committed_entries(
            node_id,
            &node.repository,
            Some(&mut node.raw),
            ready.committed_entries(),
        )
        .await
        .unwrap_or_else(|error| panic!("test committed entries should apply: {error}"));
        let light = node.raw.advance_append(ready);
        if let Some(commit) = light.commit_index() {
            node.raw
                .raft
                .mut_store()
                .wl()
                .mut_hard_state()
                .set_commit(commit);
        }
        if let Some(applied) = applied {
            node.raw.advance_apply_to(applied);
        }
        messages.extend(light.messages().iter().cloned());
        let light_applied = apply_committed_entries(
            node_id,
            &node.repository,
            Some(&mut node.raw),
            light.committed_entries(),
        )
        .await
        .unwrap_or_else(|error| panic!("test committed entries should apply: {error}"));
        if let Some(applied) = light_applied {
            node.raw.advance_apply_to(applied);
        }
        update_runtime_status(node_id, &node.repository, &node.raw, &node.status)
            .await
            .unwrap_or_else(|error| panic!("test raft status update should succeed: {error}"));
        messages
    }

    async fn test_raft_repository_for(node_id: &str) -> RaftRepository {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should initialize: {error}"));
        let repository = RaftRepository::new(db);
        repository
            .upsert_metadata(UpsertRaftMetadata {
                cluster_id: CLUSTER_ID.to_owned(),
                node_id: node_id.to_owned(),
                current_term: 1,
                voted_for: None,
                commit_index: 0,
                applied_index: 0,
                leader_fencing_token: None,
                conf_state: None,
            })
            .await
            .unwrap_or_else(|error| panic!("metadata should upsert: {error}"));
        repository
    }

    fn test_cluster_status(node_id: &str, nodes: usize) -> ClusterStatus {
        ClusterStatus {
            mode: ClusterMode::Raft,
            role: ClusterRole::Follower,
            node_id: node_id.to_owned(),
            nodes: u32::try_from(nodes).unwrap_or(u32::MAX),
            can_schedule: false,
            leader_fencing_token: None,
            detail: "test raft node".to_owned(),
        }
    }

    fn test_raw_node_from_ids(
        local: &str,
        voters: Vec<u64>,
    ) -> raft::raw_node::RawNode<raft::storage::MemStorage> {
        let local_id = raft_numeric_id(local);
        let mut config = raft::Config::new(local_id);
        config.heartbeat_tick = 2;
        config.election_tick = 10;
        config.pre_vote = false;
        config
            .validate()
            .unwrap_or_else(|error| panic!("test raft config should validate: {error}"));
        let storage = raft::storage::MemStorage::new_with_conf_state((voters, Vec::new()));
        raft::raw_node::RawNode::with_default_logger(&config, storage)
            .unwrap_or_else(|error| panic!("test raw node should build: {error}"))
    }
}
