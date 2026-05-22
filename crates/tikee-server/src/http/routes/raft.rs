use std::sync::Arc;

use axum::{Json, extract::State, http::HeaderMap};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use raft::eraftpb::{Entry, EntryType, Message, MessageType};
use tikee_storage::RecordRaftMembershipProposal;
use url::Url;

use crate::cluster::RaftMembershipProposal;
use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, RaftAppendEntriesApiResponse, RaftAppendEntriesRequest,
        RaftMembershipProposalApiResponse, RaftMembershipProposalRequest,
        RaftMembershipProposalResponse, RaftMessageResult, RaftWireEntry,
    },
    error::ApiError,
};

use super::common::client_ip;

/// Receive a Raft `AppendEntries` transport message.
///
/// This endpoint validates the LB/K8s-safe HTTP transport shape and enqueues messages
/// into a running raft-rs runtime inbox when available. Enqueueing does not grant
/// tikee leadership; scheduling remains fenced until a persisted leader token exists.
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
        (status = 200, description = "Raft message validation/submission result", body = RaftAppendEntriesApiResponse),
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
    if !is_internal_raft_transport(&headers, &state) {
        auth::require_permission(&headers, &state, "cluster", "read").await?;
    }
    let message = request_to_raft_message(&request)?;
    let submission = state.cluster.submit_raft_message(message).await;
    let local = state.cluster.status().await;
    Ok(Json(ApiResponse::success(RaftMessageResult {
        accepted: submission.accepted,
        reason: submission.reason,
        local_node_id: local.node_id,
        local_role: local.role.as_str().to_owned(),
        leader_fencing_token: local.leader_fencing_token,
        remote_addr: client_ip(&headers),
        received_term: request.term,
    })))
}

/// Create a gated Raft membership proposal intent.
///
/// This endpoint validates operator intent and leader fencing before storing the proposal.
/// It intentionally does not apply raft-rs `ConfChange` yet; committed config-change handling
/// remains gated until `ConfState` persistence and quorum-safe transitions are implemented.
///
/// # Errors
///
/// Returns auth, validation, or storage errors as the standard API envelope.
#[utoipa::path(
    post,
    path = "/api/v1/raft/members:propose",
    tag = "raft",
    request_body = RaftMembershipProposalRequest,
    responses(
        (status = 200, description = "Membership proposal intent stored", body = RaftMembershipProposalApiResponse),
        (status = 400, description = "Invalid proposal", body = crate::http::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::http::dto::ErrorResponse),
        (status = 403, description = "Forbidden or not leader", body = crate::http::dto::ErrorResponse)
    )
)]
pub async fn propose_member_change(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<RaftMembershipProposalRequest>,
) -> Result<Json<RaftMembershipProposalApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "cluster", "manage").await?;
    let status = state.cluster.status().await;
    validate_membership_proposal_leader(&status)?;
    let proposal = validate_membership_proposal_request(&request, &status, &state).await?;
    let stored = state
        .raft
        .record_membership_proposal(RecordRaftMembershipProposal {
            cluster_id: "default".to_owned(),
            proposal_id: proposal.proposal_id.clone(),
            action: proposal.action.clone(),
            node_id: proposal.node_id.clone(),
            endpoint: proposal.endpoint.clone(),
            status: "pending_propose".to_owned(),
            message: "membership proposal intent stored; awaiting raft-rs propose_conf_change"
                .to_owned(),
            created_by: principal.username,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    let submission = state
        .cluster
        .propose_membership_change(RaftMembershipProposal {
            proposal_id: proposal.proposal_id,
            action: proposal.action,
            node_id: proposal.node_id,
            endpoint: proposal.endpoint,
        })
        .await;
    let stored = if submission.accepted {
        state
            .raft
            .update_membership_proposal_status(
                "default",
                &stored.proposal_id,
                "proposed_conf_change",
                &submission.reason,
            )
            .await
            .map_err(|error| ApiError::storage(&error))?
            .unwrap_or(stored)
    } else {
        state
            .raft
            .update_membership_proposal_status(
                "default",
                &stored.proposal_id,
                "rejected",
                &submission.reason,
            )
            .await
            .map_err(|error| ApiError::storage(&error))?
            .unwrap_or(stored)
    };
    let reason = submission.reason;

    Ok(Json(ApiResponse::success(RaftMembershipProposalResponse {
        accepted: submission.accepted,
        reason,
        local_node_id: status.node_id,
        local_role: status.role.as_str().to_owned(),
        leader_fencing_token: status.leader_fencing_token,
        proposal: Some(stored),
    })))
}

#[derive(Debug, Clone)]
struct ValidatedMembershipProposal {
    proposal_id: String,
    action: String,
    node_id: String,
    endpoint: Option<String>,
}

fn validate_membership_proposal_leader(
    status: &crate::cluster::ClusterStatus,
) -> Result<(), ApiError> {
    if status.mode != crate::cluster::ClusterMode::Raft
        || status.role != crate::cluster::ClusterRole::Leader
        || !status.can_schedule
        || status
            .leader_fencing_token
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return Err(ApiError::forbidden(
            "membership proposals require a real raft leader with a persisted fencing token",
        ));
    }
    Ok(())
}

async fn validate_membership_proposal_request(
    request: &RaftMembershipProposalRequest,
    status: &crate::cluster::ClusterStatus,
    state: &AppState,
) -> Result<ValidatedMembershipProposal, ApiError> {
    let proposal_id = require_non_empty(&request.proposal_id, "proposal_id")?;
    let action = require_non_empty(&request.action, "action")?;
    let node_id = require_non_empty(&request.node_id, "node_id")?;
    match action.as_str() {
        "add_voter" => {
            let endpoint = request
                .endpoint
                .as_ref()
                .ok_or_else(|| ApiError::bad_request("endpoint is required for add_voter"))?;
            validate_member_endpoint(endpoint)?;
            Ok(ValidatedMembershipProposal {
                proposal_id,
                action,
                node_id,
                endpoint: Some(endpoint.trim().to_owned()),
            })
        }
        "remove_voter" => {
            if node_id == status.node_id {
                return Err(ApiError::bad_request(
                    "remove_voter cannot target the current leader node",
                ));
            }
            let members = state
                .raft
                .list_members()
                .await
                .map_err(|error| ApiError::storage(&error))?;
            let active_count = members
                .iter()
                .filter(|member| {
                    matches!(member.status.as_str(), "configured" | "joining" | "active")
                })
                .count();
            if active_count <= 2 {
                return Err(ApiError::bad_request(
                    "remove_voter refuses quorum reduction until joint-consensus safety is implemented",
                ));
            }
            Ok(ValidatedMembershipProposal {
                proposal_id,
                action,
                node_id,
                endpoint: None,
            })
        }
        _ => Err(ApiError::bad_request(
            "action must be one of add_voter, remove_voter",
        )),
    }
}

fn require_non_empty(value: &str, field: &str) -> Result<String, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(ApiError::bad_request(format!("{field} must not be empty")))
    } else {
        Ok(trimmed.to_owned())
    }
}

fn validate_member_endpoint(endpoint: &str) -> Result<(), ApiError> {
    let parsed = Url::parse(endpoint).map_err(|error| {
        ApiError::bad_request(format!("endpoint must be an absolute URL: {error}"))
    })?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ApiError::bad_request("endpoint must use http or https"));
    }
    if parsed.host_str().is_none() {
        return Err(ApiError::bad_request("endpoint must include a host"));
    }
    Ok(())
}

fn is_internal_raft_transport(headers: &HeaderMap, state: &AppState) -> bool {
    let Some(expected) = state.raft_transport_token.as_deref() else {
        return false;
    };
    headers
        .get("x-tikee-raft-token")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|actual| !actual.is_empty() && actual == expected)
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
