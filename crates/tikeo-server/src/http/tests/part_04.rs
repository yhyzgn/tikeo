    #[tokio::test]
    async fn tenant_scope_delete_rejects_non_empty_parents_and_deletes_empty_worker_pool() {
        let app = router().await;
        let namespace = post_json(app.clone(), "/api/v1/namespaces", r#"{"name":"ops"}"#).await;
        let app_scope = post_json(
            app.clone(),
            "/api/v1/apps",
            r#"{"namespace":"ops","name":"control"}"#,
        )
        .await;
        let pool = post_json(
            app.clone(),
            "/api/v1/worker-pools",
            r#"{"namespace":"ops","app":"control","name":"blue"}"#,
        )
        .await;

        let namespace_id = namespace["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("namespace id"));
        let app_id = app_scope["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("app id"));
        let pool_id = pool["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("pool id"));

        let blocked_namespace = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "DELETE",
                    format!("/api/v1/namespaces/{namespace_id}"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(
            blocked_namespace.status(),
            axum::http::StatusCode::BAD_REQUEST
        );

        let deleted_pool = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "DELETE",
                    format!("/api/v1/worker-pools/{pool_id}"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(deleted_pool.status(), axum::http::StatusCode::OK);

        let deleted_app = app
            .clone()
            .oneshot(
                admin_request_builder(app.clone(), "DELETE", format!("/api/v1/apps/{app_id}"))
                    .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(deleted_app.status(), axum::http::StatusCode::OK);

        let deleted_namespace = app
            .clone()
            .oneshot(
                admin_request_builder(app.clone(), "DELETE", format!("/api/v1/namespaces/{namespace_id}"))
                    .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(deleted_namespace.status(), axum::http::StatusCode::OK);

        for (resource_type, resource_id) in [
            ("worker_pool", pool_id),
            ("app", app_id),
            ("namespace", namespace_id),
        ] {
            let audit = app
                .clone()
                .oneshot(
                    admin_request_builder(
                        app.clone(),
                        "GET",
                        format!("/api/v1/audit-logs?action=delete&resource_type={resource_type}&resource_id={resource_id}&page_size=1"),
                    )
                    .await,
                )
                .await
                .unwrap_or_else(|error| panic!("delete audit should respond: {error}"));
            assert!(audit.status().is_success());
            let body = axum::body::to_bytes(audit.into_body(), usize::MAX)
                .await
                .unwrap_or_else(|error| panic!("audit body should collect: {error}"));
            let json: Value = serde_json::from_slice(&body)
                .unwrap_or_else(|error| panic!("audit body should be JSON: {error}"));
            assert_eq!(json["data"]["items"][0]["action"], "delete");
            assert_eq!(json["data"]["items"][0]["resource_type"], resource_type);
            assert_eq!(json["data"]["items"][0]["resource_id"], resource_id);
        }
    }

    #[tokio::test]
    async fn tenant_scope_management_api_creates_and_lists_namespaces_apps_and_worker_pools() {
        let app = router().await;

        let namespace =
            post_json(app.clone(), "/api/v1/namespaces", r#"{"name":"payments"}"#).await;
        assert_eq!(namespace["code"], 0);
        assert_eq!(namespace["data"]["name"], "payments");

        let app_scope = post_json(
            app.clone(),
            "/api/v1/apps",
            r#"{"namespace":"payments","name":"settlement"}"#,
        )
        .await;
        assert_eq!(app_scope["code"], 0);
        assert_eq!(app_scope["data"]["namespace"], "payments");
        assert_eq!(app_scope["data"]["name"], "settlement");

        let pool = post_json(
            app.clone(),
            "/api/v1/worker-pools",
            r#"{"namespace":"payments","app":"settlement","name":"critical"}"#,
        )
        .await;
        assert_eq!(pool["code"], 0);
        assert_eq!(pool["data"]["namespace"], "payments");
        assert_eq!(pool["data"]["app"], "settlement");
        assert_eq!(pool["data"]["name"], "critical");

        let namespaces = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/namespaces").await)
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(namespaces.status().is_success());
        let body = axum::body::to_bytes(namespaces.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert!(
            json["data"]
                .as_array()
                .is_some_and(|items| items.iter().any(|item| item["name"] == "payments"))
        );

        let pools = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    "/api/v1/worker-pools?namespace=payments&app=settlement",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(pools.status().is_success());
        let body = axum::body::to_bytes(pools.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert!(
            json["data"]
                .as_array()
                .is_some_and(|items| items.iter().any(|item| item["name"] == "critical"))
        );

        let pool_id = pool["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("worker pool id should exist"));
        let quota = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "PATCH",
                    format!("/api/v1/worker-pools/{pool_id}/quota"),
                    r#"{"max_queue_depth":42,"max_concurrency":7}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("quota update should respond: {error}"));
        assert!(quota.status().is_success());

        for (action, resource_type, resource_id) in [
            ("create", "namespace", namespace["data"]["id"].as_str().unwrap_or_default()),
            ("create", "app", app_scope["data"]["id"].as_str().unwrap_or_default()),
            ("create", "worker_pool", pool_id),
            ("update", "worker_pool", pool_id),
        ] {
            let audit = app
                .clone()
                .oneshot(
                    admin_request_builder(
                        app.clone(),
                        "GET",
                        format!("/api/v1/audit-logs?action={action}&resource_type={resource_type}&resource_id={resource_id}&page_size=1"),
                    )
                    .await,
                )
                .await
                .unwrap_or_else(|error| panic!("scope audit should respond: {error}"));
            assert!(audit.status().is_success());
            let body = axum::body::to_bytes(audit.into_body(), usize::MAX)
                .await
                .unwrap_or_else(|error| panic!("audit body should collect: {error}"));
            let json: Value = serde_json::from_slice(&body)
                .unwrap_or_else(|error| panic!("audit body should be JSON: {error}"));
            assert_eq!(json["data"]["items"][0]["action"], action);
            assert_eq!(json["data"]["items"][0]["resource_type"], resource_type);
            assert_eq!(json["data"]["items"][0]["resource_id"], resource_id);
        }
    }

    #[tokio::test]
    async fn create_job_requires_bearer_token() {
        let app = router().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/jobs")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"namespace":"default","app":"billing","name":"blocked"}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 40101);
        assert!(json.get("data").is_some());
    }

    #[tokio::test]
    async fn script_publish_and_rollback_return_release_pointer_envelopes() {
        let app = router().await;
        let created = post_json(
            app.clone(),
            "/api/v1/scripts",
            r#"{"name":"wasm-release","language":"wasm","version":"1.0.0","content":"module-v1","timeout_seconds":3,"max_memory_bytes":4096,"allow_network":false}"#,
        )
        .await;
        assert_eq!(created["code"], 0);
        let script_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("script id should exist"))
            .to_owned();

        let token = admin_token(app.clone()).await;
        let update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/v1/scripts/{script_id}"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"version":"1.0.1","content":"module-v2"}"#.to_owned(),
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(update.status().is_success());

        let published = post_json(
            app.clone(),
            &format!("/api/v1/scripts/{script_id}/publish"),
            r"{}",
        )
        .await;
        assert_eq!(published["code"], 0);
        assert_eq!(published["data"]["status"], "approved");
        assert_eq!(published["data"]["released_version_number"], 2);
        assert!(published["data"]["released_version_id"].is_string());
        assert_eq!(published["data"]["policy"]["network"]["enabled"], false);

        let rolled_back = post_json(
            app.clone(),
            &format!("/api/v1/scripts/{script_id}/rollback"),
            r#"{"version_number":1}"#,
        )
        .await;
        assert_eq!(rolled_back["code"], 0);
        assert_eq!(rolled_back["data"]["status"], "approved");
        assert_eq!(rolled_back["data"]["released_version_number"], 1);
        assert_ne!(
            rolled_back["data"]["released_version_id"],
            published["data"]["released_version_id"]
        );

        let publish_audit = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    format!("/api/v1/audit-logs?action=publish&resource_type=script&resource_id={script_id}&page_size=1"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("publish audit route should respond: {error}"));
        let body = axum::body::to_bytes(publish_audit.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("publish audit body should collect: {error}"));
        let publish_audit: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("publish audit body should be JSON: {error}"));
        assert_eq!(publish_audit["data"]["items"][0]["action"], "publish");
        assert_eq!(publish_audit["data"]["items"][0]["resource_type"], "script");
        assert_eq!(publish_audit["data"]["items"][0]["resource_id"], script_id);

        let rollback_audit = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    format!("/api/v1/audit-logs?action=rollback&resource_type=script&resource_id={script_id}&page_size=1"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("rollback audit route should respond: {error}"));
        let body = axum::body::to_bytes(rollback_audit.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("rollback audit body should collect: {error}"));
        let rollback_audit: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("rollback audit body should be JSON: {error}"));
        assert_eq!(rollback_audit["data"]["items"][0]["action"], "rollback");
        assert_eq!(rollback_audit["data"]["items"][0]["resource_type"], "script");
        assert_eq!(rollback_audit["data"]["items"][0]["resource_id"], script_id);
    }

    #[tokio::test]
    async fn script_policy_rejects_dangerous_network_grant_for_now() {
        let app = router().await;
        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    "/api/v1/scripts",
                    r#"{"name":"net-script","language":"python","version":"1.0.0","content":"print(1)","policy":{"resources":{"timeout_ms":30000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":true,"allowed_hosts":["example.com"]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[]}}"#,
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
                .is_some_and(|message| message.contains("network access"))
        );
        assert!(json.get("data").is_some());
    }

    #[tokio::test]
    async fn script_release_grants_are_explicit_but_fail_closed() {
        let app = router().await;
        let created = post_json(
            app.clone(),
            "/api/v1/scripts",
            r#"{"name":"grant-release","language":"python","version":"1.0.0","content":"print(1)","timeout_seconds":3,"max_memory_bytes":4096,"allow_network":false}"#,
        )
        .await;
        let script_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("script id should exist"));

        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    format!("/api/v1/scripts/{script_id}/publish"),
                    r#"{"version_number":1,"grants":{"url":["https://api.example.com"],"file_read":["/data/input"],"file_write":["/data/output"],"secret":["secret:db-readonly"]}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("publish route should respond: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_ne!(json["code"], 0);
        assert!(json["message"].as_str().is_some_and(|message| {
            message.contains("signature verification is not yet enabled")
        }));
        assert!(json.get("data").is_some());
    }

    #[tokio::test]
    async fn script_release_persists_locally_verified_grants_when_signed() {
        let app = router_with_script_signature_secret_ref("env:PATH").await;
        let created = post_json(
            app.clone(),
            "/api/v1/scripts",
            r#"{"name":"signed-grant-release","language":"python","version":"1.0.0","content":"print(1)","timeout_seconds":3,"max_memory_bytes":4096,"allow_network":false}"#,
        )
        .await;
        let script_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("script id should exist"));
        let content_sha256 = created["data"]["content_sha256"]
            .as_str()
            .unwrap_or_else(|| panic!("content digest should exist"));
        let approval_ticket = "CAB-2026-GRANTS";
        let grants_json = r#"{"url":["https://api.example.com"],"file_read":["/data/input"],"file_write":[],"secret":["secret:db-readonly"]}"#;
        let signature = local_script_release_signature(
            &std::env::var("PATH").unwrap_or_else(|error| panic!("PATH should exist: {error}")),
            script_id,
            1,
            content_sha256,
            approval_ticket,
            Some(grants_json),
        );

        let published = post_json(
            app,
            &format!("/api/v1/scripts/{script_id}/publish"),
            &format!(
                r#"{{"version_number":1,"approval_ticket":"{approval_ticket}","signature":"{signature}","grants":{grants_json}}}"#
            ),
        )
        .await;
        assert_eq!(published["code"], 0);
        assert_eq!(published["data"]["release_grants"]["url"][0], "https://api.example.com");
        assert_eq!(published["data"]["release_grants"]["secret"][0], "secret:db-readonly");
        assert_eq!(published["data"]["release_grants"]["verified_by"], "bootstrap_admin");
        assert_eq!(published["data"]["release_signature"]["approval_ticket"], approval_ticket);
    }

    #[tokio::test]
    async fn script_release_rejects_unverified_approval_or_signature_metadata() {
        let app = router().await;
        let created = post_json(
            app.clone(),
            "/api/v1/scripts",
            r#"{"name":"signed-release","language":"python","version":"1.0.0","content":"print(1)","timeout_seconds":3,"max_memory_bytes":4096,"allow_network":false}"#,
        )
        .await;
        let script_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("script id should exist"));

        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    format!("/api/v1/scripts/{script_id}/publish"),
                    r#"{"version_number":1,"approval_ticket":"CAB-42","signature":"unsigned-local"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("publish route should respond: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_ne!(json["code"], 0);
        assert!(
            json["message"].as_str().is_some_and(
                |message| message.contains("signature verification is not yet enabled")
            )
        );

        let audit = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    "/api/v1/audit-logs?failure_reason=script_signature_verification_required",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("audit route should respond: {error}"));
        let body = axum::body::to_bytes(audit.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["items"].as_array().map(Vec::len), Some(1));
    }

    #[tokio::test]
    async fn script_release_gate_preview_reports_safe_version() {
        let app = router().await;
        let created = post_json(
            app.clone(),
            "/api/v1/scripts",
            r#"{"name":"gate-safe","language":"python","version":"1.0.0","content":"print(1)","timeout_seconds":3,"max_memory_bytes":4096,"allow_network":false}"#,
        )
        .await;
        let script_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("script id should exist"));

        let response = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    format!("/api/v1/scripts/{script_id}/release-gate?version_number=1"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("release gate route should respond: {error}"));
        assert!(response.status().is_success());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["version_number"], 1);
        assert_eq!(json["data"]["releasable"], true);
        assert_eq!(
            json["data"]["blocking_reasons"].as_array().map(Vec::len),
            Some(0)
        );
        assert_eq!(json["data"]["signature_verification_enabled"], false);
    }


    #[tokio::test]
    async fn script_release_accepts_locally_verified_signature_when_configured() {
        let app = router_with_script_signature_secret_ref("env:PATH").await;
        let created = post_json(
            app.clone(),
            "/api/v1/scripts",
            r#"{"name":"signed-local-release","language":"python","version":"1.0.0","content":"print(1)","timeout_seconds":3,"max_memory_bytes":4096,"allow_network":false}"#,
        )
        .await;
        let script_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("script id should exist"));
        let content_sha256 = created["data"]["content_sha256"]
            .as_str()
            .unwrap_or_else(|| panic!("content digest should exist"));
        let approval_ticket = "CAB-2026-LOCAL";
        let signature = local_script_release_signature(
            &std::env::var("PATH").unwrap_or_else(|error| panic!("PATH should exist: {error}")),
            script_id,
            1,
            content_sha256,
            approval_ticket,
            None,
        );

        let gate = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    format!("/api/v1/scripts/{script_id}/release-gate?version_number=1"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("release gate route should respond: {error}"));
        let body = axum::body::to_bytes(gate.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["signature_verification_enabled"], true);

        let published = post_json(
            app.clone(),
            &format!("/api/v1/scripts/{script_id}/publish"),
            &format!(
                r#"{{"version_number":1,"approval_ticket":"{approval_ticket}","signature":"{signature}"}}"#
            ),
        )
        .await;
        assert_eq!(published["code"], 0);
        assert_eq!(published["data"]["released_version_number"], 1);
        assert_eq!(
            published["data"]["release_signature"]["approval_ticket"],
            approval_ticket
        );
        assert_eq!(published["data"]["release_signature"]["signature"], signature);
        assert_eq!(published["data"]["release_signature"]["verified_by"], "bootstrap_admin");
        assert!(published["data"]["release_signature"]["verified_at"].is_string());

        let reloaded_response = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    format!("/api/v1/scripts/{script_id}"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("get script route should respond: {error}"));
        assert!(reloaded_response.status().is_success());
        let reloaded_body = axum::body::to_bytes(reloaded_response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let reloaded: Value = serde_json::from_slice(&reloaded_body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(
            reloaded["data"]["release_signature"]["approval_ticket"],
            approval_ticket
        );
    }

    #[tokio::test]
    async fn script_release_rejects_wrong_local_signature_when_configured() {
        let app = router_with_script_signature_secret_ref("env:PATH").await;
        let created = post_json(
            app.clone(),
            "/api/v1/scripts",
            r#"{"name":"bad-local-signature","language":"python","version":"1.0.0","content":"print(1)","timeout_seconds":3,"max_memory_bytes":4096,"allow_network":false}"#,
        )
        .await;
        let script_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("script id should exist"));

        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    format!("/api/v1/scripts/{script_id}/publish"),
                    r#"{"version_number":1,"approval_ticket":"CAB-2026-BAD","signature":"sha256:bad"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("publish route should respond: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(json["message"]
            .as_str()
            .is_some_and(|message| message.contains("signature verification failed")));
    }

    fn local_script_release_signature(
        secret: &str,
        script_id: &str,
        version_number: i64,
        content_sha256: &str,
        approval_ticket: &str,
        grants: Option<&str>,
    ) -> String {
        use sha2::{Digest as _, Sha256};

        let grants = grants.unwrap_or(r#"{"url":[],"file_read":[],"file_write":[],"secret":[]}"#);
        let payload = format!(
            "tikeo-script-release-v1\nscript_id={script_id}\nversion_number={version_number}\ncontent_sha256={content_sha256}\napproval_ticket={approval_ticket}\ngrants={grants}"
        );
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hasher.update(b"\n");
        hasher.update(payload.as_bytes());
        format!("sha256:{}", hex::encode(hasher.finalize()))
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn script_publish_blocks_legacy_dangerous_policy_snapshot() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let scripts = ScriptRepository::new(db.clone());
        let script = scripts
            .create_script(tikeo_storage::CreateScript {
                name: "legacy-dangerous".to_owned(),
                language: "python".to_owned(),
                version: "1.0.0".to_owned(),
                content: "print(1)".to_owned(),
                created_by: "legacy-import".to_owned(),
                timeout_seconds: Some(30),
                max_memory_bytes: Some(64 * 1024 * 1024),
                allow_network: true,
                allowed_env_vars: None,
                policy_json: Some(r#"{"resources":{"timeout_ms":30000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":true,"allowed_hosts":["example.com"]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[]}"#.to_owned()),
            })
            .await
            .unwrap_or_else(|error| panic!("legacy script should create: {error}"));
        scripts
            .update_script(
                &script.id,
                tikeo_storage::UpdateScript {
                    name: None,
                    language: None,
                    version: Some("1.0.1".to_owned()),
                    content: Some("print(2)".to_owned()),
                    status: None,
                    timeout_seconds: None,
                    max_memory_bytes: None,
                    allow_network: Some(false),
                    allowed_env_vars: None,
                    policy_json: Some(r#"{"resources":{"timeout_ms":30000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[]}"#.to_owned()),
                },
            )
            .await
            .unwrap_or_else(|error| panic!("safe script version should create: {error}"));
        let app = router_with_state(AppState::new(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            scripts,
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));

        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    format!("/api/v1/scripts/{}/publish", script.id),
                    r#"{"version_number":1}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("publish route should respond: {error}"));
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
                .is_some_and(|message| message.contains("approval gate"))
        );

        let published_safe = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    format!("/api/v1/scripts/{}/publish", script.id),
                    r#"{"version_number":2}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("safe publish route should respond: {error}"));
        assert!(published_safe.status().is_success());

        let blocked_rollback = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    format!("/api/v1/scripts/{}/rollback", script.id),
                    r#"{"version_number":1}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("rollback route should respond: {error}"));
        assert_eq!(
            blocked_rollback.status(),
            axum::http::StatusCode::BAD_REQUEST
        );

        let audit = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    "/api/v1/audit-logs?failure_reason=script_policy_approval_required",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("audit route should respond: {error}"));
        let body = axum::body::to_bytes(audit.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["items"].as_array().map(Vec::len), Some(2));

        let gate = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    format!("/api/v1/scripts/{}/release-gate?version_number=1", script.id),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("release gate route should respond: {error}"));
        assert!(gate.status().is_success());
        let body = axum::body::to_bytes(gate.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["releasable"], false);
        assert!(json["data"]["blocking_reasons"][0]
            .as_str()
            .is_some_and(|reason| reason.contains("approval gate")));
        assert!(json["data"]["required_actions"]
            .as_array()
            .is_some_and(|actions| !actions.is_empty()));
    }

    #[tokio::test]
    async fn create_job_persists_and_list_jobs_returns_it() {
        let app = router().await;
        let created = post_json(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"nightly"}"#,
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(created["data"]["name"], "nightly");
        assert_eq!(created["data"]["namespace"], "default");
        assert_eq!(created["data"]["app"], "billing");

        let list = request_with(app, "/api/v1/jobs").await;
        let body = axum::body::to_bytes(list.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["items"][0]["name"], "nightly");
        assert!(json.get("data").is_some());
    }

    #[tokio::test]
    async fn broadcast_trigger_creates_worker_attempts() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let registry = crate::tunnel::WorkerRegistry::with_lifecycle(
            tikeo_storage::WorkerLifecycleRepository::new(db.clone()),
        );
        let (tx1, _rx1) = tokio::sync::mpsc::channel(1);
        let (tx2, _rx2) = tokio::sync::mpsc::channel(1);
        registry.register(worker("worker-a", "billing"), tx1).await;
        registry.register(worker("worker-b", "billing"), tx2).await;
        let app = router_with_state(AppState::new(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            registry,
            StandaloneCoordinator::shared("test-node"),
        ));

        let created = post_json(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"broadcast"}"#,
        )
        .await;
        let job_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("created job should contain id"));

        let triggered = post_json(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}:trigger"),
            r#"{"triggerType":"api","executionMode":"broadcast"}"#,
        )
        .await;
        let instance_id = triggered["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("triggered instance should contain id"));
        assert_eq!(triggered["data"]["executionMode"], "broadcast");

        let attempts =
            request_with(app, &format!("/api/v1/instances/{instance_id}/attempts")).await;
        let body = axum::body::to_bytes(attempts.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["items"].as_array().map(Vec::len), Some(2));
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn trigger_job_creates_pending_instance() {
        let app = router().await;
        let created = post_json(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"manual"}"#,
        )
        .await;
        let job_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("created job should contain id"));

        let triggered = post_json(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}:trigger"),
            r#"{"triggerType":"api"}"#,
        )
        .await;

        assert_eq!(triggered["code"], 0);
        assert_eq!(triggered["data"]["jobId"], job_id);
        assert_eq!(triggered["data"]["status"], "pending");

        let listed = request_with(app.clone(), &format!("/api/v1/jobs/{job_id}/instances")).await;
        let body = axum::body::to_bytes(listed.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["data"]["items"][0]["status"], "pending");

        let instance_id = triggered["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("triggered instance should contain id"));
        let detail = request_with(app, &format!("/api/v1/instances/{instance_id}")).await;
        let body = axum::body::to_bytes(detail.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["id"], instance_id);

        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let app = router_with_state(AppState::new(
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
        let created = post_json(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"with-log"}"#,
        )
        .await;
        let job_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("job id"));
        let triggered = post_json(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}:trigger"),
            r#"{"triggerType":"api"}"#,
        )
        .await;
        let instance_id = triggered["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("instance id"));
        let log_repo = JobInstanceLogRepository::new(db);
        log_repo
            .append(AppendJobInstanceLog {
                instance_id: instance_id.to_owned(),
                worker_id: "worker-1".to_owned(),
                level: "info".to_owned(),
                message: "hello".to_owned(),
                sequence: 1,
            })
            .await
            .unwrap_or_else(|error| panic!("log should append: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        log_repo
            .append(AppendJobInstanceLog {
                instance_id: instance_id.to_owned(),
                worker_id: "tikeo-dispatcher".to_owned(),
                level: "warn".to_owned(),
                message: serde_json::json!({
                    "event": "script_execution_governance",
                    "failure_class": "script_runtime_unavailable",
                    "message": "runtime missing",
                })
                .to_string(),
                sequence: 2,
            })
            .await
            .unwrap_or_else(|error| panic!("governance log should append: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        let logs = request_with(
            app.clone(),
            &format!("/api/v1/instances/{instance_id}/logs"),
        )
        .await;
        let body = axum::body::to_bytes(logs.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["items"][0]["message"], "hello");
        assert_eq!(
            json["data"]["items"][1]["governanceEvent"],
            "script_execution_governance"
        );
        assert_eq!(
            json["data"]["items"][1]["governanceFailureClass"],
            "script_runtime_unavailable"
        );
        assert_eq!(json["data"]["items"][1]["message"], "runtime missing");

        let filtered = request_with(
            app,
            &format!("/api/v1/instances/{instance_id}/logs?page_token=script_execution_governance"),
        )
        .await;
        let body = axum::body::to_bytes(filtered.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            json["data"]["items"][0]["governanceFailureClass"],
            "script_runtime_unavailable"
        );
    }

    #[tokio::test]
    async fn create_job_accepts_processor_binding() {
        let app = router().await;
        let json = post_json(
            app,
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"invoice-sync","scheduleType":"api","processorName":"billing.invoice-sync"}"#,
        )
        .await;

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["processorName"], "billing.invoice-sync");
    }

    #[tokio::test]
    async fn job_binding_switches_between_script_and_sdk_structurally() {
        let app = router().await;
        let created = post_json(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"script-job","scheduleType":"api","scriptId":"script-safe"}"#,
        )
        .await;
        let job_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("created job should contain id"));
        assert_eq!(created["data"]["scriptId"], "script-safe");
        assert_eq!(created["data"]["processorName"], Value::Null);

        let update = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "PATCH",
                    format!("/api/v1/jobs/{job_id}"),
                    r#"{"processorName":"demo.echo","scriptId":null}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(update.status().is_success());
        let body = axum::body::to_bytes(update.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["processorName"], "demo.echo");
        assert_eq!(json["data"]["scriptId"], Value::Null);
    }


    #[tokio::test]
    async fn job_management_updates_disables_and_deletes_jobs() {
        let app = router().await;
        let created = post_json(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"manage-me","scheduleType":"api","processorName":"demo.echo"}"#,
        )
        .await;
        let job_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("created job should contain id"));
        assert_eq!(created["data"]["scheduleType"], "api");

        let update = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "PATCH",
                    format!("/api/v1/jobs/{job_id}"),
                    r#"{"name":"managed","enabled":false,"scheduleType":"api","processorName":"demo.report"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(update.status().is_success());
        let body = axum::body::to_bytes(update.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["name"], "managed");
        assert_eq!(json["data"]["enabled"], false);
        assert_eq!(json["data"]["processorName"], "demo.report");

        let delete = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "DELETE", format!("/api/v1/jobs/{job_id}")).await)
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(delete.status().is_success());

        let list = request_with(app, "/api/v1/jobs").await;
        let body = axum::body::to_bytes(list.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let items = json["data"]["items"]
            .as_array()
            .unwrap_or_else(|| panic!("items should be an array"));
        assert!(items.iter().all(|item| item["id"] != job_id));
    }

    #[tokio::test]
    async fn job_management_update_can_move_namespace_and_app() {
        let app = router().await;
        let created = post_json(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"move-me","scheduleType":"api","processorName":"demo.echo"}"#,
        )
        .await;
        post_json(
            app.clone(),
            "/api/v1/namespaces",
            r#"{"name":"ops"}"#,
        )
        .await;
        post_json(
            app.clone(),
            "/api/v1/apps",
            r#"{"namespace":"ops","name":"control"}"#,
        )
        .await;
        let job_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("created job should contain id"));

        let update = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "PATCH",
                    format!("/api/v1/jobs/{job_id}"),
                    r#"{"namespace":"ops","app":"control","name":"moved"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(update.status().is_success());
        let body = axum::body::to_bytes(update.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let updated: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(updated["code"], 0);
        assert_eq!(updated["data"]["namespace"], "ops");
        assert_eq!(updated["data"]["app"], "control");
        assert_eq!(updated["data"]["name"], "moved");
    }

    async fn post_json(app: axum::Router, uri: &str, body: &str) -> Value {
        post_json_with_auth(app, uri, body, true).await
    }

    async fn post_json_without_auth(app: axum::Router, uri: &str, body: &str) -> Value {
        post_json_raw(app, uri, body, None).await
    }

    async fn post_json_with_auth(app: axum::Router, uri: &str, body: &str, auth: bool) -> Value {
        let token = if auth {
            Some(admin_token(app.clone()).await)
        } else {
            None
        };
        post_json_raw(app, uri, body, token.as_deref()).await
    }

    async fn post_json_raw(app: axum::Router, uri: &str, body: &str, token: Option<&str>) -> Value {
        let mut builder = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        let response = app
            .oneshot(
                builder
                    .body(Body::from(body.to_owned()))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(response.status().is_success());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        serde_json::from_slice(&body).unwrap_or_else(|error| panic!("body should be JSON: {error}"))
    }

    async fn get_json(uri: &str) -> Value {
        let response = request(uri).await;
        assert!(response.status().is_success());

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));

        serde_json::from_slice(&body).unwrap_or_else(|error| panic!("body should be JSON: {error}"))
    }

    async fn request(uri: &str) -> axum::response::Response {
        request_with(router().await, uri).await
    }

    async fn request_with(app: axum::Router, uri: &str) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .unwrap_or_else(|error| panic!("request should build: {error}")),
        )
        .await
        .unwrap_or_else(|error| panic!("router should respond: {error}"))
    }

    async fn admin_token(app: axum::Router) -> String {
        ensure_bootstrap_admin(app.clone()).await;
        let login = post_json_raw(app, "/api/v1/auth/login", ADMIN_LOGIN, None).await;
        login["data"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("admin login should return token"))
            .to_owned()
    }


    async fn ensure_bootstrap_admin(app: axum::Router) {
        let status = request_with(app.clone(), "/api/v1/auth/bootstrap").await;
        let status_body = axum::body::to_bytes(status.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("bootstrap status body should collect: {error}"));
        let status_json: Value = serde_json::from_slice(&status_body)
            .unwrap_or_else(|error| panic!("bootstrap status body should be JSON: {error}"));
        if status_json["data"]["registrationOpen"].as_bool() != Some(true) {
            return;
        }
        let payload = r#"{"username":"bootstrap_admin","email":"bootstrap.admin@example.com","password":"TestOnlyOwnerPassword!2026","confirmPassword":"TestOnlyOwnerPassword!2026"}"#;
        let created = post_json_raw(app, "/api/v1/auth/bootstrap/register", payload, None).await;
        assert_eq!(created["data"]["roles"][0], "owner");
    }

    async fn admin_request_builder(
        app: axum::Router,
        method: &str,
        uri: impl ToString,
    ) -> Request<Body> {
        let token = admin_token(app).await;
        Request::builder()
            .method(method)
            .uri(uri.to_string())
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap_or_else(|error| panic!("request should build: {error}"))
    }

    async fn admin_json_request_builder(
        app: axum::Router,
        method: &str,
        uri: impl ToString,
        body: &str,
    ) -> Request<Body> {
        let token = admin_token(app).await;
        Request::builder()
            .method(method)
            .uri(uri.to_string())
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap_or_else(|error| panic!("request should build: {error}"))
    }

    fn worker(client_instance_id: &str, app: &str) -> RegisterWorker {
        RegisterWorker {
            client_instance_id: client_instance_id.to_owned(),
            app: app.to_owned(),
            namespace: "default".to_owned(),
            cluster: "local".to_owned(),
            region: "local".to_owned(),
            capabilities: Vec::new(),
            structured_capabilities: None,
            election: None,
            labels: std::collections::HashMap::default(),
        }
    }
