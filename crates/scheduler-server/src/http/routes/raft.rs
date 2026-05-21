use std::sync::Arc;

use axum::{Json, extract::State, http::HeaderMap};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use raft::eraftpb::{Entry, EntryType, Message, MessageType};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, RaftAppendEntriesApiResponse, RaftAppendEntriesRequest, RaftMessageResult,
        RaftWireEntry,
    },
    error::ApiError,
};

use super::common::client_ip;

/// Receive a Raft `AppendEntries` transport message placeholder.
///
/// This endpoint is intentionally non-mutating until a real consensus runtime is wired.
/// It documents the LB/K8s-safe HTTP transport shape without granting leadership.
///
/// # Errors
///
/// Returns authorization errors when the caller lacks cluster read permission.
#[utoipa::path(
    post,
    path = "/api/v1/raft/append-entries",
    tag = "raft",
    request_body = RaftAppendEntriesRequest,
    responses(
        (status = 200, description = "Raft message accepted as not implemented", body = RaftAppendEntriesApiResponse),
        (status = 400, description = "Invalid raft-rs message", body = crate::http::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::http::dto::ErrorResponse)
    )
)]
pub async fn append_entries(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<RaftAppendEntriesRequest>,
) -> Result<Json<RaftAppendEntriesApiResponse>, ApiError> {
    auth::require_permission(&headers, &state, "cluster", "read").await?;
    let message = request_to_raft_message(&request)?;
    let local = state.cluster.status().await;
    Ok(Json(ApiResponse::success(RaftMessageResult {
        accepted: false,
        reason: format!(
            "raft-rs {message_type:?} transport validated but event loop is not started",
            message_type = message.get_msg_type()
        ),
        local_node_id: local.node_id,
        local_role: local.role.as_str().to_owned(),
        leader_fencing_token: None,
        remote_addr: client_ip(&headers),
        received_term: request.term,
    })))
}

fn request_to_raft_message(request: &RaftAppendEntriesRequest) -> Result<Message, ApiError> {
    let mut message = Message::new();
    message.set_msg_type(parse_message_type(&request.message_type)?);
    message.from = request.from;
    message.to = request.to;
    message.term = non_negative_i64_to_u64(request.term, "term")?;
    message.index = non_negative_i64_to_u64(request.index, "index")?;
    message.log_term = non_negative_i64_to_u64(request.log_term, "log_term")?;
    message.commit = non_negative_i64_to_u64(request.commit, "commit")?;
    message.reject = request.reject.unwrap_or(false);
    message.reject_hint =
        non_negative_i64_to_u64(request.reject_hint.unwrap_or_default(), "reject_hint")?;
    if let Some(context) = &request.context {
        message.context = decode_base64(context, "context")?.into();
    }
    let entries = request
        .entries
        .iter()
        .map(wire_entry_to_raft_entry)
        .collect::<Result<Vec<_>, _>>()?;
    message.set_entries(entries.into());
    Ok(message)
}

fn wire_entry_to_raft_entry(entry: &RaftWireEntry) -> Result<Entry, ApiError> {
    let mut raft_entry = Entry::new();
    raft_entry.set_entry_type(parse_entry_type(&entry.entry_type)?);
    raft_entry.index = non_negative_i64_to_u64(entry.index, "entry.index")?;
    raft_entry.term = non_negative_i64_to_u64(entry.term, "entry.term")?;
    raft_entry.data = decode_base64(&entry.data, "entry.data")?.into();
    if let Some(context) = &entry.context {
        raft_entry.context = decode_base64(context, "entry.context")?.into();
    }
    Ok(raft_entry)
}

fn parse_message_type(value: &str) -> Result<MessageType, ApiError> {
    match value {
        "MsgHup" => Ok(MessageType::MsgHup),
        "MsgBeat" => Ok(MessageType::MsgBeat),
        "MsgPropose" => Ok(MessageType::MsgPropose),
        "MsgAppend" => Ok(MessageType::MsgAppend),
        "MsgAppendResponse" => Ok(MessageType::MsgAppendResponse),
        "MsgRequestVote" => Ok(MessageType::MsgRequestVote),
        "MsgRequestVoteResponse" => Ok(MessageType::MsgRequestVoteResponse),
        "MsgSnapshot" => Ok(MessageType::MsgSnapshot),
        "MsgHeartbeat" => Ok(MessageType::MsgHeartbeat),
        "MsgHeartbeatResponse" => Ok(MessageType::MsgHeartbeatResponse),
        "MsgUnreachable" => Ok(MessageType::MsgUnreachable),
        "MsgSnapStatus" => Ok(MessageType::MsgSnapStatus),
        "MsgCheckQuorum" => Ok(MessageType::MsgCheckQuorum),
        "MsgTransferLeader" => Ok(MessageType::MsgTransferLeader),
        "MsgTimeoutNow" => Ok(MessageType::MsgTimeoutNow),
        "MsgReadIndex" => Ok(MessageType::MsgReadIndex),
        "MsgReadIndexResp" => Ok(MessageType::MsgReadIndexResp),
        "MsgRequestPreVote" => Ok(MessageType::MsgRequestPreVote),
        "MsgRequestPreVoteResponse" => Ok(MessageType::MsgRequestPreVoteResponse),
        _ => Err(ApiError::bad_request(format!(
            "unsupported raft-rs message_type: {value}"
        ))),
    }
}

fn parse_entry_type(value: &str) -> Result<EntryType, ApiError> {
    match value {
        "EntryNormal" => Ok(EntryType::EntryNormal),
        "EntryConfChange" => Ok(EntryType::EntryConfChange),
        "EntryConfChangeV2" => Ok(EntryType::EntryConfChangeV2),
        _ => Err(ApiError::bad_request(format!(
            "unsupported raft-rs entry_type: {value}"
        ))),
    }
}

fn non_negative_i64_to_u64(value: i64, field: &str) -> Result<u64, ApiError> {
    u64::try_from(value).map_err(|_| ApiError::bad_request(format!("{field} cannot be negative")))
}

fn decode_base64(value: &str, field: &str) -> Result<Vec<u8>, ApiError> {
    STANDARD
        .decode(value)
        .map_err(|_| ApiError::bad_request(format!("{field} must be base64 encoded")))
}

#[cfg(test)]
mod tests {
    use super::{MessageType, request_to_raft_message};
    use crate::http::dto::{RaftAppendEntriesRequest, RaftWireEntry};

    #[test]
    fn request_to_raft_message_decodes_entries() {
        let message = request_to_raft_message(&RaftAppendEntriesRequest {
            from: 1,
            to: 2,
            term: 3,
            message_type: "MsgAppend".to_owned(),
            index: 4,
            log_term: 2,
            commit: 4,
            snapshot_index: None,
            snapshot_term: None,
            entries: vec![RaftWireEntry {
                entry_type: "EntryNormal".to_owned(),
                index: 5,
                term: 3,
                data: "cGl4ZWw=".to_owned(),
                context: Some("Y3R4".to_owned()),
            }],
            context: Some("bXNn".to_owned()),
            reject: Some(false),
            reject_hint: None,
            leader_fencing_token: None,
        })
        .unwrap_or_else(|error| panic!("wire request should convert: {error:?}"));

        assert_eq!(message.get_msg_type(), MessageType::MsgAppend);
        assert_eq!(message.from, 1);
        assert_eq!(message.to, 2);
        assert_eq!(message.entries[0].data.as_ref(), b"pixel");
        assert_eq!(message.entries[0].context.as_ref(), b"ctx");
        assert_eq!(message.context.as_ref(), b"msg");
    }

    #[test]
    fn request_to_raft_message_rejects_negative_terms() {
        let result = request_to_raft_message(&RaftAppendEntriesRequest {
            from: 1,
            to: 2,
            term: -1,
            message_type: "MsgAppend".to_owned(),
            index: 0,
            log_term: 0,
            commit: 0,
            snapshot_index: None,
            snapshot_term: None,
            entries: Vec::new(),
            context: None,
            reject: None,
            reject_hint: None,
            leader_fencing_token: None,
        });
        let Err(error) = result else {
            panic!("negative term should be rejected");
        };

        assert!(format!("{error:?}").contains("term cannot be negative"));
    }
}
