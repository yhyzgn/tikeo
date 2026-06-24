use tikeo_proto::worker::v1::{
    DispatchTask, PluginProcessorCapability, ProcessorCapability, RegisterWorker,
    ScriptRunnerCapability, WorkerCapabilities, WorkerClusterElection,
};
use tokio::sync::mpsc;

use tikeo_storage::{RegisterWorkerSession, WorkerLifecycleRepository};

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
async fn registry_dispatch_returns_assignment_token_without_becoming_assignment_authority() {
    let registry = WorkerRegistry::default();
    let (tx, mut rx) = mpsc::channel(1);
    let worker = registry.register(register_worker("pod-a"), tx).await;

    let token = registry
        .dispatch_to_worker(
            &worker.worker_id,
            DispatchTask {
                instance_id: "inst-1".to_owned(),
                job_id: "job-1".to_owned(),
                payload: Vec::new(),
                processor_name: "demo.echo".to_owned(),
                processor_binding: None,
                assignment_token: String::new(),
            },
        )
        .await
        .unwrap_or_else(|| panic!("dispatch should return a token for persistence"));
    let message = rx
        .recv()
        .await
        .unwrap_or_else(|| panic!("dispatch message should arrive"))
        .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
    match message.kind {
        Some(tikeo_proto::worker::v1::server_message::Kind::DispatchTask(task)) => {
            assert_eq!(task.assignment_token, token);
            assert!(task.assignment_token.starts_with("asg-"));
        }
        other => panic!("unexpected server message: {other:?}"),
    }
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
                    normal_processors: vec![ProcessorCapability {
                        name: "demo.echo".to_owned(),
                        description: "Echo processor".to_owned(),
                    }],
                    script_runners: vec![ScriptRunnerCapability {
                        language: "python".to_owned(),
                        sandbox_backend: "srt".to_owned(),
                    }],
                    plugin_processors: vec![PluginProcessorCapability {
                        r#type: "sql".to_owned(),
                        processor_names: vec!["billing.sql-sync".to_owned()],
                        processors: Vec::new(),
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
                Some(&WorkerRequirement::NormalProcessor {
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
    let db = tikeo_storage::connect_and_migrate("sqlite::memory:")
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
    let db = tikeo_storage::connect_and_migrate("sqlite::memory:")
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

#[tokio::test]
async fn registry_persists_worker_snapshots_after_master_election() {
    let db = tikeo_storage::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let lifecycle = WorkerLifecycleRepository::new(db);
    let registry = WorkerRegistry::with_lifecycle(lifecycle.clone());

    let follower = registry
        .register(election_worker("pod-follower", 10), mpsc::channel(1).0)
        .await;
    let master = registry
        .register(
            RegisterWorker {
                structured_capabilities: Some(WorkerCapabilities {
                    tags: vec!["java".to_owned()],
                    normal_processors: vec![ProcessorCapability {
                        name: "demo.echo".to_owned(),
                        description: "Echo processor".to_owned(),
                    }],
                    script_runners: vec![ScriptRunnerCapability {
                        language: "python".to_owned(),
                        sandbox_backend: "srt".to_owned(),
                    }],
                    plugin_processors: vec![PluginProcessorCapability {
                        r#type: "sql".to_owned(),
                        processor_names: vec!["billing.sql-sync".to_owned()],
                        processors: Vec::new(),
                    }],
                }),
                ..election_worker("pod-master", 1)
            },
            mpsc::channel(1).0,
        )
        .await;

    assert!(
        master.master.is_master,
        "register should return the post-election master state"
    );
    let follower_after_election = registry
        .get(&follower.worker_id)
        .await
        .unwrap_or_else(|| panic!("follower worker should remain registered"));
    assert!(!follower_after_election.master.is_master);

    let persisted = lifecycle
        .list_online_workers(20)
        .await
        .unwrap_or_else(|error| panic!("persisted online workers should load: {error}"));
    let persisted_master = persisted
        .iter()
        .find(|worker| worker.worker_id == master.worker_id)
        .unwrap_or_else(|| panic!("master worker should be persisted online"));
    assert!(persisted_master.master_json.contains("\"isMaster\":true"));
    assert!(
        persisted_master
            .master_json
            .contains(&format!("\"masterWorkerId\":\"{}\"", master.worker_id))
    );
    let structured: serde_json::Value =
        serde_json::from_str(&persisted_master.structured_capabilities_json)
            .unwrap_or_else(|error| panic!("structured capabilities should be JSON: {error}"));
    assert_eq!(structured["normalProcessors"][0]["name"], "demo.echo");
    assert_eq!(
        structured["normalProcessors"][0]["description"],
        "Echo processor"
    );
    assert!(
        persisted_master
            .structured_capabilities_json
            .contains("\"sandboxBackend\":\"srt\"")
    );
    assert!(
        persisted_master
            .structured_capabilities_json
            .contains("\"processorNames\":[\"billing.sql-sync\"]")
    );
}

#[tokio::test]
async fn registry_lasso_prefers_local_gateway_before_remote_master() {
    let db = tikeo_storage::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let lifecycle = WorkerLifecycleRepository::new(db);
    let registry = WorkerRegistry::with_lifecycle(lifecycle.clone()).with_gateway_node_id("node-a");
    lifecycle
        .register_session(lifecycle_worker_session(
            "wrk-remote-master",
            "remote-master",
            "node-b",
            r#"{"isMaster":true,"domain":"finance/billing/prod/cn","masterWorkerId":"wrk-remote-master"}"#,
        ))
        .await
        .unwrap_or_else(|error| panic!("remote worker should persist: {error}"));
    lifecycle
        .register_session(lifecycle_worker_session(
            "wrk-local-follower",
            "local-follower",
            "node-a",
            r#"{"isMaster":false,"domain":"finance/billing/prod/cn"}"#,
        ))
        .await
        .unwrap_or_else(|error| panic!("local worker should persist: {error}"));

    let candidates = registry
        .find_lasso_persisted_dispatch_workers("finance", "billing", None, "inst-locality")
        .await;

    assert_eq!(
        candidates.first().map(String::as_str),
        Some("wrk-local-follower"),
        "LASSO must prefer local gateway delivery before remote authority to reduce cross-pod relay"
    );
}

#[tokio::test]
async fn registry_lasso_spreads_with_stable_dispatch_key_within_same_locality_bucket() {
    let db = tikeo_storage::connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
    let lifecycle = WorkerLifecycleRepository::new(db);
    let registry = WorkerRegistry::with_lifecycle(lifecycle.clone()).with_gateway_node_id("node-a");
    for worker_id in ["wrk-local-a", "wrk-local-b"] {
        lifecycle
            .register_session(lifecycle_worker_session(
                worker_id,
                worker_id,
                "node-a",
                r#"{"isMaster":false,"domain":"finance/billing/prod/cn"}"#,
            ))
            .await
            .unwrap_or_else(|error| panic!("local worker should persist: {error}"));
    }

    let first_key = registry
        .find_lasso_persisted_dispatch_workers("finance", "billing", None, "dispatch-key-a")
        .await;
    let mut alternate_key = None;
    for index in 0..256 {
        let key = format!("dispatch-key-{index}");
        let candidates = registry
            .find_lasso_persisted_dispatch_workers("finance", "billing", None, &key)
            .await;
        if candidates.first() != first_key.first() {
            alternate_key = Some((key, candidates));
            break;
        }
    }
    let (key, alternate) = alternate_key.unwrap_or_else(|| {
        panic!("rendezvous spread should pick the other worker for at least one key")
    });

    assert_eq!(first_key.len(), 2);
    assert_eq!(alternate.len(), 2);
    assert_ne!(
        first_key.first(),
        alternate.first(),
        "key {key} should spread first pick"
    );
    assert_eq!(
        alternate,
        registry
            .find_lasso_persisted_dispatch_workers("finance", "billing", None, &key)
            .await,
        "same dispatch key must produce stable worker ordering"
    );
}

fn lifecycle_worker_session(
    worker_id: &str,
    client_instance_id: &str,
    gateway_node_id: &str,
    master_json: &str,
) -> RegisterWorkerSession {
    RegisterWorkerSession {
        worker_id: worker_id.to_owned(),
        namespace_name: "finance".to_owned(),
        app_name: "billing".to_owned(),
        cluster: "prod".to_owned(),
        region: "cn".to_owned(),
        client_instance_id: client_instance_id.to_owned(),
        connection_id: format!("conn-{worker_id}"),
        gateway_node_id: gateway_node_id.to_owned(),
        fencing_token: format!("fence-{worker_id}"),
        lease_seconds: 30,
        capabilities_json: r#"["http"]"#.to_owned(),
        structured_capabilities_json: r"{}".to_owned(),
        labels_json: r"{}".to_owned(),
        master_json: master_json.to_owned(),
    }
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
