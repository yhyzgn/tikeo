use base64::{Engine as _, engine::general_purpose::STANDARD};
use raft::eraftpb::{Entry, Message};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(super) struct RaftWireMessage {
    pub(super) from: u64,
    pub(super) to: u64,
    pub(super) term: i64,
    pub(super) message_type: String,
    pub(super) index: i64,
    pub(super) log_term: i64,
    pub(super) commit: i64,
    pub(super) snapshot_index: Option<i64>,
    pub(super) snapshot_term: Option<i64>,
    pub(super) entries: Vec<RaftWireLogEntry>,
    pub(super) context: Option<String>,
    pub(super) reject: bool,
    pub(super) reject_hint: Option<i64>,
    pub(super) leader_fencing_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct RaftWireLogEntry {
    pub(super) entry_type: String,
    pub(super) index: i64,
    pub(super) term: i64,
    pub(super) data: String,
    pub(super) context: Option<String>,
}

pub(super) fn raft_append_entries_url(endpoint: &str) -> String {
    const PATH: &str = "/api/v1/raft/append-entries";
    if endpoint.ends_with(PATH) {
        endpoint.to_owned()
    } else {
        format!("{}{}", endpoint.trim_end_matches('/'), PATH)
    }
}

pub(super) fn raft_message_to_wire_request(message: &Message) -> RaftWireMessage {
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
