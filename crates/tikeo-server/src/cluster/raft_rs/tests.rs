use base64::Engine as _;
use protobuf::Message as _;
use raft::{GetEntriesContext, Storage, eraftpb::EntryType};
use tikeo_config::{ClusterConfig, ClusterModeConfig, ClusterPeerConfig};
use tikeo_storage::{
    RaftRepository, UpsertClusterShardOwnership, UpsertRaftLogEntry, UpsertRaftMember,
    UpsertRaftMetadata, connect_and_migrate,
};

use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;

use super::{
    CLUSTER_ID, RaftMembershipProposalContext, RaftRuntimeCoordinator, STANDARD, StateRole,
    apply_committed_entries, build_membership_conf_change, build_runtime_from_repository,
    leader_fencing_token_for_role, persist_entry, persist_hard_state,
    project_balanced_shard_ownership, raft_append_entries_url, raft_message_to_wire_request,
    raft_numeric_id, trigger_autonomous_campaign, update_runtime_status,
    validate_raft_rs_bootstrap,
};
use crate::cluster::{ClusterMode, ClusterRole, ClusterStatus, RaftMembershipProposal};

#[test]
fn raft_numeric_id_is_stable_non_zero() {
    let first = raft_numeric_id("tikeo-0");
    let second = raft_numeric_id("tikeo-0");

    assert_ne!(first, 0);
    assert_eq!(first, second);
}

#[test]
fn raft_rs_bootstrap_constructs_raw_node_without_leadership() {
    let config = test_raft_config();

    let status = validate_raft_rs_bootstrap(&config)
        .unwrap_or_else(|error| panic!("raft-rs bootstrap should validate: {error}"));

    assert_eq!(status.node_id, "tikeo-0");
    assert_eq!(status.voter_ids.len(), 2);
    assert_eq!(status.initial_role, "follower");
}

#[tokio::test]
async fn raft_runtime_restore_replays_persisted_metadata_and_clears_stale_fencing() {
    let repository = test_raft_repository_for("tikeo-0").await;
    repository
        .upsert_metadata(UpsertRaftMetadata {
            cluster_id: CLUSTER_ID.to_owned(),
            node_id: "tikeo-0".to_owned(),
            current_term: 7,
            voted_for: Some("42".to_owned()),
            commit_index: 2,
            applied_index: 1,
            leader_fencing_token: Some("raft:term:7:node:tikeo-0".to_owned()),
            conf_state: None,
        })
        .await
        .unwrap_or_else(|error| panic!("metadata should persist: {error}"));
    for index in 1..=2 {
        repository
            .upsert_log_entry(UpsertRaftLogEntry {
                cluster_id: CLUSTER_ID.to_owned(),
                node_id: "tikeo-0".to_owned(),
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
        .get_metadata("tikeo-0")
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
        raft_append_entries_url("http://tikeo-1:9998"),
        "http://tikeo-1:9998/api/v1/raft/append-entries"
    );
    assert_eq!(
        raft_append_entries_url("http://tikeo-1:9998/api/v1/raft/append-entries"),
        "http://tikeo-1:9998/api/v1/raft/append-entries"
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

    let applied = apply_committed_entries("tikeo-0", &repository, None, &[first, second])
        .await
        .unwrap_or_else(|error| panic!("committed entries should apply: {error}"));
    let metadata = repository
        .get_metadata("tikeo-0")
        .await
        .unwrap_or_else(|error| panic!("metadata should load: {error}"))
        .unwrap_or_else(|| panic!("metadata should exist"));

    assert_eq!(applied, Some(3));
    assert_eq!(metadata.applied_index, 3);
    assert_eq!(metadata.leader_fencing_token, None);
    let commands = repository
        .list_applied_commands("tikeo-0")
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

    let applied = apply_committed_entries("tikeo-0", &repository, None, &[entry])
        .await
        .unwrap_or_else(|error| panic!("noop command should apply: {error}"));
    let commands = repository
        .list_applied_commands("tikeo-0")
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
    first.data = br#"{"command_id":"cmd-member-1","command_type":"raft_member_upsert","payload":{"node_id":"tikeo-2","endpoint":"http://tikeo-2.tikeo-headless:9998","status":"active"}}"#
            .to_vec()
            .into();
    let mut replay = raft::eraftpb::Entry::new();
    replay.set_entry_type(EntryType::EntryNormal);
    replay.index = 11;
    replay.term = 4;
    replay.data = br#"{"command_id":"cmd-member-1","command_type":"raft_member_upsert","payload":{"node_id":"tikeo-2","endpoint":"http://tikeo-2.example:9998","status":"removed"}}"#
            .to_vec()
            .into();

    let applied = apply_committed_entries("tikeo-0", &repository, None, &[first, replay])
        .await
        .unwrap_or_else(|error| panic!("member upsert command should apply: {error}"));
    let commands = repository
        .list_applied_commands("tikeo-0")
        .await
        .unwrap_or_else(|error| panic!("applied commands should list: {error}"));
    let members = repository
        .list_members()
        .await
        .unwrap_or_else(|error| panic!("members should list: {error}"));
    let metadata = repository
        .get_metadata("tikeo-0")
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
    assert_eq!(members[0].node_id, "tikeo-2");
    assert_eq!(members[0].endpoint, "http://tikeo-2.tikeo-headless:9998");
    assert_eq!(members[0].status, "active");
}

#[tokio::test]
async fn raft_apply_committed_entries_rejects_unknown_and_invalid_payloads() {
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
    rejected.data = br#"{"command_id":"cmd-member-bad","command_type":"raft_member_upsert","payload":{"node_id":"tikeo-3","endpoint":"ftp://tikeo-3","status":"active"}}"#
            .to_vec()
            .into();
    let mut invalid_json = raft::eraftpb::Entry::new();
    invalid_json.set_entry_type(EntryType::EntryNormal);
    invalid_json.index = 14;
    invalid_json.term = 4;
    invalid_json.data = b"not-json".to_vec().into();

    let applied = apply_committed_entries(
        "tikeo-0",
        &repository,
        None,
        &[unsupported, rejected, invalid_json],
    )
    .await
    .unwrap_or_else(|error| panic!("non-applied command records should be stored: {error}"));
    let commands = repository
        .list_applied_commands("tikeo-0")
        .await
        .unwrap_or_else(|error| panic!("applied commands should list: {error}"));
    let members = repository
        .list_members()
        .await
        .unwrap_or_else(|error| panic!("members should list: {error}"));

    assert_eq!(applied, Some(14));
    assert_eq!(commands.len(), 3);
    assert_eq!(commands[0].command_id, "cmd-unknown-1");
    assert_eq!(commands[0].status, "rejected");
    assert!(
        commands[0]
            .message
            .contains("unsupported raft command_type")
    );
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
        "tikeo-0",
        &repository,
        None,
        &[normal, conf_change, after_conf_change],
    )
    .await
    .unwrap_or_else(|error| panic!("committed entries should gate config changes: {error}"));
    let metadata = repository
        .get_metadata("tikeo-0")
        .await
        .unwrap_or_else(|error| panic!("metadata should load: {error}"))
        .unwrap_or_else(|| panic!("metadata should exist"));

    assert_eq!(applied, Some(4));
    assert_eq!(metadata.applied_index, 4);
}

#[tokio::test]
async fn raft_apply_committed_conf_change_adds_member_after_commit() {
    let repository = test_raft_repository().await;
    let mut node = test_raw_node("tikeo-0", &["tikeo-0"]);
    let proposal = RaftMembershipProposal {
        proposal_id: "prop-add-3".to_owned(),
        action: "add_voter".to_owned(),
        node_id: "tikeo-3".to_owned(),
        endpoint: Some("http://tikeo-3.tikeo-headless:9998".to_owned()),
    };
    repository
        .record_membership_proposal(tikeo_storage::RecordRaftMembershipProposal {
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

    let applied = apply_committed_entries("tikeo-0", &repository, Some(&mut node), &[entry])
        .await
        .unwrap_or_else(|error| panic!("conf change should apply: {error}"));
    let metadata = repository
        .get_metadata("tikeo-0")
        .await
        .unwrap_or_else(|error| panic!("metadata should load: {error}"))
        .unwrap_or_else(|| panic!("metadata should exist"));
    let member = repository
        .get_member("tikeo-3")
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
    let mut node = test_raw_node("tikeo-0", &["tikeo-0"]);
    let mut entry = raft::eraftpb::Entry::new();
    entry.set_entry_type(EntryType::EntryConfChange);
    entry.index = 21;
    entry.term = 5;
    entry.data = b"bad-conf-change".to_vec().into();

    let applied = apply_committed_entries("tikeo-0", &repository, Some(&mut node), &[entry])
        .await
        .unwrap_or_else(|error| {
            panic!("malformed conf change should be recorded as handled: {error}")
        });
    let metadata = repository
        .get_metadata("tikeo-0")
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
    conf_change.node_id = raft_numeric_id("tikeo-4");
    conf_change.context = serde_json::to_vec(&RaftMembershipProposalContext {
        proposal_id: "prop-add-4".to_owned(),
        action: "add_voter".to_owned(),
        node_id: "tikeo-4".to_owned(),
        endpoint: Some("http://tikeo-4.tikeo-headless:9998".to_owned()),
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

    let applied = apply_committed_entries("tikeo-0", &repository, None, &[entry])
        .await
        .unwrap_or_else(|error| panic!("conf change without runtime should gate: {error}"));
    let metadata = repository
        .get_metadata("tikeo-0")
        .await
        .unwrap_or_else(|error| panic!("metadata should load: {error}"))
        .unwrap_or_else(|| panic!("metadata should exist"));

    assert_eq!(applied, None);
    assert_eq!(metadata.applied_index, 0);
}

#[test]
fn leader_fencing_token_requires_real_leader_role_and_term() {
    assert_eq!(
        leader_fencing_token_for_role(ClusterRole::Leader, "tikeo-0", 7).as_deref(),
        Some("raft:term:7:node:tikeo-0")
    );
    assert_eq!(
        leader_fencing_token_for_role(ClusterRole::Leader, "tikeo-0", 0),
        None
    );
    assert_eq!(
        leader_fencing_token_for_role(ClusterRole::Follower, "tikeo-0", 7),
        None
    );
}

#[tokio::test]
async fn raft_runtime_starts_ticker_without_granting_tikeo_ownership() {
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
    message.from = raft_numeric_id("tikeo-1");
    message.to = raft_numeric_id("tikeo-0");
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
async fn raft_inprocess_harness_autonomously_elects_unique_leader_after_ticks() {
    let mut cluster = TestRaftCluster::new(&["tikeo-0", "tikeo-1", "tikeo-2"]).await;

    cluster.tick_all(12).await;
    cluster.drain().await;

    let leaders = cluster.leader_ids();
    assert_eq!(
        leaders.len(),
        1,
        "exactly one raft leader should be elected autonomously"
    );
    let leader_id = leaders[0].clone();
    let status = cluster.nodes[&leader_id].status.read().await.clone();
    let metadata = cluster.nodes[&leader_id]
        .repository
        .get_metadata(&leader_id)
        .await
        .unwrap_or_else(|error| panic!("metadata should load: {error}"))
        .unwrap_or_else(|| panic!("metadata should exist"));

    assert_eq!(status.role, ClusterRole::Leader);
    assert!(status.can_schedule);
    assert_eq!(
        status.leader_fencing_token.as_deref(),
        Some(format!("raft:term:1:node:{leader_id}").as_str())
    );
    assert_eq!(metadata.leader_fencing_token, status.leader_fencing_token);
}

#[tokio::test]
async fn raft_inprocess_harness_elects_real_leader_and_persists_fencing() {
    let mut cluster = TestRaftCluster::new(&["tikeo-0", "tikeo-1", "tikeo-2"]).await;
    cluster
        .nodes
        .get_mut("tikeo-0")
        .unwrap_or_else(|| panic!("tikeo-0 should exist"))
        .raw
        .campaign()
        .unwrap_or_else(|error| panic!("campaign should start: {error}"));
    cluster.drain().await;

    let leaders = cluster.leader_ids();
    let status = cluster.nodes["tikeo-0"].status.read().await.clone();
    let metadata = cluster.nodes["tikeo-0"]
        .repository
        .get_metadata("tikeo-0")
        .await
        .unwrap_or_else(|error| panic!("metadata should load: {error}"))
        .unwrap_or_else(|| panic!("metadata should exist"));

    assert_eq!(leaders, vec!["tikeo-0".to_owned()]);
    assert_eq!(status.role, ClusterRole::Leader);
    assert!(status.can_schedule);
    assert_eq!(
        metadata.leader_fencing_token.as_deref(),
        Some("raft:term:1:node:tikeo-0")
    );
}

#[tokio::test]
async fn raft_inprocess_membership_proposal_commits_and_applies_member() {
    let mut cluster = TestRaftCluster::new(&["tikeo-0", "tikeo-1", "tikeo-2"]).await;
    cluster
        .nodes
        .get_mut("tikeo-0")
        .unwrap_or_else(|| panic!("tikeo-0 should exist"))
        .raw
        .campaign()
        .unwrap_or_else(|error| panic!("campaign should start: {error}"));
    cluster.drain().await;
    let proposal = RaftMembershipProposal {
        proposal_id: "prop-add-4-e2e".to_owned(),
        action: "add_voter".to_owned(),
        node_id: "tikeo-4".to_owned(),
        endpoint: Some("http://tikeo-4.tikeo-headless:9998".to_owned()),
    };
    cluster.record_proposal("tikeo-0", &proposal).await;
    let conf_change = build_membership_conf_change(&proposal)
        .unwrap_or_else(|error| panic!("conf change should build: {error}"));
    cluster
        .nodes
        .get_mut("tikeo-0")
        .unwrap_or_else(|| panic!("tikeo-0 should exist"))
        .raw
        .propose_conf_change(conf_change.get_context().to_vec(), conf_change)
        .unwrap_or_else(|error| panic!("conf change should propose: {error}"));
    cluster.drain().await;

    let leader = &cluster.nodes["tikeo-0"];
    let member = leader
        .repository
        .get_member("tikeo-4")
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
        .get_metadata("tikeo-0")
        .await
        .unwrap_or_else(|error| panic!("metadata should load: {error}"))
        .unwrap_or_else(|| panic!("metadata should exist"));

    assert_eq!(member.status, "active");
    assert_eq!(proposal.status, "applied");
    assert!(metadata.conf_state.is_some());
}

#[tokio::test]
async fn raft_leader_status_skips_shard_projection_without_active_members() {
    let repository = test_raft_repository_for("tikeo-0").await;
    for node_id in ["tikeo-0", "tikeo-1"] {
        repository
            .upsert_member(UpsertRaftMember {
                node_id: node_id.to_owned(),
                endpoint: format!("http://{node_id}.tikeo-headless:9999"),
                status: "removed".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("member {node_id} should persist: {error}"));
    }
    let mut config = test_raft_config_for("tikeo-0", &["tikeo-0", "tikeo-1"]);
    config.scheduler_shard_map_version = 7;
    config.scheduler_shard_count = 4;
    let mut raw = test_raw_node_from_ids("tikeo-0", vec![raft_numeric_id("tikeo-0")]);
    raw.raft.become_candidate();
    raw.raft.become_leader();
    let status = Arc::new(RwLock::new(test_cluster_status("tikeo-0", 2)));

    update_runtime_status(&config, &repository, &raw, &status)
        .await
        .unwrap_or_else(|error| panic!("status update should not fail: {error}"));

    let ownership = tikeo_storage::ClusterShardOwnershipRepository::new(repository.db())
        .list()
        .await
        .unwrap_or_else(|error| panic!("ownership should list: {error}"));
    assert!(
        ownership.is_empty(),
        "removed members must not receive projected shards: {ownership:?}"
    );
}

#[tokio::test]
async fn raft_leader_status_excludes_non_active_members_from_shard_projection() {
    let repository = test_raft_repository_for("tikeo-0").await;
    for (node_id, status) in [
        ("tikeo-0", "active"),
        ("tikeo-1", "removed"),
        ("tikeo-2", "active"),
    ] {
        repository
            .upsert_member(UpsertRaftMember {
                node_id: node_id.to_owned(),
                endpoint: format!("http://{node_id}.tikeo-headless:9999"),
                status: status.to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("member {node_id} should persist: {error}"));
    }
    let mut config = test_raft_config_for("tikeo-0", &["tikeo-0", "tikeo-1", "tikeo-2"]);
    config.scheduler_shard_map_version = 4;
    config.scheduler_shard_count = 6;
    let mut raw = test_raw_node_from_ids("tikeo-0", vec![raft_numeric_id("tikeo-0")]);
    raw.raft.become_candidate();
    raw.raft.become_leader();
    let status = Arc::new(RwLock::new(test_cluster_status("tikeo-0", 3)));

    update_runtime_status(&config, &repository, &raw, &status)
        .await
        .unwrap_or_else(|error| panic!("status update should project shards: {error}"));

    let ownership = tikeo_storage::ClusterShardOwnershipRepository::new(repository.db())
        .list()
        .await
        .unwrap_or_else(|error| panic!("ownership should list: {error}"));
    let owners = ownership
        .iter()
        .map(|row| row.owner_node_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        owners,
        std::collections::BTreeSet::from(["tikeo-0", "tikeo-2"])
    );
    assert!(ownership.iter().all(|row| row.owner_node_id != "tikeo-1"));
}

#[tokio::test]
async fn raft_leader_status_rebalances_shards_with_minimal_movement() {
    let repository = test_raft_repository_for("tikeo-0").await;
    for node_id in ["tikeo-0", "tikeo-1", "tikeo-2"] {
        repository
            .upsert_member(UpsertRaftMember {
                node_id: node_id.to_owned(),
                endpoint: format!("http://{node_id}.tikeo-headless:9999"),
                status: "active".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("member {node_id} should persist: {error}"));
    }
    let mut config = test_raft_config_for("tikeo-0", &["tikeo-0", "tikeo-1", "tikeo-2"]);
    config.scheduler_shard_map_version = 5;
    config.scheduler_shard_count = 6;
    let shard_repository = tikeo_storage::ClusterShardOwnershipRepository::new(repository.db());
    for shard_id in 0..6 {
        let owner_node_id = if shard_id < 3 { "tikeo-0" } else { "tikeo-1" };
        shard_repository
            .upsert_newer(UpsertClusterShardOwnership {
                shard_id,
                shard_map_version: 5,
                shard_count: 6,
                owner_node_id: owner_node_id.to_owned(),
                epoch: 1,
                raft_term: 1,
                lease_seconds: Some(30),
            })
            .await
            .unwrap_or_else(|error| panic!("seed ownership should persist: {error}"));
    }
    let before = shard_repository
        .list()
        .await
        .unwrap_or_else(|error| panic!("seed ownership should list: {error}"));
    project_balanced_shard_ownership(&config, &repository, 2, Some("raft:term:2:node:tikeo-0"))
        .await
        .unwrap_or_else(|error| panic!("status update should rebalance shards: {error}"));

    let after = shard_repository
        .list()
        .await
        .unwrap_or_else(|error| panic!("ownership should list: {error}"));
    let moved = after
        .iter()
        .filter(|row| {
            before
                .iter()
                .find(|candidate| candidate.shard_id == row.shard_id)
                .is_some_and(|old| old.owner_node_id != row.owner_node_id)
        })
        .count();
    let owner_counts = after
        .iter()
        .fold(std::collections::BTreeMap::new(), |mut counts, row| {
            *counts.entry(row.owner_node_id.clone()).or_insert(0usize) += 1;
            counts
        });
    assert_eq!(
        moved, 2,
        "adding a third owner to six shards only needs two moves: {after:?}"
    );
    assert_eq!(owner_counts.get("tikeo-0"), Some(&2));
    assert_eq!(owner_counts.get("tikeo-1"), Some(&2));
    assert_eq!(owner_counts.get("tikeo-2"), Some(&2));
}

#[tokio::test]
async fn raft_shard_projection_rebalances_membership_change_within_same_term() {
    let repository = test_raft_repository_for("tikeo-0").await;
    for node_id in ["tikeo-0", "tikeo-1", "tikeo-2"] {
        repository
            .upsert_member(UpsertRaftMember {
                node_id: node_id.to_owned(),
                endpoint: format!("http://{node_id}.tikeo-headless:9999"),
                status: "active".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("member {node_id} should persist: {error}"));
    }
    let mut config = test_raft_config_for("tikeo-0", &["tikeo-0", "tikeo-1", "tikeo-2"]);
    config.scheduler_shard_map_version = 6;
    config.scheduler_shard_count = 6;
    let shard_repository = tikeo_storage::ClusterShardOwnershipRepository::new(repository.db());
    for shard_id in 0..6 {
        let owner_node_id = if shard_id < 3 { "tikeo-0" } else { "tikeo-1" };
        shard_repository
            .upsert_newer(UpsertClusterShardOwnership {
                shard_id,
                shard_map_version: 6,
                shard_count: 6,
                owner_node_id: owner_node_id.to_owned(),
                epoch: 2,
                raft_term: 2,
                lease_seconds: Some(30),
            })
            .await
            .unwrap_or_else(|error| panic!("seed ownership should persist: {error}"));
    }

    project_balanced_shard_ownership(&config, &repository, 2, Some("raft:term:2:node:tikeo-0"))
        .await
        .unwrap_or_else(|error| panic!("same-term membership change should rebalance: {error}"));

    let after = shard_repository
        .list()
        .await
        .unwrap_or_else(|error| panic!("ownership should list: {error}"));
    let owner_counts = after
        .iter()
        .fold(std::collections::BTreeMap::new(), |mut counts, row| {
            *counts.entry(row.owner_node_id.clone()).or_insert(0usize) += 1;
            counts
        });
    assert_eq!(owner_counts.get("tikeo-0"), Some(&2));
    assert_eq!(owner_counts.get("tikeo-1"), Some(&2));
    assert_eq!(owner_counts.get("tikeo-2"), Some(&2));
    assert!(
        after
            .iter()
            .any(|row| row.owner_node_id == "tikeo-2" && row.epoch == 3),
        "same-term moved shards need a fresh ownership epoch: {after:?}"
    );
}

#[tokio::test]
async fn raft_leader_status_balances_shards_across_active_members() {
    let repository = test_raft_repository_for("tikeo-0").await;
    let config = ClusterConfig {
        mode: ClusterModeConfig::Raft,
        node_id: "tikeo-0".to_owned(),
        peers: vec![
            ClusterPeerConfig {
                node_id: "tikeo-0".to_owned(),
                endpoint: "http://tikeo-0.tikeo-headless:9999".to_owned(),
            },
            ClusterPeerConfig {
                node_id: "tikeo-1".to_owned(),
                endpoint: "http://tikeo-1.tikeo-headless:9999".to_owned(),
            },
        ],
        transport_token: None,
        scheduler_shard_map_version: 3,
        scheduler_shard_count: 4,
    };
    let mut raw = test_raw_node_from_ids("tikeo-0", vec![raft_numeric_id("tikeo-0")]);
    raw.raft.become_candidate();
    raw.raft.become_leader();
    let status = Arc::new(RwLock::new(test_cluster_status("tikeo-0", 2)));

    update_runtime_status(&config, &repository, &raw, &status)
        .await
        .unwrap_or_else(|error| panic!("status update should project shards: {error}"));

    let runtime = status.read().await.clone();
    assert!(runtime.can_schedule);
    let ownership = tikeo_storage::ClusterShardOwnershipRepository::new(repository.db())
        .list()
        .await
        .unwrap_or_else(|error| panic!("ownership should list: {error}"));
    assert_eq!(ownership.len(), 4);
    let owner_counts =
        ownership
            .iter()
            .fold(std::collections::BTreeMap::new(), |mut counts, row| {
                *counts.entry(row.owner_node_id.clone()).or_insert(0usize) += 1;
                counts
            });
    assert_eq!(owner_counts.get("tikeo-0"), Some(&2));
    assert_eq!(owner_counts.get("tikeo-1"), Some(&2));
    assert!(ownership.iter().all(|row| row.shard_map_version == 3));
    assert!(ownership.iter().all(|row| row.shard_count == 4));
    assert!(ownership.iter().all(|row| row.epoch == 1));
}

fn test_raft_config() -> ClusterConfig {
    ClusterConfig {
        mode: ClusterModeConfig::Raft,
        node_id: "tikeo-0".to_owned(),
        peers: vec![
            ClusterPeerConfig {
                node_id: "tikeo-0".to_owned(),
                endpoint: "http://tikeo-0.tikeo-headless:9999".to_owned(),
            },
            ClusterPeerConfig {
                node_id: "tikeo-1".to_owned(),
                endpoint: "http://tikeo-1.tikeo-headless:9999".to_owned(),
            },
        ],
        transport_token: None,
        scheduler_shard_map_version: 1,
        scheduler_shard_count: 64,
    }
}

fn test_raft_config_for(node_id: &str, node_ids: &[&str]) -> ClusterConfig {
    ClusterConfig {
        mode: ClusterModeConfig::Raft,
        node_id: node_id.to_owned(),
        peers: node_ids
            .iter()
            .map(|peer| ClusterPeerConfig {
                node_id: (*peer).to_owned(),
                endpoint: format!("http://{peer}.tikeo-headless:9999"),
            })
            .collect(),
        transport_token: None,
        scheduler_shard_map_version: 1,
        scheduler_shard_count: 64,
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
            node_id: "tikeo-0".to_owned(),
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
    config: ClusterConfig,
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
                    config: test_raft_config_for(node_id, node_ids),
                    raw: test_raw_node_from_ids(node_id, voter_ids.clone()),
                    repository: test_raft_repository_for(node_id).await,
                    status: Arc::new(RwLock::new(test_cluster_status(node_id, node_ids.len()))),
                },
            );
        }
        Self { nodes }
    }

    async fn tick_all(&mut self, ticks: usize) {
        for _ in 0..ticks {
            for node in self.nodes.values_mut() {
                node.raw.tick();
                trigger_autonomous_campaign(&mut node.raw);
            }
            self.drain().await;
        }
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
            .record_membership_proposal(tikeo_storage::RecordRaftMembershipProposal {
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

async fn process_test_ready(node_id: &str, node: &mut TestRaftNode) -> Vec<raft::eraftpb::Message> {
    if !node.raw.has_ready() {
        update_runtime_status(&node.config, &node.repository, &node.raw, &node.status)
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
    update_runtime_status(&node.config, &node.repository, &node.raw, &node.status)
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
