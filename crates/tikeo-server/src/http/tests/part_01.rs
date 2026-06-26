    #[tokio::test]
    async fn http_tls_listener_serves_https_when_configured() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("test listener should bind: {error}"));
        let addr = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("test listener should expose local addr: {error}"));
        let cert_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tls/server.crt");
        let key_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tls/server.key");
        let tls = TlsEndpointConfig {
            tls_enabled: true,
            mtls_required: false,
            cert_path: Some(cert_path.to_owned()),
            key_path: Some(key_path.to_owned()),
            client_ca_path: None,
        };
        let app = Router::new().route("/tls-smoke", get(|| async { "tls-ok" }));
        let server = tokio::spawn(async move {
            serve_listener_with_state(listener, app, &tls)
                .await
                .unwrap_or_else(|error| panic!("TLS listener should serve: {error}"));
        });

        let url = format!("https://{addr}/tls-smoke");
        let body = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap_or_else(|error| panic!("test client should build: {error}"))
            .get(url)
            .send()
            .await
            .unwrap_or_else(|error| panic!("TLS request should succeed: {error}"))
            .text()
            .await
            .unwrap_or_else(|error| panic!("TLS response body should read: {error}"));
        assert_eq!(body, "tls-ok");
        server.abort();
    }

    struct MockOidcProvider {
        issuer: String,
        token_hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        discovery_hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        userinfo_hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        server: tokio::task::JoinHandle<()>,
    }

    async fn authorize_oidc_state(app: axum::Router, redirect_uri: &str) -> String {
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/auth/oidc/authorize?redirect_uri={redirect_uri}"
                    ))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("authorize route should respond: {error}"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let authorization_url = json["data"]["authorization_url"]
            .as_str()
            .unwrap_or_else(|| panic!("authorization_url should be a string"));
        Url::parse(authorization_url)
            .unwrap_or_else(|error| panic!("authorization_url should parse: {error}"))
            .query_pairs()
            .find_map(|(key, value)| (key == "state").then(|| value.into_owned()))
            .unwrap_or_else(|| panic!("authorization_url should include state"))
    }

    async fn spawn_mock_oidc_provider() -> MockOidcProvider {
        let token_hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let discovery_hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let userinfo_hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let app = mock_oidc_router(
            token_hits.clone(),
            discovery_hits.clone(),
            userinfo_hits.clone(),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("mock OIDC listener should bind: {error}"));
        let base_url = format!(
            "http://{}",
            listener
                .local_addr()
                .unwrap_or_else(|error| panic!("mock OIDC addr should resolve: {error}"))
        );
        let issuer = format!("{base_url}/realms/tikeo");
        let app = app.with_state(base_url);
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .unwrap_or_else(|error| panic!("mock OIDC server should run: {error}"));
        });
        MockOidcProvider {
            issuer,
            token_hits,
            discovery_hits,
            userinfo_hits,
            server,
        }
    }

    fn mock_oidc_router(
        token_hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        discovery_hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        userinfo_hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    ) -> axum::Router<String> {
        axum::Router::new()
            .route(
                "/realms/tikeo/protocol/openid-connect/token",
                axum::routing::post(
                    move |axum::Form(form): axum::Form<
                        std::collections::HashMap<String, String>,
                    >| {
                        let token_hits = token_hits.clone();
                        async move {
                            assert_eq!(
                                form.get("grant_type").map(String::as_str),
                                Some("authorization_code")
                            );
                            assert_eq!(form.get("code").map(String::as_str), Some("mock-code"));
                            assert_eq!(
                                form.get("client_id").map(String::as_str),
                                Some("tikeo-web")
                            );
                            assert_eq!(
                                form.get("client_secret").map(String::as_str),
                                Some("super-secret")
                            );
                            token_hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            axum::Json(serde_json::json!({
                                "access_token": "provider-access-token",
                                "token_type": "Bearer"
                            }))
                        }
                    },
                ),
            )
            .route(
                "/realms/tikeo/.well-known/openid-configuration",
                axum::routing::get(
                    move |axum::extract::State(base_url): axum::extract::State<String>| {
                        let discovery_hits = discovery_hits.clone();
                        async move {
                            discovery_hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            axum::Json(serde_json::json!({
                                "issuer": format!("{base_url}/realms/tikeo"),
                                "userinfo_endpoint": format!("{base_url}/realms/tikeo/protocol/openid-connect/userinfo")
                            }))
                        }
                    },
                ),
            )
            .route(
                "/realms/tikeo/protocol/openid-connect/userinfo",
                axum::routing::get(move |headers: axum::http::HeaderMap| {
                    let userinfo_hits = userinfo_hits.clone();
                    async move {
                        assert_eq!(
                            headers
                                .get(axum::http::header::AUTHORIZATION)
                                .and_then(|value| value.to_str().ok()),
                            Some("Bearer provider-access-token")
                        );
                        userinfo_hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        axum::Json(serde_json::json!({
                            "sub": "idp-user-001",
                            "preferred_username": "oidc.alice",
                            "email": "alice@example.com"
                        }))
                    }
                }),
            )
    }

    #[tokio::test]
    async fn healthz_returns_ok() {
        let json = get_json("/healthz").await;

        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn readyz_returns_ok() {
        let response = request("/readyz").await;

        assert!(response.status().is_success());
    }

    #[tokio::test]
    async fn system_info_returns_tikeo_metadata() {
        let json = get_json("/api/v1/system/info").await;

        assert_eq!(json["code"], 0);
        assert_eq!(json["message"], "success");
        assert_eq!(json["data"]["name"], "tikeo");
        assert_eq!(json["data"]["version"], env!("CARGO_PKG_VERSION"));
        assert!(json["data"].get("gitTag").is_some());
        assert!(json["data"].get("gitSha").is_some());
        assert!(json["data"].get("buildTime").is_some());
        if let Some(git_tag) = json["data"]["gitTag"].as_str().filter(|value| !value.is_empty()) {
            assert_eq!(git_tag, format!("v{}", env!("CARGO_PKG_VERSION")));
        }
    }

    #[tokio::test]
    async fn http_tracing_echoes_or_generates_trace_id_headers() {
        let app = router().await;
        let echoed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/system/info")
                    .header("x-request-id", "trace-explicit-1")
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));
        assert_eq!(
            echoed
                .headers()
                .get("x-trace-id")
                .and_then(|value| value.to_str().ok()),
            Some("trace-explicit-1")
        );

        let generated = request_with(app, "/api/v1/system/info").await;
        let trace_id = generated
            .headers()
            .get("x-trace-id")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_else(|| panic!("trace id should be generated"));
        assert!(trace_id.starts_with("trc-"));
        assert!(trace_id.len() > 8);
    }

    #[tokio::test]
    async fn auth_status_reports_local_and_oidc_configuration_without_live_provider() {
        let local = get_json("/api/v1/auth/status").await;
        assert_eq!(local["code"], 0);
        assert_eq!(local["data"]["mode"], "local");
        assert_eq!(local["data"]["local_login_enabled"], true);
        assert_eq!(local["data"]["oidc"]["enabled"], false);
        assert_eq!(local["data"]["oidc"]["client_secret_configured"], false);

        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let mut auth = tikeo_config::AuthConfig::default();
        auth.oidc.enabled = true;
        auth.oidc.issuer_url = Some("https://idp.example.com/realms/tikeo".to_owned());
        auth.oidc.client_id = Some("tikeo-web".to_owned());
        auth.oidc.client_secret = Some("super-secret".to_owned());
        auth.oidc.scopes = vec![
            "openid".to_owned(),
            "profile".to_owned(),
            "email".to_owned(),
        ];
        let app = router_with_state(
            app_state!(
                JobRepository::new(db.clone()),
                JobInstanceRepository::new(db.clone()),
                JobInstanceLogRepository::new(db.clone()),
                JobInstanceAttemptRepository::new(db.clone()),
                UserRepository::new(db.clone()),
                ScriptRepository::new(db.clone()),
                WorkflowRepository::new(db.clone()),
                AuditLogRepository::new(db),
                crate::tunnel::WorkerRegistry::default(),
                StandaloneCoordinator::shared("test-node"),
            )
            .with_auth_config(auth),
        );
        let oidc = request_with(app, "/api/v1/auth/status").await;
        let body = axum::body::to_bytes(oidc.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["mode"], "oidc");
        assert_eq!(json["data"]["local_login_enabled"], true);
        assert_eq!(json["data"]["oidc"]["enabled"], true);
        assert_eq!(
            json["data"]["oidc"]["issuer_url"],
            "https://idp.example.com/realms/tikeo"
        );
        assert_eq!(json["data"]["oidc"]["client_id"], "tikeo-web");
        assert_eq!(json["data"]["oidc"]["client_secret_configured"], true);
        assert_eq!(json["data"]["oidc"]["scopes"][0], "openid");
        assert!(json["data"]["oidc"].get("client_secret").is_none());
    }

    #[tokio::test]
    async fn oidc_authorize_and_callback_shapes_are_local_without_live_provider() {
        let local = request("/api/v1/auth/oidc/authorize").await;
        assert_eq!(local.status(), axum::http::StatusCode::BAD_REQUEST);

        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let mut auth = tikeo_config::AuthConfig::default();
        auth.oidc.enabled = true;
        auth.oidc.issuer_url = Some("https://idp.example.com/realms/tikeo".to_owned());
        auth.oidc.client_id = Some("tikeo-web".to_owned());
        auth.oidc.client_secret = Some("super-secret".to_owned());
        let app = router_with_state(
            app_state!(
                JobRepository::new(db.clone()),
                JobInstanceRepository::new(db.clone()),
                JobInstanceLogRepository::new(db.clone()),
                JobInstanceAttemptRepository::new(db.clone()),
                UserRepository::new(db.clone()),
                ScriptRepository::new(db.clone()),
                WorkflowRepository::new(db.clone()),
                AuditLogRepository::new(db),
                crate::tunnel::WorkerRegistry::default(),
                StandaloneCoordinator::shared("test-node"),
            )
            .with_auth_config(auth),
        );

        let authorize = app
            .clone()
            .oneshot(Request::builder()
                    .uri("/api/v1/auth/oidc/authorize?redirect_uri=http://localhost:5173/auth/callback")
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")))
            .await
            .unwrap_or_else(|error| panic!("authorize route should respond: {error}"));
        let body = axum::body::to_bytes(authorize.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["provider"], "oidc");
        assert_eq!(json["data"]["client_id"], "tikeo-web");
        assert!(
            json["data"]["authorization_url"]
                .as_str()
                .is_some_and(|value| value.contains("response_type=code"))
        );
        let auth_url = json["data"]["authorization_url"]
            .as_str()
            .unwrap_or_else(|| panic!("authorization_url should be a string"));
        let state = Url::parse(auth_url)
            .unwrap_or_else(|error| panic!("authorization_url should parse: {error}"))
            .query_pairs()
            .find_map(|(key, value)| (key == "state").then(|| value.into_owned()))
            .unwrap_or_else(|| panic!("authorization_url should include persisted state"));
        assert_ne!(state, "fake");
        assert!(json["data"].get("client_secret").is_none());

        let callback = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/auth/oidc/callback?code=fake&state={state}&redirect_uri=http://localhost:5173/auth/callback"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("callback route should respond: {error}"));
        let status = callback.status();
        let body = axum::body::to_bytes(callback.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert_ne!(json["code"], 0);
        assert!(
            json["message"]
                .as_str()
                .is_some_and(|value| value.contains("token exchange failed"))
        );

        let replay = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/auth/oidc/callback?code=fake&state={state}&redirect_uri=http://localhost:5173/auth/callback"
                    ))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("replay callback route should respond: {error}"));
        let replay_body = axum::body::to_bytes(replay.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let replay_json: Value = serde_json::from_slice(&replay_body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(
            replay_json["message"]
                .as_str()
                .is_some_and(|value| value.contains("already used"))
        );
    }

    #[tokio::test]
    async fn oidc_callback_issues_opaque_session_for_mapped_external_subject() {
        let mock = spawn_mock_oidc_provider().await;
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        UserRepository::new(db.clone())
            .create_user(tikeo_storage::CreateUser {
                username: "oidc.alice".to_owned(),
                email: "oidc.alice@example.com".to_owned(),
                password: "external-oidc-login-disabled".to_owned(),
                role: "viewer".to_owned(),
                bootstrap_admin: false,
            })
            .await
            .unwrap_or_else(|error| panic!("mapped user should be created: {error}"));
        tikeo_storage::OidcIdentityRepository::new(db.clone())
            .upsert_identity(tikeo_storage::UpsertOidcIdentity {
                issuer: mock.issuer.clone(),
                subject: "idp-user-001".to_owned(),
                username: "oidc.alice".to_owned(),
                namespace: Some("tenant-a".to_owned()),
                app: Some("billing".to_owned()),
                worker_pool: None,
            })
            .await
            .unwrap_or_else(|error| panic!("oidc identity mapping should be created: {error}"));

        let mut auth = tikeo_config::AuthConfig::default();
        auth.oidc.enabled = true;
        auth.oidc.issuer_url = Some(mock.issuer.clone());
        auth.oidc.client_id = Some("tikeo-web".to_owned());
        auth.oidc.client_secret = Some("super-secret".to_owned());
        let app = router_with_state(
            app_state!(
                JobRepository::new(db.clone()),
                JobInstanceRepository::new(db.clone()),
                JobInstanceLogRepository::new(db.clone()),
                JobInstanceAttemptRepository::new(db.clone()),
                UserRepository::new(db.clone()),
                ScriptRepository::new(db.clone()),
                WorkflowRepository::new(db.clone()),
                AuditLogRepository::new(db),
                crate::tunnel::WorkerRegistry::default(),
                StandaloneCoordinator::shared("test-node"),
            )
            .with_auth_config(auth),
        );

        let state = authorize_oidc_state(app.clone(), "http://localhost:5173/auth/callback").await;
        let callback = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/auth/oidc/callback?code=mock-code&state={state}&redirect_uri=http://localhost:5173/auth/callback"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("callback route should respond: {error}"));
        let status = callback.status();
        let body = axum::body::to_bytes(callback.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["username"], "oidc.alice");
        assert_eq!(json["data"]["roles"][0], "viewer");
        let token = json["data"]["token"]
            .as_str()
            .filter(|value| value.len() == 48 && value.chars().all(|ch| ch.is_ascii_alphanumeric()))
            .unwrap_or_else(|| panic!("callback should return a local opaque tikeo token"));
        assert!(!token.contains("provider-access-token"));

        let me = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("me route should respond: {error}"));
        let body = axum::body::to_bytes(me.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let me_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(me_json["code"], 0);
        assert_eq!(me_json["data"]["username"], "oidc.alice");
        assert_eq!(me_json["data"]["scope_limited"], true);
        assert_eq!(
            me_json["data"]["scope_bindings"][0]["namespace"],
            "tenant-a"
        );
        assert_eq!(me_json["data"]["scope_bindings"][0]["app"], "billing");
        assert_eq!(mock.token_hits.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(
            mock.userinfo_hits.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        mock.server.abort();
    }


    #[tokio::test]
    async fn oidc_identity_mapping_api_is_tenant_governed_and_fail_closed() {
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
            AuditLogRepository::new(db),
            crate::tunnel::WorkerRegistry::default(),
            StandaloneCoordinator::shared("test-node"),
        ));

        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/oidc-identities",
                    &serde_json::json!({
                        "issuer": "https://idp.example.com/realms/tikeo",
                        "subject": "idp-user-001",
                        "username": "alice",
                        "namespace": "tenant-a",
                        "app": "billing",
                        "worker_pool": "critical"
                    })
                    .to_string(),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("OIDC mapping create should respond: {error}"));
        assert_eq!(created.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["namespace"], "tenant-a");
        assert_eq!(json["data"]["worker_pool"], "critical");
        let mapping_id = json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("mapping id should be string"))
            .to_owned();

        let listed = app
            .clone()
            .oneshot(
                admin_request_builder(app.clone(), "GET", "/api/v1/oidc-identities").await,
            )
            .await
            .unwrap_or_else(|error| panic!("OIDC mapping list should respond: {error}"));
        let body = axum::body::to_bytes(listed.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"].as_array().map(Vec::len), Some(1));

        let deleted = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "DELETE",
                    &format!("/api/v1/oidc-identities/{mapping_id}"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("OIDC mapping delete should respond: {error}"));
        assert_eq!(deleted.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn oidc_callback_exchanges_code_and_fetches_userinfo_before_local_session_mapping() {
        let mock = spawn_mock_oidc_provider().await;
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let mut auth = tikeo_config::AuthConfig::default();
        auth.oidc.enabled = true;
        auth.oidc.issuer_url = Some(mock.issuer.clone());
        auth.oidc.client_id = Some("tikeo-web".to_owned());
        auth.oidc.client_secret = Some("super-secret".to_owned());
        let app = router_with_state(
            app_state!(
                JobRepository::new(db.clone()),
                JobInstanceRepository::new(db.clone()),
                JobInstanceLogRepository::new(db.clone()),
                JobInstanceAttemptRepository::new(db.clone()),
                UserRepository::new(db.clone()),
                ScriptRepository::new(db.clone()),
                WorkflowRepository::new(db.clone()),
                AuditLogRepository::new(db),
                crate::tunnel::WorkerRegistry::default(),
                StandaloneCoordinator::shared("test-node"),
            )
            .with_auth_config(auth),
        );

        let state = authorize_oidc_state(app.clone(), "http://localhost:5173/auth/callback").await;
        let callback = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/auth/oidc/callback?code=mock-code&state={state}&redirect_uri=http://localhost:5173/auth/callback"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("callback route should respond: {error}"));
        let status = callback.status();
        let body = axum::body::to_bytes(callback.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert_ne!(json["code"], 0);
        assert!(
            json["message"]
                .as_str()
                .is_some_and(|value| value.contains("no local session mapping"))
        );
        assert_eq!(mock.token_hits.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(
            mock.discovery_hits
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        assert_eq!(
            mock.userinfo_hits.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        mock.server.abort();
    }

    #[tokio::test]
    async fn observability_status_reports_default_and_configured_otlp_without_collector() {
        let app = router().await;
        let default_status = app
            .clone()
            .oneshot(
                admin_request_builder(app.clone(), "GET", "/api/v1/observability/status").await,
            )
            .await
            .unwrap_or_else(|error| panic!("observability status route should respond: {error}"));
        let body = axum::body::to_bytes(default_status.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["logging"]["root_level"], "info");
        assert_eq!(json["data"]["logging"]["http"]["level"], "info");
        assert_eq!(json["data"]["logging"]["http"]["include_body"], false);
        assert_eq!(json["data"]["logging"]["sql"]["enabled"], false);
        assert_eq!(json["data"]["logging"]["sql"]["level"], "DEBUG");
        assert_eq!(json["data"]["logging"]["sql"]["include_values"], false);
        assert_eq!(json["data"]["logging"]["sql"]["slow_threshold_ms"], 250);
        assert_eq!(json["data"]["logging"]["console"]["enabled"], true);
        assert_eq!(json["data"]["logging"]["console"]["target"], "stdout");
        assert_eq!(json["data"]["logging"]["file"]["enabled"], false);
        assert_eq!(json["data"]["logging"]["elk"]["enabled"], false);
        assert_eq!(json["data"]["tracing"]["enabled"], false);
        assert_eq!(json["data"]["tracing"]["exporter"], "none");
        assert_eq!(json["data"]["ready"], true);

        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let mut observability = tikeo_config::ObservabilityConfig::default();
        observability.logging.root.level = "WARN".to_owned();
        observability.logging.http.level = "DEBUG".to_owned();
        observability.logging.sql.enabled = true;
        observability.logging.sql.level = "TRACE".to_owned();
        observability.logging.sql.include_values = true;
        observability.logging.sql.slow_threshold_ms = 125;
        observability.logging.channels.file.enabled = true;
        observability.logging.channels.file.path = "/tmp/tikeo-test-logs".to_owned();
        observability.logging.channels.elk.enabled = true;
        observability.logging.channels.elk.servers = "127.0.0.1:19094".to_owned();
        observability.tracing.enabled = true;
        observability.tracing.otlp_endpoint =
            Some("https://collector.example.com/v1/traces".to_owned());
        observability.tracing.headers = vec!["authorization".to_owned(), "x-tenant".to_owned()];
        let app = router_with_state(
            app_state!(
                JobRepository::new(db.clone()),
                JobInstanceRepository::new(db.clone()),
                JobInstanceLogRepository::new(db.clone()),
                JobInstanceAttemptRepository::new(db.clone()),
                UserRepository::new(db.clone()),
                ScriptRepository::new(db.clone()),
                WorkflowRepository::new(db.clone()),
                AuditLogRepository::new(db),
                crate::tunnel::WorkerRegistry::default(),
                StandaloneCoordinator::shared("test-node"),
            )
            .with_observability_config(observability),
        );
        let configured_status = app
            .clone()
            .oneshot(admin_request_builder(app, "GET", "/api/v1/observability/status").await)
            .await
            .unwrap_or_else(|error| panic!("observability status route should respond: {error}"));
        let body = axum::body::to_bytes(configured_status.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["logging"]["root_level"], "WARN");
        assert_eq!(json["data"]["logging"]["http"]["level"], "DEBUG");
        assert_eq!(json["data"]["logging"]["sql"]["enabled"], true);
        assert_eq!(json["data"]["logging"]["sql"]["level"], "TRACE");
        assert_eq!(json["data"]["logging"]["sql"]["include_values"], true);
        assert_eq!(json["data"]["logging"]["sql"]["slow_threshold_ms"], 125);
        assert_eq!(json["data"]["logging"]["file"]["enabled"], true);
        assert_eq!(
            json["data"]["logging"]["file"]["target"],
            "/tmp/tikeo-test-logs"
        );
        assert_eq!(json["data"]["logging"]["elk"]["enabled"], true);
        assert_eq!(
            json["data"]["logging"]["elk"]["target"],
            "127.0.0.1:19094"
        );
        assert_eq!(json["data"]["tracing"]["enabled"], true);
        assert_eq!(json["data"]["tracing"]["exporter"], "otlp");
        assert_eq!(json["data"]["tracing"]["endpoint_configured"], true);
        assert_eq!(
            json["data"]["tracing"]["header_names"]
                .as_array()
                .map(Vec::len),
            Some(2)
        );
        assert!(json["data"]["tracing"].get("otlp_endpoint").is_none());
        assert_eq!(json["data"]["ready"], true);
    }

    #[tokio::test]
    async fn transport_security_status_reports_defaults_and_partial_mtls_config() {
        let app = router().await;
        let default_status = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/security/transport").await)
            .await
            .unwrap_or_else(|error| panic!("transport status route should respond: {error}"));
        let body = axum::body::to_bytes(default_status.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["http"]["tls_enabled"], false);
        assert_eq!(json["data"]["http"]["listener_mode"], "plaintext");
        assert_eq!(json["data"]["worker_tunnel"]["mtls_required"], false);
        assert_eq!(json["data"]["ready"], true);
        assert_eq!(json["data"]["issues"].as_array().map(Vec::len), Some(0));

        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let mut security = tikeo_config::TransportSecurityConfig::default();
        security.worker_tunnel.tls_enabled = true;
        security.worker_tunnel.mtls_required = true;
        security.worker_tunnel.cert_path = Some("/certs/worker.crt".to_owned());
        let app = router_with_state(
            app_state!(
                JobRepository::new(db.clone()),
                JobInstanceRepository::new(db.clone()),
                JobInstanceLogRepository::new(db.clone()),
                JobInstanceAttemptRepository::new(db.clone()),
                UserRepository::new(db.clone()),
                ScriptRepository::new(db.clone()),
                WorkflowRepository::new(db.clone()),
                AuditLogRepository::new(db),
                crate::tunnel::WorkerRegistry::default(),
                StandaloneCoordinator::shared("test-node"),
            )
            .with_transport_security_config(security),
        );
        let partial_status = app
            .clone()
            .oneshot(admin_request_builder(app, "GET", "/api/v1/security/transport").await)
            .await
            .unwrap_or_else(|error| panic!("transport status route should respond: {error}"));
        let body = axum::body::to_bytes(partial_status.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["worker_tunnel"]["tls_enabled"], true);
        assert_eq!(json["data"]["worker_tunnel"]["mtls_required"], true);
        assert_eq!(json["data"]["worker_tunnel"]["cert_configured"], true);
        assert_eq!(json["data"]["worker_tunnel"]["key_configured"], false);
        assert_eq!(json["data"]["worker_tunnel"]["ca_configured"], false);
        assert_eq!(
            json["data"]["worker_tunnel"]["listener_mode"],
            "tls_config_error"
        );
        assert_eq!(json["data"]["ready"], false);
        assert!(
            json["data"]["issues"]
                .as_array()
                .unwrap_or_else(|| panic!("issues array"))
                .iter()
                .any(|issue| issue
                    .as_str()
                    .is_some_and(|value| value.contains("key_path")))
        );

        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let mut wired_security = tikeo_config::TransportSecurityConfig::default();
        wired_security.http.tls_enabled = true;
        wired_security.http.cert_path =
            Some(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tls/server.crt").to_owned());
        wired_security.http.key_path =
            Some(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tls/server.key").to_owned());
        let app = router_with_state(
            app_state!(
                JobRepository::new(db.clone()),
                JobInstanceRepository::new(db.clone()),
                JobInstanceLogRepository::new(db.clone()),
                JobInstanceAttemptRepository::new(db.clone()),
                UserRepository::new(db.clone()),
                ScriptRepository::new(db.clone()),
                WorkflowRepository::new(db.clone()),
                AuditLogRepository::new(db),
                crate::tunnel::WorkerRegistry::default(),
                StandaloneCoordinator::shared("test-node"),
            )
            .with_transport_security_config(wired_security),
        );
        let tls_status = app
            .clone()
            .oneshot(admin_request_builder(app, "GET", "/api/v1/security/transport").await)
            .await
            .unwrap_or_else(|error| panic!("transport status route should respond: {error}"));
        let body = axum::body::to_bytes(tls_status.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["http"]["listener_mode"], "tls");
        assert_eq!(json["data"]["http"]["cert_configured"], true);
        assert_eq!(json["data"]["http"]["key_configured"], true);
        assert_eq!(json["data"]["ready"], true);
    }

    #[tokio::test]
    async fn security_posture_projects_real_policy_sources() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let mut transport = tikeo_config::TransportSecurityConfig::default();
        transport.worker_tunnel.tls_enabled = true;
        transport.worker_tunnel.mtls_required = true;
        transport.worker_tunnel.cert_path = Some("/missing/worker.crt".to_owned());
        let governance = ScriptGovernanceConfig {
            release_signature_secret_ref: Some("env:TIKEO_SCRIPT_RELEASE_SECRET".to_owned()),
        };
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
                StandaloneCoordinator::shared("security-test-node"),
            )
            .with_transport_security_config(transport)
            .with_script_governance_config(governance)
            .with_raft_transport_token(Some("cluster-token".to_owned())),
        );

        let safe_script = post_json(
            app.clone(),
            "/api/v1/scripts",
            r#"{"name":"safe-script","language":"python","version":"1.0.0","content":"print('safe')","timeout_seconds":3,"max_memory_bytes":4096,"allow_network":false}"#,
        )
        .await;
        assert_eq!(safe_script["code"], 0);
        let unsafe_response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/scripts",
                    r#"{"name":"unsafe-script","language":"python","version":"1.0.0","content":"print('unsafe')","timeout_seconds":3,"max_memory_bytes":4096,"allow_network":false,"policy":{"network":{"enabled":true,"allowed_hosts":["api.example.com"]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"resources":{"timeout_ms":30000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"sandbox":{"backend":"auto"},"env_vars":[]}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("unsafe script route should respond: {error}"));
        assert_eq!(unsafe_response.status(), axum::http::StatusCode::BAD_REQUEST, "dangerous policy must remain rejected");

        let response = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/security/posture").await)
            .await
            .unwrap_or_else(|error| panic!("security posture route should respond: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["scriptGovernance"]["totalScripts"], 1);
        assert_eq!(json["data"]["scriptGovernance"]["safeDefaultDenyScripts"], 1);
        assert_eq!(json["data"]["scriptGovernance"]["dangerousPolicyScripts"], 0);
        assert_eq!(json["data"]["scriptGovernance"]["releaseSignatureRequired"], true);
        assert_eq!(json["data"]["clusterTransport"]["raftTransportTokenConfigured"], true);
        assert_eq!(json["data"]["transport"]["ready"], false);
        assert!(json["data"]["checks"].as_array().is_some_and(|checks| checks.iter().any(|check| {
            check["source"] == "config" && check["status"] == "warning" && check["id"] == "transport.worker_tunnel"
        })));
        assert!(json["data"]["checks"].as_array().is_some_and(|checks| checks.iter().any(|check| {
            check["source"] == "script_policy_snapshot" && check["id"] == "script.default_deny"
        })));
    }

    #[tokio::test]
    async fn cluster_status_reports_explicit_standalone_role() {
        let json = get_json("/api/v1/cluster").await;

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["mode"], "standalone");
        assert_eq!(json["data"]["role"], "standalone");
        assert_eq!(json["data"]["nodes"], 1);
        assert_eq!(json["data"]["can_schedule"], true);
        assert_eq!(
            json["data"]["leaderFencingToken"],
            serde_json::Value::Null
        );
    }

    #[tokio::test]
    async fn api_responses_identify_the_responding_server_node() {
        let response = request("/api/v1/cluster").await;
        assert!(response.status().is_success());
        assert_eq!(
            response
                .headers()
                .get("x-tikeo-node-id")
                .and_then(|value| value.to_str().ok()),
            Some("test-node")
        );
    }


    #[tokio::test]
    async fn cluster_diagnostics_probes_remote_member_cluster_status() {
        let remote = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("remote test listener should bind: {error}"));
        let remote_addr = remote
            .local_addr()
            .unwrap_or_else(|error| panic!("remote listener addr should exist: {error}"));
        tokio::spawn(async move {
            axum::serve(
                remote,
                Router::new().route(
                    "/api/v1/cluster",
                    get(|| async {
                        Json(serde_json::json!({
                            "code": 0,
                            "message": "success",
                            "data": {
                                "mode": "raft",
                                "role": "follower",
                                "node_id": "remote-pod",
                                "nodes": 2,
                                "can_schedule": false,
                                "leader_fencing_token": null,
                                "detail": "remote diagnostic probe"
                            }
                        }))
                    }),
                ),
            )
            .await
            .unwrap_or_else(|error| panic!("remote test server should run: {error}"));
        });

        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let raft = RaftRepository::new(db.clone());
        raft.upsert_member(tikeo_storage::UpsertRaftMember {
            node_id: "test-node".to_owned(),
            endpoint: "http://127.0.0.1:1".to_owned(),
            status: "active".to_owned(),
        })
        .await
        .unwrap_or_else(|error| panic!("local member should persist: {error}"));
        raft.upsert_member(tikeo_storage::UpsertRaftMember {
            node_id: "remote-pod".to_owned(),
            endpoint: format!("http://{remote_addr}"),
            status: "active".to_owned(),
        })
        .await
        .unwrap_or_else(|error| panic!("remote member should persist: {error}"));
        let app = router_with_state(app_state!(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db),
            crate::tunnel::WorkerRegistry::default(),
            StaticCoordinator::shared(ClusterStatus {
                mode: ClusterMode::Raft,
                role: ClusterRole::Leader,
                node_id: "test-node".to_owned(),
                nodes: 2,
                can_schedule: true,
                leader_fencing_token: Some("raft:term:1:node:test-node".to_owned()),
                detail: "local leader".to_owned(),
            }),
        ));

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/cluster/diagnostics")
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("diagnostics should respond: {error}"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be json: {error}"));
        let remote_node = json["data"]["nodes"]
            .as_array()
            .unwrap_or_else(|| panic!("nodes should be array: {json}"))
            .iter()
            .find(|node| node["nodeId"] == "remote-pod")
            .unwrap_or_else(|| panic!("remote node should exist: {json}"));
        assert_eq!(remote_node["probeStatus"], "ok");
        assert_eq!(remote_node["observedRole"], "follower");
        assert_eq!(remote_node["observedCanSchedule"], false);
        assert!(remote_node["probeLatencyMs"].as_u64().is_some());
    }

    #[tokio::test]
    async fn cluster_diagnostics_exposes_runtime_boundary_without_fake_leader() {
        let json = get_json("/api/v1/cluster/diagnostics").await;

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["respondingNode"]["node_id"], "test-node");
        assert_eq!(json["data"]["status"]["role"], "standalone");
        assert_eq!(json["data"]["nodes"][0]["nodeId"], "test-node");
        assert_eq!(json["data"]["nodes"][0]["isRespondingNode"], true);
        assert_eq!(json["data"]["nodes"][0]["canSchedule"], true);
        assert_eq!(json["data"]["schedulingGated"], false);
        assert_eq!(
            json["data"]["transport"]["append_entries_path"],
            "/api/v1/raft/append-entries"
        );
        assert_eq!(json["data"]["transport"]["mutating"], false);
        assert_eq!(
            json["data"]["transport"]["status"],
            "standalone_unavailable"
        );
        assert_eq!(
            json["data"]["runtimeBoundary"],
            "tikv/raft-rs runtime can tick, accept inbound messages, emit gated membership proposals, and apply committed ConfChange with persisted ConfState; leader fencing remains required for scheduling/proposals"
        );
    }

    #[tokio::test]
    async fn cluster_diagnostics_exposes_smart_gateway_safe_optimization_evidence() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let lifecycle = tikeo_storage::WorkerLifecycleRepository::new(db.clone());
        lifecycle
            .register_session(tikeo_storage::RegisterWorkerSession {
                worker_id: "worker-local".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "demo".to_owned(),
                cluster: "kind".to_owned(),
                region: "local".to_owned(),
                client_instance_id: "demo-local".to_owned(),
                connection_id: "conn-local".to_owned(),
                gateway_node_id: "test-node".to_owned(),
                fencing_token: "local-token".to_owned(),
                lease_seconds: 300,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("local worker should register: {error}"));
        lifecycle
            .register_session(tikeo_storage::RegisterWorkerSession {
                worker_id: "worker-remote".to_owned(),
                namespace_name: "default".to_owned(),
                app_name: "demo".to_owned(),
                cluster: "kind".to_owned(),
                region: "remote".to_owned(),
                client_instance_id: "demo-remote".to_owned(),
                connection_id: "conn-remote".to_owned(),
                gateway_node_id: "remote-node".to_owned(),
                fencing_token: "remote-token".to_owned(),
                lease_seconds: 300,
                capabilities_json: "[]".to_owned(),
                structured_capabilities_json: "{}".to_owned(),
                labels_json: "{}".to_owned(),
                master_json: "{}".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("remote worker should register: {error}"));
        tikeo_storage::WorkerDispatchOutboxRepository::new(db.clone())
            .create(tikeo_storage::CreateWorkerDispatchOutbox {
                instance_id: "inst-gateway".to_owned(),
                attempt_id: "attempt-gateway".to_owned(),
                worker_id: "worker-remote".to_owned(),
                logical_instance_id: "default/demo/kind/local/demo-remote".to_owned(),
                gateway_node_id: "remote-node".to_owned(),
                gateway_generation: 1,
                assignment_token: "assignment-gateway".to_owned(),
                dispatch_payload: r#"{"instanceId":"inst-gateway"}"#.to_owned(),
                shard_id: 3,
                shard_map_version: 1,
                shard_count: 16,
                owner_node_id: "test-node".to_owned(),
                owner_epoch: 1,
                owner_fencing_token: "owner-fence".to_owned(),
                next_delivery_at: None,
            })
            .await
            .unwrap_or_else(|error| panic!("outbox row should create: {error}"));

        let app = router_with_state(app_state!(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db),
            crate::tunnel::WorkerRegistry::default().with_gateway_node_id("test-node"),
            StaticCoordinator::shared(ClusterStatus {
                mode: ClusterMode::Raft,
                role: ClusterRole::Follower,
                node_id: "test-node".to_owned(),
                nodes: 2,
                can_schedule: false,
                leader_fencing_token: None,
                detail: "non-leader diagnostic endpoint".to_owned(),
            }),
        ));

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/cluster/diagnostics")
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("diagnostics should respond: {error}"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be json: {error}"));

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["smartGateway"]["mode"], "diagnostic_safe_optimization");
        assert_eq!(json["data"]["smartGateway"]["localGatewayNodeId"], "test-node");
        assert_eq!(json["data"]["smartGateway"]["onlineWorkers"], 2);
        assert_eq!(json["data"]["smartGateway"]["localGatewayWorkers"], 1);
        assert_eq!(json["data"]["smartGateway"]["remoteGatewayWorkers"], 1);
        assert_eq!(json["data"]["smartGateway"]["outboxTotal"], 1);
        assert_eq!(json["data"]["smartGateway"]["queuedOrReroutePending"], 1);
        assert_eq!(json["data"]["smartGateway"]["status"], "ready");
        assert!(json["data"]["smartGateway"]["oldestQueuedAgeSeconds"].as_u64().is_some());
        assert!(json["data"]["smartGateway"]["safetyBoundary"]
            .as_str()
            .unwrap_or_default()
            .contains("durable outbox remains the source of truth"));
    }

    #[tokio::test]
    async fn openapi_json_contains_management_paths() {
        let json = get_json("/api-docs/openapi.json").await;

        assert!(json["paths"]["/api/v1/system/info"].is_object());
        assert!(json["paths"]["/api/v1/cluster/diagnostics"].is_object());
        assert!(json["paths"]["/api/v1/auth/login"].is_object());
        assert!(json["paths"]["/api/v1/raft/append-entries"].is_object());
        assert!(json["paths"]["/api/v1/auth/me"].is_object());
        assert!(json["paths"]["/api/v1/auth/logout"].is_object());
        assert!(json["paths"]["/api/v1/jobs"].is_object());
        assert!(json["paths"]["/api/v1/jobs/{job}:trigger"].is_object());
        assert!(json["paths"]["/api/v1/jobs/{job}/instances"].is_object());
        assert!(json["paths"]["/api/v1/instances/{instance}"].is_object());
        assert!(json["paths"]["/api/v1/instances/{instance}/logs"].is_object());
        assert!(json["paths"]["/api/v1/instances/{instance}/attempts"].is_object());
    }

    #[tokio::test]
    async fn raft_append_entries_placeholder_returns_envelope_without_accepting_leadership() {
        let app = router().await;
        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    "/api/v1/raft/append-entries",
                    r#"{"from":1,"to":2,"term":1,"message_type":"MsgAppend","index":0,"log_term":0,"commit":0,"snapshot_index":null,"snapshot_term":null,"entries":[],"context":null,"reject":false,"reject_hint":null,"leaderFencingToken":"candidate"}"#,
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
        assert_eq!(json["data"]["accepted"], false);
        assert!(
            json["data"]["reason"]
                .as_str()
                .is_some_and(|value| value.contains("runtime inbox is not available"))
        );
        assert_eq!(json["data"]["local_role"], "standalone");
        assert_eq!(
            json["data"]["leaderFencingToken"],
            serde_json::Value::Null
        );
        assert_eq!(json["data"]["received_term"], 1);
    }

    #[tokio::test]
    async fn raft_append_entries_invalid_message_returns_error_envelope() {
        let app = router().await;
        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    "/api/v1/raft/append-entries",
                    r#"{"from":1,"to":2,"term":-1,"message_type":"MsgAppend","index":0,"log_term":0,"commit":0,"snapshot_index":null,"snapshot_term":null,"entries":[],"context":null,"reject":false,"reject_hint":null,"leaderFencingToken":null}"#,
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
                .is_some_and(|value| value.contains("term cannot be negative"))
        );
        assert!(json.get("data").is_some());
    }
