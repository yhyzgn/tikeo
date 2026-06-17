    #[tokio::test]
    async fn script_governance_audit_logs_filter_by_failure_reason() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "governed-script".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: Some("script-missing-runtime".to_owned()),
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("parent job should exist"));

        crate::tunnel::governance::materialize_script_governance_audit(
            &audit,
            "tikeo-dispatcher",
            &instance.id,
            "script_runtime_unavailable",
            "runtime missing",
        )
        .await
        .unwrap_or_else(|error| panic!("governance audit should append: {error}"));

        let app = router_with_state(AppState::new(
            jobs,
            instances,
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
                    app,
                    "GET",
                    "/api/v1/audit-logs?resource_type=script_execution_governance&failure_reason=script_runtime_unavailable",
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
        assert_eq!(
            json["data"]["items"][0]["action"],
            "script_governance_failure"
        );
        assert_eq!(
            json["data"]["items"][0]["resource_type"],
            "script_execution_governance"
        );
        assert_eq!(json["data"]["items"][0]["resource_id"], instance.id);
        assert_eq!(
            json["data"]["items"][0]["failure_reason"],
            "script_runtime_unavailable"
        );
        assert_eq!(json["data"]["items"][0]["result"], "failed");
    }

    #[tokio::test]
    async fn login_succeeds_and_me_returns_principal() {
        let app = router().await;
        ensure_bootstrap_admin(app.clone()).await;
        let login = post_json_without_auth(
            app.clone(),
            "/api/v1/auth/login",
            ADMIN_LOGIN,
        )
        .await;

        assert_eq!(login["code"], 0);
        let token = login["data"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("login should return token"))
            .to_owned();
        assert_eq!(token.len(), 48);
        assert!(token.chars().all(|value| value.is_ascii_alphanumeric()));
        assert_eq!(login["data"]["roles"][0], "owner");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let me: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(me["code"], 0);
        assert_eq!(me["data"]["username"], "bootstrap_admin");
    }




    #[tokio::test]
    async fn cancel_instance_route_records_audit_log() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let created = post_json_raw(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"cancel-audit","scheduleType":"api","processorName":"demo.echo"}"#,
            Some(&admin),
        )
        .await;
        let job_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("job id should be present"));
        let triggered = post_json_raw(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}:trigger"),
            r#"{"triggerType":"api","executionMode":"single"}"#,
            Some(&admin),
        )
        .await;
        let instance_id = triggered["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("instance id should be present"));
        let cancelled = post_json_raw(
            app.clone(),
            &format!("/api/v1/instances/{instance_id}/cancel"),
            "{}",
            Some(&admin),
        )
        .await;
        assert_eq!(cancelled["data"]["status"], "cancelled");

        let audit = get_json_with_auth(
            app,
            "/api/v1/audit-logs?action=cancel&resource_type=job_instance&page_size=1",
            &admin,
        )
        .await;
        assert_eq!(audit["data"]["items"][0]["action"], "cancel");
        assert_eq!(audit["data"]["items"][0]["resource_type"], "job_instance");
        assert_eq!(audit["data"]["items"][0]["resource_id"], instance_id);
    }

    #[tokio::test]
    async fn job_version_api_lists_and_rolls_back_snapshots() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let created = post_json_raw(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"versioned-job","scheduleType":"api","processorName":"demo.echo"}"#,
            Some(&admin),
        )
        .await;
        let job_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("job id should be present"))
            .to_owned();
        assert_eq!(created["data"]["versionNumber"], 1);

        let updated = patch_json_raw(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}"),
            r#"{"name":"versioned-job-v2","enabled":false}"#,
            &admin,
        )
        .await;
        assert_eq!(updated["data"]["versionNumber"], 2);
        assert_eq!(updated["data"]["enabled"], false);

        let versions = get_json_with_auth(app.clone(), &format!("/api/v1/jobs/{job_id}/versions"), &admin).await;
        let items = versions["data"]["items"]
            .as_array()
            .unwrap_or_else(|| panic!("versions should be an array"));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["version_number"], 2);
        assert_eq!(items[1]["version_number"], 1);

        let rolled_back = post_json_raw(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}/rollback"),
            r#"{"versionNumber":1}"#,
            Some(&admin),
        )
        .await;
        assert_eq!(rolled_back["data"]["versionNumber"], 3);
        assert_eq!(rolled_back["data"]["name"], "versioned-job");
        assert_eq!(rolled_back["data"]["enabled"], true);

        let audit = get_json_with_auth(
            app,
            "/api/v1/audit-logs?action=rollback&resource_type=job&page_size=1",
            &admin,
        )
        .await;
        assert_eq!(audit["data"]["items"][0]["action"], "rollback");
        assert_eq!(audit["data"]["items"][0]["resource_type"], "job");
        assert_eq!(audit["data"]["items"][0]["resource_id"], job_id);
    }

    #[tokio::test]
    async fn plugin_registry_supports_custom_processor_types_and_alert_channels() {
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

        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/plugins",
                    r#"{"name":"Ops Plugin","kind":"processor","processorTypes":[{"type":"sql","label":"SQL Processor","capability":"sql","processorNames":["billing.sql-sync"],"description":"Runs governed SQL processor tasks"},{"type":"external_jar","label":"External JAR Processor","capability":"external_jar","processorNames":["billing.jar-sync"],"description":"Runs versioned JAR in container sandbox","artifactRef":"s3://plugins/billing-jar-sync-1.0.0.jar","containerImage":"registry.example.com/tikeo/jar-runner:1.0.0","entrypoint":["java","-jar","/plugins/billing-jar-sync.jar"],"checksum":"sha256:abc123"}],"alertChannelTypes":[{"type":"ops_webhook","label":"Ops Webhook","targetKind":"webhook","description":"Routes alerts to the ops bridge","template":{"headers":{"X-Tikeo-Plugin":"ops"},"body":{"text":"{{message}}","resource":"{{resource_id}}"}}}],"enabled":true}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("plugin route should respond: {error}"));
        assert!(created.status().is_success());
        let body = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["processorTypes"][0]["capability"], "sql");
        assert_eq!(json["data"]["processorTypes"][0]["processorNames"][0], "billing.sql-sync");
        assert_eq!(json["data"]["processorTypes"][1]["type"], "external_jar");
        assert_eq!(json["data"]["processorTypes"][1]["artifactRef"], "s3://plugins/billing-jar-sync-1.0.0.jar");
        assert_eq!(json["data"]["processorTypes"][1]["containerImage"], "registry.example.com/tikeo/jar-runner:1.0.0");
        assert_eq!(json["data"]["alertChannelTypes"][0]["type"], "ops_webhook");

        let plugins = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/plugins").await)
            .await
            .unwrap_or_else(|error| panic!("plugin list should respond: {error}"));
        assert!(plugins.status().is_success());
        let body = axum::body::to_bytes(plugins.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"].as_array().map(Vec::len), Some(1));

        let job = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/jobs",
                    r#"{"namespace":"default","app":"billing","name":"sql-sync","scheduleType":"api","processorType":"sql","processorName":"billing.sql-sync"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("job create should respond: {error}"));
        assert!(job.status().is_success());
        let body = axum::body::to_bytes(job.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["processorType"], "sql");

        let jar_job = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/jobs",
                    r#"{"namespace":"default","app":"billing","name":"jar-sync","scheduleType":"api","processorType":"external_jar","processorName":"billing.jar-sync"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("external jar job create should respond: {error}"));
        assert!(jar_job.status().is_success());

        let invalid_job = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/jobs",
                    r#"{"namespace":"default","app":"billing","name":"bad-sql-sync","scheduleType":"api","processorType":"sql","processorName":"mixed.sql"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("invalid job create should respond: {error}"));
        assert_eq!(invalid_job.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(invalid_job.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let invalid_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(invalid_json["message"]
            .as_str()
            .is_some_and(|message| message.contains("plugin processorName is not declared")));

        let advice = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    format!(
                        "/api/v1/jobs/{}/scheduling-advice",
                        json["data"]["id"].as_str().unwrap_or_else(|| panic!("job id"))
                    ),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("advice route should respond: {error}"));
        assert!(advice.status().is_success());
        let body = axum::body::to_bytes(advice.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(
            json["data"]["requiredCapability"],
            "plugin processor type 'sql' name 'billing.sql-sync'"
        );

        let status = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/alert-rules",
                    r#"{"name":"Ops plugin alert","severity":"warning","condition":{"type":"script_governance_failure","failure_class":"plugin_test","threshold":1},"channels":[{"type":"ops_webhook","url":"https://ops.example.invalid/hook","enabled":true}],"enabled":true,"dedupe_seconds":30}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert rule route should respond: {error}"));
        assert!(status.status().is_success());
        let body = axum::body::to_bytes(status.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let rule_id = json["data"]["id"].as_str().unwrap_or_else(|| panic!("rule id"));

        let readiness = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    format!("/api/v1/alert-rules/{rule_id}/delivery-status"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("alert readiness route should respond: {error}"));
        assert!(readiness.status().is_success());
        let body = axum::body::to_bytes(readiness.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["ready"], true);
        assert_eq!(json["data"]["channels"][0]["provider"], "ops_webhook");
        assert_eq!(json["data"]["channels"][0]["transport_security"], "https");
    }

    #[tokio::test]
    async fn sdk_api_key_lifecycle_uses_header_and_app_scope() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let (api_key, key_id) = create_billing_sdk_api_key(app.clone(), &admin).await;

        assert_sdk_api_key_list_redacted(app.clone(), &admin, &api_key, &key_id).await;
        assert_sdk_key_audit_action(app.clone(), &admin, &key_id, "sdk_api_key_create").await;
        seed_sdk_key_scope_jobs(app.clone(), &admin).await;
        assert_sdk_key_lists_only_bound_app(app.clone(), &api_key).await;
        assert_sdk_key_authentication_is_audited(app.clone(), &admin, &key_id).await;
        assert_sdk_key_cannot_write_other_app(app.clone(), &api_key).await;
        update_sdk_api_key(app.clone(), &admin, &key_id).await;
        assert_sdk_key_audit_action(app.clone(), &admin, &key_id, "sdk_api_key_update").await;
        assert_sdk_key_lists_only_bound_app(app.clone(), &api_key).await;
        assert_sdk_key_cannot_write_bound_app_after_scope_edit(app.clone(), &api_key).await;
        assert_sdk_api_key_list_still_contains_updated_key(app.clone(), &admin, &key_id).await;
        revoke_sdk_api_key(app.clone(), &admin, &key_id).await;
        assert_sdk_key_audit_action(app.clone(), &admin, &key_id, "sdk_api_key_revoke").await;
        assert_revoked_sdk_key_rejected(app, &api_key).await;
    }

    #[tokio::test]
    async fn disabling_service_account_revokes_bound_sdk_keys() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let (api_key, key_id) = create_billing_sdk_api_key(app.clone(), &admin).await;
        let list = get_json_with_auth(app.clone(), "/api/v1/management/api-keys", &admin).await;
        let service_account_id = list["data"]
            .as_array()
            .unwrap_or_else(|| panic!("api key list should be an array"))
            .iter()
            .find(|item| item["id"] == key_id)
            .and_then(|item| item["service_account_id"].as_str())
            .unwrap_or_else(|| panic!("bound service account should be listed"))
            .to_owned();

        let disabled = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/management/service-accounts/{service_account_id}"))
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(disabled.status().is_success());

        let service_accounts = get_json_with_auth(app.clone(), "/api/v1/management/service-accounts", &admin).await;
        let disabled_account = service_accounts["data"]
            .as_array()
            .unwrap_or_else(|| panic!("service account list should be an array"))
            .iter()
            .find(|item| item["id"] == service_account_id)
            .unwrap_or_else(|| panic!("disabled service account should remain visible"));
        assert_eq!(disabled_account["status"], "disabled");

        let keys = get_json_with_auth(app.clone(), "/api/v1/management/api-keys", &admin).await;
        assert!(keys["data"]
            .as_array()
            .unwrap_or_else(|| panic!("api key list should be an array"))
            .iter()
            .all(|item| item["id"] != key_id));
        assert_revoked_sdk_key_rejected(app, &api_key).await;
    }


    async fn patch_json_raw(app: axum::Router, uri: &str, body: &str, token: &str) -> Value {
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(uri)
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_owned()))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body).unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(status.is_success(), "unexpected status {status}: {json}");
        json
    }

    async fn get_json_with_auth(app: axum::Router, uri: &str, token: &str) -> Value {
        let response = app
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body).unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(status.is_success(), "unexpected status {status}: {json}");
        json
    }

    async fn create_billing_sdk_api_key(app: axum::Router, admin: &str) -> (String, String) {
        let service_account = create_billing_service_account(app.clone(), admin).await;
        let service_account_id = service_account["id"]
            .as_str()
            .unwrap_or_else(|| panic!("service account id should be present"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/management/api-keys")
                    .header("authorization", format!("Bearer {admin}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"name":"java demo","namespace":"default","app":"billing","service_account_id":"{service_account_id}","scopes":["jobs:read","jobs:write","instances:execute"]}}"#
                    )))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let created: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(&body)}));
        assert!(status.is_success(), "unexpected status {status}: {created}");
        assert_eq!(created["code"], 0);
        let api_key = created["data"]["api_key"]
            .as_str()
            .unwrap_or_else(|| panic!("api key should be returned once"))
            .to_owned();
        assert!(api_key.starts_with("tk-"));
        assert_eq!(api_key.len(), 67);
        assert!(api_key[3..].chars().all(|ch| ch.is_ascii_alphanumeric()));
        assert_eq!(created["data"]["key"]["namespace"], "default");
        assert_eq!(created["data"]["key"]["app"], "billing");
        assert_eq!(created["data"]["key"]["service_account_name"], "java-demo-sa");
        assert_eq!(created["data"]["key"]["service_account_id"], service_account_id);
        let key_id = created["data"]["key"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("key id should be present"))
            .to_owned();
        (api_key, key_id)
    }

    async fn create_billing_service_account(app: axum::Router, admin: &str) -> Value {
        let created = post_json_raw(
            app.clone(),
            "/api/v1/management/service-accounts",
            r#"{"name":"java-demo-sa","description":"Java demo SDK identity","namespace":"default","app":"billing"}"#,
            Some(admin),
        )
        .await;
        assert_eq!(created["data"]["name"], "java-demo-sa");
        assert_eq!(created["data"]["namespace"], "default");
        assert_eq!(created["data"]["app"], "billing");
        let list = get_json_with_auth(app, "/api/v1/management/service-accounts", admin).await;
        assert!(list["data"]
            .as_array()
            .unwrap_or_else(|| panic!("service account list should be an array"))
            .iter()
            .any(|item| item["id"] == created["data"]["id"]));
        created["data"].clone()
    }

    async fn assert_sdk_api_key_list_redacted(app: axum::Router, admin: &str, api_key: &str, key_id: &str) {
        let list = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/management/api-keys")
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(list.status().is_success());
        let list_body = axum::body::to_bytes(list.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let list_text = String::from_utf8(list_body.to_vec())
            .unwrap_or_else(|error| panic!("body should be utf8: {error}"));
        assert!(!list_text.contains(api_key));
        assert!(!list_text.contains("key_hash"));
        let list_json: Value = serde_json::from_str(&list_text)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(list_json["data"][0]["id"], key_id);
        assert_eq!(list_json["data"][0]["service_account_name"], "java-demo-sa");
    }

    async fn seed_sdk_key_scope_jobs(app: axum::Router, admin: &str) {
        let _billing = post_json_raw(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"sdk-visible"}"#,
            Some(admin),
        )
        .await;
        let _other = post_json_raw(
            app,
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"other","name":"sdk-hidden"}"#,
            Some(admin),
        )
        .await;
    }

    async fn assert_sdk_key_lists_only_bound_app(app: axum::Router, api_key: &str) {
        let visible = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/jobs")
                    .header("x-tikeo-api-key", api_key)
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(visible.status().is_success());
        let visible_body = axum::body::to_bytes(visible.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let visible_json: Value = serde_json::from_slice(&visible_body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let visible_items = visible_json["data"]["items"]
            .as_array()
            .unwrap_or_else(|| panic!("items should be an array"));
        assert!(visible_items.iter().any(|item| item["name"] == "sdk-visible"));
        assert!(visible_items.iter().all(|item| item["name"] != "sdk-hidden"));
    }



    async fn assert_sdk_key_audit_action(
        app: axum::Router,
        admin: &str,
        key_id: &str,
        action: &str,
    ) {
        let audit = get_json_with_auth(
            app,
            &format!("/api/v1/audit-logs?action={action}&resource_type=sdk_api_key&resource_id={key_id}&page_size=1"),
            admin,
        )
        .await;
        assert_eq!(audit["data"]["items"][0]["resource_type"], "sdk_api_key");
        assert_eq!(audit["data"]["items"][0]["resource_id"], key_id);
        assert!(!audit["data"]["items"][0]["actor"]
            .as_str()
            .unwrap_or_default()
            .is_empty());
    }

    async fn assert_sdk_key_authentication_is_audited(app: axum::Router, admin: &str, key_id: &str) {
        let audit = get_json_with_auth(
            app,
            &format!("/api/v1/audit-logs?action=sdk_api_key_authenticate&resource_type=sdk_api_key&resource_id={key_id}&pageSize=20"),
            admin,
        )
        .await;
        assert_eq!(audit["data"]["items"][0]["resource_type"], "sdk_api_key");
        assert_eq!(audit["data"]["items"][0]["resource_id"], key_id);
        assert!(
            audit["data"]["items"]
                .as_array()
                .unwrap_or_else(|| panic!("audit items should be an array"))
                .iter()
                .any(|item| item["actor"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("service_account:sa_"))
        );
    }

    async fn assert_sdk_key_cannot_write_other_app(app: axum::Router, api_key: &str) {
        let denied = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/jobs")
                    .header("x-tikeo-api-key", api_key)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"namespace":"default","app":"other","name":"blocked"}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(denied.status(), axum::http::StatusCode::FORBIDDEN);
    }


    async fn update_sdk_api_key(app: axum::Router, admin: &str, key_id: &str) {
        let updated = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/v1/management/api-keys/{key_id}"))
                    .header("authorization", format!("Bearer {admin}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"updated-management-key","scopes":["jobs:read"],"expires_at":null}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(updated.status().is_success());
        let body = axum::body::to_bytes(updated.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["id"], key_id);
        assert_eq!(json["data"]["name"], "updated-management-key");
        assert_eq!(json["data"]["scopes"][0], "jobs:read");
        assert_eq!(json["data"]["expires_at"], serde_json::Value::Null);
    }

    async fn assert_sdk_key_cannot_write_bound_app_after_scope_edit(app: axum::Router, api_key: &str) {
        let denied = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/jobs")
                    .header("x-tikeo-api-key", api_key)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"namespace":"default","app":"billing","name":"blocked-by-scope"}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(denied.status(), axum::http::StatusCode::FORBIDDEN);
    }

    async fn assert_sdk_api_key_list_still_contains_updated_key(
        app: axum::Router,
        admin: &str,
        key_id: &str,
    ) {
        let list = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/management/api-keys")
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(list.status().is_success());
        let body = axum::body::to_bytes(list.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let item = json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("api key list should be an array"))
            .iter()
            .find(|item| item["id"] == key_id)
            .unwrap_or_else(|| panic!("updated api key should remain listed"));
        assert_eq!(item["scopes"][0], "jobs:read");
    }

    async fn revoke_sdk_api_key(app: axum::Router, admin: &str, key_id: &str) {
        let revoked = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/management/api-keys/{key_id}"))
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(revoked.status().is_success());
    }

    async fn assert_revoked_sdk_key_rejected(app: axum::Router, api_key: &str) {
        let rejected = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/jobs")
                    .header("x-tikeo-api-key", api_key)
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(rejected.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_token_lifecycle_creates_lists_authenticates_and_revokes() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;

        let created = post_json_raw(
            app.clone(),
            "/api/v1/auth/api-tokens",
            r#"{"name":"nightly automation"}"#,
            Some(&admin),
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(created["data"]["token"]["name"], "nightly automation");
        assert_eq!(
            created["data"]["token"]["scopes"].as_array().map(Vec::len),
            Some(0)
        );
        let token_id = created["data"]["token"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("api token id should be present"))
            .to_owned();
        let api_token = created["data"]["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("api token value should be returned once"))
            .to_owned();
        assert_eq!(api_token.len(), 48);
        assert!(api_token.chars().all(|value| value.is_ascii_alphanumeric()));

        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/api-tokens")
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(list_response.status().is_success());
        let list_body = axum::body::to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let list_text = String::from_utf8(list_body.to_vec())
            .unwrap_or_else(|error| panic!("body should be utf8: {error}"));
        assert!(!list_text.contains("token_hash"));
        assert!(!list_text.contains(&api_token));
        let list: Value = serde_json::from_str(&list_text)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(list["code"], 0);
        assert_eq!(list["data"].as_array().map(Vec::len), Some(1));
        assert_eq!(list["data"][0]["id"], token_id);

        let me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("authorization", format!("Bearer {api_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(me.status().is_success());

        let revoke = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/auth/api-tokens/{token_id}"))
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(revoke.status().is_success());

        let rejected = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("authorization", format!("Bearer {api_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(rejected.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn scoped_api_token_limits_effective_permissions() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let created = post_json_raw(
            app.clone(),
            "/api/v1/auth/api-tokens",
            r#"{"name":"read only users","scopes":["users:read"]}"#,
            Some(&admin),
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(created["data"]["token"]["scopes"][0], "users:read");
        let api_token = created["data"]["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("api token value should be returned once"))
            .to_owned();

        let scoped_read = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/users")
                    .header("authorization", format!("Bearer {api_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(scoped_read.status().is_success());

        let scoped_write = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/users")
                    .header("authorization", format!("Bearer {api_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"scoped-denied","email":"scoped-denied@example.com","password":"Secret123!","role":"viewer"}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(scoped_write.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn api_token_policy_bounds_expiry_and_rotation_revokes_old_token() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let created = post_json_raw(
            app.clone(),
            "/api/v1/auth/api-tokens",
            r#"{"name":"short lived automation","scopes":["users:read"],"expires_in_seconds":900}"#,
            Some(&admin),
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(created["data"]["token"]["scopes"][0], "users:read");
        let first_token_id = created["data"]["token"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("api token id should be present"))
            .to_owned();
        let first_access_token = created["data"]["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("api token value should be returned once"))
            .to_owned();
        let first_expires_at = DateTime::parse_from_rfc3339(
            created["data"]["token"]["expires_at"]
                .as_str()
                .unwrap_or_else(|| panic!("expires_at should be present")),
        )
        .unwrap_or_else(|error| panic!("expires_at should be RFC3339: {error}"))
        .with_timezone(&Utc);
        assert!(
            first_expires_at <= Utc::now() + chrono::Duration::seconds(1_000),
            "requested 900 second token should not receive the default long session TTL"
        );

        let rotated = post_json_raw(
            app.clone(),
            &format!("/api/v1/auth/api-tokens/{first_token_id}/rotate"),
            r#"{"name":"rotated automation","expires_in_seconds":1800}"#,
            Some(&admin),
        )
        .await;
        assert_eq!(rotated["code"], 0);
        assert_ne!(rotated["data"]["token"]["id"], first_token_id);
        assert_eq!(rotated["data"]["token"]["name"], "rotated automation");
        assert_eq!(rotated["data"]["token"]["scopes"][0], "users:read");
        let rotated_access_token = rotated["data"]["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("rotated token value should be returned once"))
            .to_owned();
        assert_ne!(rotated_access_token, first_access_token);

        let old_token_rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("authorization", format!("Bearer {first_access_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(
            old_token_rejected.status(),
            axum::http::StatusCode::UNAUTHORIZED
        );

        let rotated_read = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/users")
                    .header("authorization", format!("Bearer {rotated_access_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(rotated_read.status().is_success());

        let list = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/api-tokens")
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        let list_body = axum::body::to_bytes(list.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let list_json: Value = serde_json::from_slice(&list_body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(list_json["data"].as_array().map(Vec::len), Some(1));
        assert_eq!(list_json["data"][0]["name"], "rotated automation");
    }

    #[tokio::test]
    async fn api_token_policy_rejects_ttl_outside_configured_bounds() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/api-tokens")
                    .header("authorization", format!("Bearer {admin}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"too long","expires_in_seconds":31536000}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
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
                .is_some_and(|message| message.contains("expires_in_seconds"))
        );
    }

    #[tokio::test]
    async fn api_token_scope_bindings_limit_job_namespace_and_app_access() {
        let app = router().await;
        let admin = admin_token(app.clone()).await;
        let _billing = post_json_raw(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"billing-visible"}"#,
            Some(&admin),
        )
        .await;
        let _payroll = post_json_raw(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"finance","app":"payroll","name":"payroll-hidden"}"#,
            Some(&admin),
        )
        .await;

        let created = post_json_raw(
            app.clone(),
            "/api/v1/auth/api-tokens",
            r#"{"name":"billing automation","scopes":["jobs:read","jobs:write"],"scope_bindings":[{"namespace":"default","app":"billing","worker_pool":"pool-a"}]}"#,
            Some(&admin),
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(
            created["data"]["token"]["scope_bindings"][0]["namespace"],
            "default"
        );
        assert_eq!(
            created["data"]["token"]["scope_bindings"][0]["app"],
            "billing"
        );
        assert_eq!(
            created["data"]["token"]["scope_bindings"][0]["worker_pool"],
            "pool-a"
        );
        let api_token = created["data"]["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("api token value should be returned once"))
            .to_owned();

        let visible_jobs = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/jobs")
                    .header("authorization", format!("Bearer {api_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert!(visible_jobs.status().is_success());
        let body = axum::body::to_bytes(visible_jobs.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["data"]["items"][0]["name"], "billing-visible");

        let denied_create = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/jobs")
                    .header("authorization", format!("Bearer {api_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"namespace":"finance","app":"payroll","name":"blocked"}"#,
                    ))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(denied_create.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn api_token_scope_bindings_filter_worker_pool_visibility() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let registry = crate::tunnel::WorkerRegistry::with_lifecycle(
            tikeo_storage::WorkerLifecycleRepository::new(db.clone()),
        );
        let (tx1, _rx1) = tokio::sync::mpsc::channel(1);
        let (tx2, _rx2) = tokio::sync::mpsc::channel(1);
        let mut pool_a = worker("pool-a-worker", "billing");
        pool_a
            .labels
            .insert("worker_pool".to_owned(), "pool-a".to_owned());
        let mut pool_b = worker("pool-b-worker", "billing");
        pool_b
            .labels
            .insert("worker_pool".to_owned(), "pool-b".to_owned());
        let registered_a = registry.register(pool_a, tx1).await;
        let _registered_b = registry.register(pool_b, tx2).await;
        let app = router_with_state(AppState::new(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db),
            registry,
            StandaloneCoordinator::shared("test-node"),
        ));
        let admin = admin_token(app.clone()).await;
        let created = post_json_raw(
            app.clone(),
            "/api/v1/auth/api-tokens",
            r#"{"name":"pool a worker reader","scopes":["workers:read"],"scope_bindings":[{"namespace":"default","app":"billing","worker_pool":"pool-a"}]}"#,
            Some(&admin),
        )
        .await;
        let api_token = created["data"]["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("api token value should be returned once"))
            .to_owned();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workers")
                    .header("authorization", format!("Bearer {api_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["online"], 1);
        assert_eq!(
            json["data"]["items"][0]["workerId"],
            registered_a.worker_id
        );
    }


    #[tokio::test]
    async fn workers_list_uses_persisted_online_sessions_as_authoritative_source() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let lifecycle = tikeo_storage::WorkerLifecycleRepository::new(db.clone());
        let registry = crate::tunnel::WorkerRegistry::default();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        registry.register(worker("registry-only", "billing"), tx).await;
        lifecycle
            .register_session(tikeo_storage::RegisterWorkerSession {
                worker_id: "wrk-db-online".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "billing".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "persisted-pod".to_owned(),
                connection_id: "conn-db-online".to_owned(),
                gateway_node_id: "tikeo-test".to_owned(),
                fencing_token: "token-db-online".to_owned(),
                lease_seconds: 30,
                capabilities_json: r#"["java"]"#.to_owned(),
                structured_capabilities_json: r#"{"tags":["java"],"sdkProcessors":["demo.echo"],"scriptRunners":[],"pluginProcessors":[]}"#.to_owned(),
                labels_json: r#"{"worker_pool":"blue"}"#.to_owned(),
                master_json: r#"{"domain":"default/billing/local/local","isMaster":true,"masterWorkerId":"wrk-db-online","term":1,"fencingToken":"wmf-test"}"#.to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("persisted worker should register: {error}"));
        let app = router_with_state(AppState::new(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db),
            registry,
            StandaloneCoordinator::shared("test-node"),
        ));

        let response = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/workers").await)
            .await
            .unwrap_or_else(|error| panic!("workers route should respond: {error}"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["data"]["online"], 1);
        assert_eq!(json["data"]["items"][0]["workerId"], "wrk-db-online");
        assert_eq!(json["data"]["items"][0]["clientInstanceId"], "persisted-pod");
    }

    #[tokio::test]
    async fn workers_list_shows_latest_generation_for_reconnected_logical_instance() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let registry = crate::tunnel::WorkerRegistry::with_lifecycle(
            tikeo_storage::WorkerLifecycleRepository::new(db.clone()),
        );
        let (tx1, _rx1) = tokio::sync::mpsc::channel(1);
        let (tx2, _rx2) = tokio::sync::mpsc::channel(1);
        let first = registry.register(worker("pod-1", "billing"), tx1).await;
        let second = registry.register(worker("pod-1", "billing"), tx2).await;
        let app = router_with_state(AppState::new(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db),
            registry,
            StandaloneCoordinator::shared("test-node"),
        ));

        let response = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/workers").await)
            .await
            .unwrap_or_else(|error| panic!("workers route should respond: {error}"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["data"]["online"], 1);
        assert_eq!(json["data"]["items"][0]["workerId"], second.worker_id);
        assert_eq!(json["data"]["items"][0]["generation"], 2);
        assert_eq!(json["data"]["items"][0]["status"], "online");
        assert_eq!(json["data"]["items"][0]["clientInstanceId"], "pod-1");
        assert_eq!(first.worker_id, second.worker_id);
    }

    #[tokio::test]
    async fn login_failure_uses_unauthorized_envelope() {
        let app = router().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"bootstrap_admin","password":"wrong"}"#,
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
    async fn login_accepts_email_identifier_for_password_session() {
        let app = router().await;
        ensure_bootstrap_admin(app.clone()).await;

        let login = post_json_without_auth(
            app,
            "/api/v1/auth/login",
            r#"{"username":"bootstrap.admin@example.com","password":"TestOnlyOwnerPassword!2026"}"#,
        )
        .await;

        assert_eq!(login["code"], 0);
        assert_eq!(login["data"]["username"], "bootstrap_admin");
        assert!(login["data"]["token"].as_str().is_some_and(|token| !token.is_empty()));
    }
