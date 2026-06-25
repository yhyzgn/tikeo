    #[tokio::test]
    async fn raft_append_entries_enqueues_when_runtime_exists_without_leadership() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let cluster = coordinator_from_config_with_storage(
            &ClusterConfig {
                mode: ClusterModeConfig::Raft,
                node_id: "tikeo-0".to_owned(),
                peers: vec![
                    ClusterPeerConfig {
                        node_id: "tikeo-0".to_owned(),
                        endpoint: "http://tikeo-0.tikeo-headless:9998".to_owned(),
                    },
                    ClusterPeerConfig {
                        node_id: "tikeo-1".to_owned(),
                        endpoint: "http://tikeo-1.tikeo-headless:9998".to_owned(),
                    },
                ],
                transport_token: None,
                scheduler_shard_map_version: 1,
                scheduler_shard_count: 64,
            },
            &RaftRepository::new(db.clone()),
        )
        .await
        .unwrap_or_else(|error| panic!("raft coordinator should start: {error}"));
        let app = router_with_state(app_state!(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            cluster,
        ));

        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    "/api/v1/raft/append-entries",
                    r#"{"from":1,"to":2,"term":1,"message_type":"MsgHeartbeat","index":0,"log_term":0,"commit":0,"snapshot_index":null,"snapshot_term":null,"entries":[],"context":null,"reject":false,"reject_hint":null,"leaderFencingToken":null}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(response.status().is_success());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["accepted"], true);
        assert!(
            json["data"]["reason"]
                .as_str()
                .is_some_and(|value| value.contains("enqueued"))
        );
        assert_eq!(json["data"]["local_role"], "follower");
        assert_eq!(
            json["data"]["leaderFencingToken"],
            serde_json::Value::Null
        );
    }

    #[tokio::test]
    async fn raft_append_entries_internal_token_bypasses_human_session_only_for_transport() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let cluster = coordinator_from_config_with_storage(
            &ClusterConfig {
                mode: ClusterModeConfig::Raft,
                node_id: "tikeo-0".to_owned(),
                peers: vec![
                    ClusterPeerConfig {
                        node_id: "tikeo-0".to_owned(),
                        endpoint: "http://tikeo-0.tikeo-headless:9998".to_owned(),
                    },
                    ClusterPeerConfig {
                        node_id: "tikeo-1".to_owned(),
                        endpoint: "http://tikeo-1.tikeo-headless:9998".to_owned(),
                    },
                ],
                transport_token: Some("secret-raft-token".to_owned()),
                scheduler_shard_map_version: 1,
                scheduler_shard_count: 64,
            },
            &RaftRepository::new(db.clone()),
        )
        .await
        .unwrap_or_else(|error| panic!("raft coordinator should start: {error}"));
        let app = router_with_state(
            app_state!(
                JobRepository::new(db.clone()),
                JobInstanceRepository::new(db.clone()),
                JobInstanceLogRepository::new(db.clone()),
                JobInstanceAttemptRepository::new(db.clone()),
                UserRepository::new(db.clone()),
                ScriptRepository::new(db.clone()),
                WorkflowRepository::new(db.clone()),
                AuditLogRepository::new(db.clone()),
                crate::tunnel::WorkerRegistry::default(),
                cluster,
            )
            .with_raft_transport_token(Some("secret-raft-token".to_owned())),
        );
        let body = r#"{"from":1,"to":2,"term":1,"message_type":"MsgHeartbeat","index":0,"log_term":0,"commit":0,"snapshot_index":null,"snapshot_term":null,"entries":[],"context":null,"reject":false,"reject_hint":null,"leaderFencingToken":null}"#;

        let accepted = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/raft/append-entries")
                    .header("content-type", "application/json")
                    .header("x-tikeo-raft-token", "secret-raft-token")
                    .body(Body::from(body))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(accepted.status().is_success());
        let accepted_body = axum::body::to_bytes(accepted.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let accepted_json: Value = serde_json::from_slice(&accepted_body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(accepted_json["code"], 0);
        assert_eq!(accepted_json["data"]["accepted"], true);
        assert_eq!(accepted_json["data"]["local_role"], "follower");
        assert_eq!(accepted_json["data"]["leaderFencingToken"], Value::Null);

        let rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/raft/append-entries")
                    .header("content-type", "application/json")
                    .header("x-tikeo-raft-token", "wrong-token")
                    .body(Body::from(body))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(rejected.status(), axum::http::StatusCode::UNAUTHORIZED);
        let rejected_body = axum::body::to_bytes(rejected.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let rejected_json: Value = serde_json::from_slice(&rejected_body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_ne!(rejected_json["code"], 0);
        assert!(rejected_json.get("data").is_some());
    }

    #[tokio::test]
    async fn raft_membership_proposal_requires_real_leader_fencing() {
        let app = router().await;
        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    "/api/v1/raft/members:propose",
                    r#"{"proposal_id":"prop-1","action":"add_voter","node_id":"tikeo-2","endpoint":"http://tikeo-2.tikeo-headless:9998"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_ne!(json["code"], 0);
        assert!(
            json["message"]
                .as_str()
                .is_some_and(|value| value.contains("persisted fencing token"))
        );
        assert!(json.get("data").is_some());
    }

    #[tokio::test]
    async fn raft_membership_proposal_validates_endpoint_before_storing() {
        let app = router_with_leader_cluster().await;
        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    "/api/v1/raft/members:propose",
                    r#"{"proposal_id":"prop-bad","action":"add_voter","node_id":"tikeo-2","endpoint":"ftp://tikeo-2"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_ne!(json["code"], 0);
        assert!(
            json["message"]
                .as_str()
                .is_some_and(|value| value.contains("http or https"))
        );
    }

    #[tokio::test]
    async fn raft_membership_proposal_records_intent_idempotently() {
        let app = router_with_leader_cluster().await;
        let request = r#"{"proposal_id":"prop-add-2","action":"add_voter","node_id":"tikeo-2","endpoint":"http://tikeo-2.tikeo-headless:9998"}"#;

        let first = post_json(app.clone(), "/api/v1/raft/members:propose", request).await;
        let second = post_json(app, "/api/v1/raft/members:propose", request).await;

        assert_eq!(first["code"], 0);
        assert_eq!(first["data"]["accepted"], false);
        assert_eq!(first["data"]["proposal"]["status"], "rejected");
        assert_eq!(second["code"], 0);
        assert_eq!(
            first["data"]["proposal"]["id"],
            second["data"]["proposal"]["id"]
        );
        assert!(
            second["data"]["reason"]
                .as_str()
                .is_some_and(|value| value.contains("runtime is not available"))
        );
    }

    #[tokio::test]
    async fn audit_logs_support_server_side_filters_and_pagination() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let audit = AuditLogRepository::new(db.clone());
        audit
            .append(CreateAuditLog {
                actor: "alice".to_owned(),
                action: "create".to_owned(),
                resource_type: "job".to_owned(),
                resource_id: "job-1".to_owned(),
                detail: None,
                before: None,
                after: None,
                trace_id: None,
                result: "success".to_owned(),
                failure_reason: None,
                ip_address: None,
            })
            .await
            .unwrap_or_else(|error| panic!("audit should append: {error}"));
        audit
            .append(CreateAuditLog {
                actor: "bob".to_owned(),
                action: "delete".to_owned(),
                resource_type: "script".to_owned(),
                resource_id: "script-1".to_owned(),
                detail: Some("delete script".to_owned()),
                before: Some(r#"{"status":"enabled"}"#.to_owned()),
                after: Some(r#"{"status":"deleted"}"#.to_owned()),
                trace_id: Some("trace-audit-1".to_owned()),
                result: "failed".to_owned(),
                failure_reason: Some("dry-run failure sample".to_owned()),
                ip_address: Some("10.0.0.1".to_owned()),
            })
            .await
            .unwrap_or_else(|error| panic!("audit should append: {error}"));
        let app = router_with_state(app_state!(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            audit,
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));

        let response = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    "/api/v1/audit-logs?action=delete&resource_type=script&page_size=1",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["total"], 1);
        assert_eq!(json["data"]["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["data"]["items"][0]["actor"], "bob");
        assert_eq!(json["data"]["items"][0]["resource_type"], "script");
        assert_eq!(json["data"]["items"][0]["trace_id"], "trace-audit-1");
        assert_eq!(json["data"]["items"][0]["result"], "failed");
        assert_eq!(
            json["data"]["items"][0]["failure_reason"],
            "dry-run failure sample"
        );
        assert!(json["data"]["items"][0]["before"].is_string());
        assert!(json["data"]["items"][0]["after"].is_string());

        let export = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    "/api/v1/audit-logs:export?action=delete&resource_type=script&format=json",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("export route should respond: {error}"));
        let export_body = axum::body::to_bytes(export.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("export body should collect: {error}"));
        let export_json: Value = serde_json::from_slice(&export_body)
            .unwrap_or_else(|error| panic!("export body should be JSON: {error}"));
        assert_eq!(export_json["code"], 0);
        assert_eq!(export_json["data"]["format"], "json");
        assert_eq!(export_json["data"]["exported"], 1);
        assert_eq!(export_json["data"]["max_rows"], 500);
        assert_eq!(export_json["data"]["redacted"], false);
        assert!(
            export_json["data"]["governance"]
                .as_str()
                .is_some_and(|value| value.contains("capped at 500 rows"))
        );
        assert_eq!(export_json["data"]["items"][0]["trace_id"], "trace-audit-1");

        let csv = app
            .clone()
            .oneshot(
                admin_request_builder(app, "GET", "/api/v1/audit-logs:export?format=csv").await,
            )
            .await
            .unwrap_or_else(|error| panic!("csv export route should respond: {error}"));
        assert_eq!(csv.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn alert_rules_api_records_script_governance_event_history() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let app = router_with_state(app_state!(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));

        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/alert-rules",
                    r#"{"name":"Runtime governance","severity":"warning","condition":{"type":"script_governance_failure","failure_class":"script_runtime_unavailable","threshold":1},"channels":[{"type":"webhook","url":"http://127.0.0.1:9/alert?token=secret"}],"enabled":true,"dedupe_seconds":300}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert rule route should respond: {error}"));
        assert!(created.status().is_success());
        let body = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(
            json["data"]["condition"]["type"],
            "script_governance_failure"
        );
        let created_text = json["data"]["channels"].to_string();
        assert!(!created_text.contains("token=secret"));
        assert_eq!(json["data"]["channels"][0]["target_redacted"], "http://127.0.0.1:9/...");
        let listed = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/alert-rules").await)
            .await
            .unwrap_or_else(|error| panic!("alert rule list should respond: {error}"));
        let body = axum::body::to_bytes(listed.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let listed_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let listed_text = listed_json["data"].to_string();
        assert!(!listed_text.contains("token=secret"));
        assert_eq!(listed_json["data"][0]["channels"][0]["target_redacted"], "http://127.0.0.1:9/...");

        crate::tunnel::governance::materialize_script_governance_audit(
            &AuditLogRepository::new(db.clone()),
            "tikeo-dispatcher",
            "inst-alert-1",
            "script_runtime_unavailable",
            "runtime missing",
        )
        .await
        .unwrap_or_else(|error| panic!("governance materialization should append: {error}"));
        crate::tunnel::governance::materialize_script_governance_audit(
            &AuditLogRepository::new(db.clone()),
            "tikeo-dispatcher",
            "inst-alert-2",
            "script_runtime_unavailable",
            "runtime still missing",
        )
        .await
        .unwrap_or_else(|error| panic!("governance materialization should append: {error}"));

        let events = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    "/api/v1/alert-events?resource_type=script_execution_governance&failure_class=script_runtime_unavailable",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert events route should respond: {error}"));
        assert!(events.status().is_success());
        let body = axum::body::to_bytes(events.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"].as_array().map(Vec::len), Some(2));
        assert_eq!(json["data"][0]["status"], "suppressed");
        assert_eq!(json["data"][1]["status"], "firing");
        assert_eq!(json["data"][1]["resource_id"], "inst-alert-1");

        let attempts = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    &format!(
                        "/api/v1/alert-delivery-attempts?event_id={}",
                        json["data"][1]["id"]
                            .as_str()
                            .unwrap_or_else(|| panic!("event id should exist"))
                    ),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| {
                panic!("alert delivery attempts route should respond: {error}")
            });
        assert!(attempts.status().is_success());
        let body = axum::body::to_bytes(attempts.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["data"][0]["provider"], "webhook");
        assert_eq!(json["data"][0]["delivered"], false);
        assert_eq!(json["data"][0]["target"], "http://127.0.0.1:9/...");
        assert!(
            json["data"][0]["error"]
                .as_str()
                .is_some_and(|value| value.contains("https"))
        );
    }

    #[tokio::test]
    async fn alert_event_recovery_appends_resolved_history_entry() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let app = router_with_state(app_state!(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));
        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/alert-rules",
                    r#"{"name":"Runtime governance","severity":"warning","condition":{"type":"script_governance_failure","failure_class":"script_runtime_unavailable","threshold":1},"channels":[],"enabled":true,"dedupe_seconds":300}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert rule route should respond: {error}"));
        assert!(created.status().is_success());
        let body = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let rule_id = json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("rule id"))
            .to_owned();

        crate::tunnel::governance::materialize_script_governance_audit(
            &AuditLogRepository::new(db.clone()),
            "tikeo-dispatcher",
            "inst-alert-recover",
            "script_runtime_unavailable",
            "runtime missing",
        )
        .await
        .unwrap_or_else(|error| panic!("governance materialization should append: {error}"));

        let before = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    "/api/v1/alert-events?resource_type=script_execution_governance&failure_class=script_runtime_unavailable",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert events route should respond: {error}"));
        let before_body = axum::body::to_bytes(before.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let before_json: Value = serde_json::from_slice(&before_body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let event_id = before_json["data"][0]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("event id"));
        assert_eq!(before_json["data"][0]["status"], "firing");
        assert_eq!(before_json["data"][0]["rule_id"], rule_id);

        let resolved = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "POST",
                    &format!("/api/v1/alert-events/{event_id}/resolve"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("resolve route should respond: {error}"));
        assert!(resolved.status().is_success());

        let after = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    "/api/v1/alert-events?resource_type=script_execution_governance&failure_class=script_runtime_unavailable",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert events route should respond: {error}"));
        let after_body = axum::body::to_bytes(after.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let after_json: Value = serde_json::from_slice(&after_body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(after_json["data"].as_array().map(Vec::len), Some(2));
        assert_eq!(after_json["data"][0]["status"], "recovered");
        assert_eq!(after_json["data"][1]["status"], "firing");
    }

    #[tokio::test]
    async fn alert_event_summary_rolls_up_history_by_rule_and_resource() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let app = router_with_state(app_state!(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));
        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/alert-rules",
                    r#"{"name":"Runtime governance","severity":"warning","condition":{"type":"script_governance_failure","failure_class":"script_runtime_unavailable","threshold":1},"channels":[],"enabled":true,"dedupe_seconds":300}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert rule route should respond: {error}"));
        assert!(created.status().is_success());

        crate::tunnel::governance::materialize_script_governance_audit(
            &AuditLogRepository::new(db.clone()),
            "tikeo-dispatcher",
            "inst-alert-summary",
            "script_runtime_unavailable",
            "runtime missing",
        )
        .await
        .unwrap_or_else(|error| panic!("governance materialization should append: {error}"));
        crate::tunnel::governance::materialize_script_governance_audit(
            &AuditLogRepository::new(db.clone()),
            "tikeo-dispatcher",
            "inst-alert-summary",
            "script_runtime_unavailable",
            "runtime still missing",
        )
        .await
        .unwrap_or_else(|error| panic!("governance materialization should append: {error}"));

        let before = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    "/api/v1/alert-events?resource_type=script_execution_governance&failure_class=script_runtime_unavailable",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert events route should respond: {error}"));
        let before_body = axum::body::to_bytes(before.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let before_json: Value = serde_json::from_slice(&before_body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let event_id = before_json["data"][0]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("event id"));
        assert_eq!(before_json["data"][0]["status"], "suppressed");
        assert_eq!(before_json["data"][1]["status"], "firing");

        app.clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "POST",
                    &format!("/api/v1/alert-events/{event_id}/resolve"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("resolve route should respond: {error}"));

        let summary = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    "/api/v1/alert-events:summary?resource_type=script_execution_governance&failure_class=script_runtime_unavailable",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("summary route should respond: {error}"));
        assert!(summary.status().is_success());
        let body = axum::body::to_bytes(summary.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["data"][0]["rule_name"], "Runtime governance");
        assert_eq!(json["data"][0]["resource_id"], "inst-alert-summary");
        assert_eq!(
            json["data"][0]["failure_class"],
            "script_runtime_unavailable"
        );
        assert_eq!(json["data"][0]["event_count"], 3);
        assert_eq!(json["data"][0]["firing_count"], 1);
        assert_eq!(json["data"][0]["suppressed_count"], 1);
        assert_eq!(json["data"][0]["recovered_count"], 1);
        assert_eq!(json["data"][0]["latest_status"], "recovered");
    }

    #[tokio::test]
    async fn alert_rule_delivery_status_redacts_channel_targets_and_reports_readiness() {
        let app = router().await;
        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/alert-rules",
                    r#"{"name":"Webhook delivery","severity":"critical","condition":{"type":"script_governance_failure","failure_class":"script_runtime_unavailable","threshold":1},"channels":[{"type":"webhook","url":"https://hooks.example.com/token","secret":"super-secret"},{"type":"email","to":"ops@example.com","smtp_url":"smtps://smtp.example.com:465"}],"enabled":true,"dedupe_seconds":300}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert rule create should respond: {error}"));
        let body = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let created_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let rule_id = created_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("created rule id should be present"));

        let status = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    &format!("/api/v1/alert-rules/{rule_id}/delivery-status"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("delivery status route should respond: {error}"));
        let body = axum::body::to_bytes(status.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["ready"], true);
        assert_eq!(json["data"]["channel_count"], 2);
        assert_eq!(json["data"]["channels"][0]["provider"], "webhook");
        assert_eq!(json["data"]["channels"][0]["target_configured"], true);
        assert_eq!(json["data"]["channels"][0]["secret_configured"], true);
        assert_eq!(
            json["data"]["channels"][0]["target_redacted"],
            "https://hooks.example.com/..."
        );
        assert_eq!(json["data"]["channels"][0]["transport_security"], "https");
        assert_eq!(json["data"]["channels"][1]["transport_security"], "tls");
        assert!(json["data"]["channels"][0].get("url").is_none());
        assert!(json["data"]["channels"][0].get("secret").is_none());
    }

    #[tokio::test]
    async fn alert_delivery_queue_status_summarizes_retry_and_dlq_states() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let app = router_with_state(app_state!(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));
        let alert_repo = tikeo_storage::AlertRepository::new(db);
        let rule = alert_repo
            .create_rule(tikeo_storage::CreateAlertRule {
                name: "DLQ rule".to_owned(),
                severity: "critical".to_owned(),
                condition_json: serde_json::json!({"type":"script_governance_failure","failure_class":"script_runtime_unavailable","threshold":1}).to_string(),
                channels_json: serde_json::json!([{"type":"webhook","url":"https://hooks.example.com/token"}]).to_string(),
                enabled: true,
                dedupe_seconds: 300,
                silenced_until: None,
            })
            .await
            .unwrap_or_else(|error| panic!("rule should create: {error}"));
        let event = alert_repo
            .record_script_governance_failure("inst-dlq", "script_runtime_unavailable", "dlq")
            .await
            .unwrap_or_else(|error| panic!("event should create: {error}"))
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("event should exist"));
        alert_repo
            .record_delivery_attempt(tikeo_storage::RecordAlertDeliveryAttempt {
                event_id: event.id.clone(),
                rule_id: rule.id,
                provider: "webhook".to_owned(),
                target: "https://hooks.example.com/...".to_owned(),
                delivered: false,
                status_code: None,
                error: Some("boom".to_owned()),
                attempt: 3,
                retry_state: "dead_letter".to_owned(),
                next_retry_at: None,
            })
            .await
            .unwrap_or_else(|error| panic!("attempt should record: {error}"));

        let response = app
            .clone()
            .oneshot(
                admin_request_builder(app, "GET", "/api/v1/alert-delivery-attempts:queue-status")
                    .await,
            )
            .await
            .unwrap_or_else(|error| panic!("queue status route should respond: {error}"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["dead_letter"], 1);
        assert_eq!(
            json["data"]["recent_dead_letters"]
                .as_array()
                .map(std::vec::Vec::len),
            Some(1)
        );
    }


    #[tokio::test]
    async fn internal_worker_relay_route_dispatches_to_local_stream() {
        use axum::{body::Body, http::{Request, StatusCode}};
        use tonic_prost::prost::Message as _;
        use tower::ServiceExt;
        use std::collections::HashMap;
        use tikeo_proto::worker::v1::{DispatchTask, ProcessorCapability, WorkerCapabilities, server_message};

        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let logs = tikeo_storage::JobInstanceLogRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let users = UserRepository::new(db.clone());
        let scripts = ScriptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let registry = crate::tunnel::WorkerRegistry::with_lifecycle(
            tikeo_storage::WorkerLifecycleRepository::new(db),
        )
        .with_gateway_node_id("gateway-node");
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let worker = registry
            .register(
                RegisterWorker {
                    client_instance_id: "relay-local".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: Some(WorkerCapabilities {
                        normal_processors: vec![ProcessorCapability {
                            name: "billing.manual".to_owned(),
                            description: String::new(),
                        }],
                        ..WorkerCapabilities::default()
                    }),
                    election: None,
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;
        let state = app_state!(
            jobs,
            instances,
            logs,
            attempts,
            users,
            scripts,
            workflows,
            audit,
            registry,
            StaticCoordinator::shared(ClusterStatus {
                mode: ClusterMode::Raft,
                role: ClusterRole::Follower,
                node_id: "gateway-node".to_owned(),
                nodes: 3,
                can_schedule: false,
                leader_fencing_token: None,
                detail: "gateway test".to_owned(),
            }),
        )
        .with_raft_transport_token(Some("relay-secret".to_owned()));
        let app = crate::http::router_with_state(state);
        let task = DispatchTask {
            instance_id: "inst-relayed".to_owned(),
            job_id: "job-relayed".to_owned(),
            payload: Vec::new(),
            processor_name: "billing.manual".to_owned(),
            processor_binding: None,
            assignment_token: "asg-relayed".to_owned(),
        };
        let mut body = Vec::new();
        task.encode(&mut body)
            .unwrap_or_else(|error| panic!("task should encode: {error}"));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/v1/internal/worker-tunnel/dispatch/{}",
                        worker.worker_id
                    ))
                    .header("x-tikeo-raft-token", "relay-secret")
                    .body(Body::from(body))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("relay request should run: {error}"));
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let delivered = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker should receive relayed dispatch"))
            .unwrap_or_else(|error| panic!("relayed dispatch should be ok: {error}"));
        match delivered.kind {
            Some(server_message::Kind::DispatchTask(delivered)) => {
                assert_eq!(delivered.instance_id, "inst-relayed");
                assert_eq!(delivered.assignment_token, "asg-relayed");
            }
            other => panic!("unexpected relayed message: {other:?}"),
        }
    }
