//! HTTP management gateway for tikee.

pub mod auth;
pub mod dto;
pub mod error;
pub mod openapi;
pub mod routes;
pub mod services;
pub mod session;
pub mod trace;

use std::{net::SocketAddr, sync::Arc, time::SystemTime};

use crate::cluster::{SharedClusterCoordinator, StandaloneCoordinator};
use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{MatchedPath, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use tikee_config::AuthConfig;
use tikee_core::HealthState;
use tikee_storage::{
    AlertRepository, AuditLogRepository, AuthSessionRepository, JobInstanceAttemptRepository,
    JobInstanceLogRepository, JobInstanceRepository, JobRepository, RaftRepository, RbacRepository,
    ScriptRepository, UserRepository, WorkflowRepository, connect_and_migrate,
};

use tokio::{net::TcpListener, signal};
use tracing::info;
use utoipa::OpenApi;

use self::{
    openapi::ApiDoc,
    services::RbacService,
    session::{DbMokaSessionStore, SessionManager},
};

/// Shared HTTP application state.
#[derive(Debug, Clone)]
pub struct AppState {
    started_at: SystemTime,
    jobs: JobRepository,
    instances: JobInstanceRepository,
    logs: JobInstanceLogRepository,
    attempts: JobInstanceAttemptRepository,
    users: UserRepository,
    scripts: ScriptRepository,
    workflows: WorkflowRepository,
    audit: AuditLogRepository,
    alerts: AlertRepository,
    auth_config: AuthConfig,
    pub(crate) raft: RaftRepository,
    sessions: SessionManager,
    pub(crate) rbac: RbacService,
    pub(crate) registry: crate::tunnel::WorkerRegistry,
    pub(crate) cluster: SharedClusterCoordinator,
    pub(crate) raft_transport_token: Option<String>,
}

impl AppState {
    /// Create shared HTTP state.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        jobs: JobRepository,
        instances: JobInstanceRepository,
        logs: JobInstanceLogRepository,
        attempts: JobInstanceAttemptRepository,
        users: UserRepository,
        scripts: ScriptRepository,
        workflows: WorkflowRepository,
        audit: AuditLogRepository,
        registry: crate::tunnel::WorkerRegistry,
        cluster: SharedClusterCoordinator,
    ) -> Self {
        let db = users.db();
        let rbac = RbacService::new(RbacRepository::new(db.clone()));
        let raft = RaftRepository::new(db.clone());
        let alerts = AlertRepository::new(db.clone());
        let sessions = SessionManager::new(DbMokaSessionStore::new(
            AuthSessionRepository::new(db.clone()),
            RbacRepository::new(db),
        ));
        Self {
            started_at: SystemTime::now(),
            jobs,
            instances,
            logs,
            attempts,
            users,
            scripts,
            workflows,
            audit,
            alerts,
            auth_config: AuthConfig::default(),
            raft,
            sessions,
            rbac,
            registry,
            cluster,
            raft_transport_token: None,
        }
    }

    /// Attach auth/SSO configuration metadata.
    #[must_use]
    pub fn with_auth_config(mut self, auth_config: AuthConfig) -> Self {
        self.auth_config = auth_config;
        self
    }

    /// Attach the optional internal Raft transport token.
    #[must_use]
    pub fn with_raft_transport_token(mut self, token: Option<String>) -> Self {
        self.raft_transport_token = token.filter(|value| !value.is_empty());
        self
    }
}

/// Construct the HTTP router with an explicit application state.
pub fn router_with_state(state: AppState) -> Router {
    let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
    let handle = recorder.handle();

    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(move || std::future::ready(handle.render())))
        .nest(
            "/api/v1",
            api_router()
                .layer(middleware::from_fn(record_http_metrics))
                .layer(middleware::from_fn(trace::trace_http)),
        )
        .route("/api-docs/openapi.json", get(openapi_json))
        .with_state(Arc::new(state))
}

async fn router_for_database(database_url: &str) -> Result<Router> {
    let db = connect_and_migrate(database_url)
        .await
        .with_context(|| format!("failed to initialize storage at {database_url}"))?;
    Ok(router_with_state(AppState::new(
        JobRepository::new(db.clone()),
        JobInstanceRepository::new(db.clone()),
        JobInstanceLogRepository::new(db.clone()),
        JobInstanceAttemptRepository::new(db.clone()),
        UserRepository::new(db.clone()),
        ScriptRepository::new(db.clone()),
        WorkflowRepository::new(db.clone()),
        AuditLogRepository::new(db),
        crate::tunnel::WorkerRegistry::default(),
        StandaloneCoordinator::shared("standalone-http"),
    )))
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

async fn record_http_metrics(request: Request<axum::body::Body>, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.extensions().get::<MatchedPath>().map_or_else(
        || request.uri().path().to_owned(),
        |matched| matched.as_str().to_owned(),
    );
    let started = std::time::Instant::now();
    let response = next.run(request).await;
    let status = response.status().as_u16().to_string();
    let labels = [
        ("method", method.as_str().to_owned()),
        ("path", path),
        ("status", status),
    ];
    metrics::counter!("tikee_http_requests_total", &labels).increment(1);
    metrics::histogram!("tikee_http_request_duration_seconds", &labels)
        .record(started.elapsed().as_secs_f64());
    response
}

#[allow(clippy::too_many_lines)]
fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/system/info", get(routes::system_info))
        .route("/metrics/summary", get(routes::metrics_summary))
        .route("/cluster", get(routes::cluster_status))
        .route("/cluster/diagnostics", get(routes::cluster_diagnostics))
        .route(
            "/raft/append-entries",
            axum::routing::post(routes::append_entries),
        )
        .route(
            "/raft/members:propose",
            axum::routing::post(routes::propose_member_change),
        )
        .route("/auth/login", axum::routing::post(auth::login))
        .route("/auth/status", get(auth::status))
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", axum::routing::post(auth::logout))
        .route(
            "/users",
            axum::routing::get(routes::list_users).post(routes::create_user),
        )
        .route(
            "/users/{id}",
            axum::routing::patch(routes::update_user).delete(routes::delete_user),
        )
        .route(
            "/scripts",
            get(routes::list_scripts).post(routes::create_script),
        )
        .route(
            "/scripts/{id}",
            get(routes::get_script)
                .patch(routes::update_script)
                .delete(routes::delete_script),
        )
        .route("/scripts/{id}/versions", get(routes::list_script_versions))
        .route(
            "/scripts/{id}/publish",
            axum::routing::post(routes::publish_script),
        )
        .route(
            "/scripts/{id}/rollback",
            axum::routing::post(routes::rollback_script),
        )
        .route("/scripts/{id}/diff", get(routes::diff_script_versions))
        .route(
            "/workflows",
            get(routes::list_workflows).post(routes::create_workflow),
        )
        .route(
            "/workflows/dry-run",
            axum::routing::post(routes::dry_run_workflow),
        )
        .route(
            "/workflows/{id}",
            get(routes::get_workflow).patch(routes::update_workflow),
        )
        .route(
            "/workflows/{id}/validate",
            axum::routing::post(routes::validate_workflow),
        )
        .route(
            "/workflows/{id}/run",
            axum::routing::post(routes::run_workflow),
        )
        .route(
            "/workflow-instances/materialize-next",
            axum::routing::post(routes::materialize_next_workflow_node),
        )
        .route(
            "/workflow-instances/{id}",
            get(routes::get_workflow_instance_route),
        )
        .route(
            "/workflow-instances/{id}/advance",
            axum::routing::post(routes::advance_workflow_instance),
        )
        .route(
            "/workflow-instances/{id}/recover",
            axum::routing::post(routes::recover_workflow_node),
        )
        .route(
            "/workflow-instances/{id}/shards",
            get(routes::list_workflow_shards),
        )
        .route(
            "/workflow-shards/{id}/complete",
            axum::routing::post(routes::complete_workflow_shard),
        )
        .route(
            "/events/instances/{id}/stream",
            get(routes::stream_instance_events),
        )
        .route("/jobs", get(routes::list_jobs).post(routes::create_job))
        .route(
            "/jobs/{job_action}",
            axum::routing::post(routes::trigger_job),
        )
        .route("/jobs/{job}/instances", get(routes::list_job_instances))
        .route("/instances/{instance}", get(routes::get_job_instance))
        .route(
            "/instances/{instance}/logs",
            get(routes::list_instance_logs),
        )
        .route(
            "/instances/{instance}/attempts",
            get(routes::list_instance_attempts),
        )
        .route("/workers", get(routes::list_workers))
        .route("/dispatch-queue", get(routes::dispatch_queue))
        .route(
            "/dispatch-queue:claim",
            axum::routing::post(routes::claim_dispatch_queue),
        )
        .route("/audit-logs", get(routes::list_audit_logs))
        .route("/audit-logs:export", get(routes::export_audit_logs))
        .route(
            "/alert-rules",
            get(routes::list_alert_rules).post(routes::create_alert_rule),
        )
        .route("/alert-events", get(routes::list_alert_events))
        .route(
            "/alert-events/{id}/resolve",
            axum::routing::post(routes::resolve_alert_event),
        )
        .route(
            "/alert-events:summary",
            axum::routing::get(routes::list_alert_event_summaries),
        )
}

/// Run the unified HTTP listener.
///
/// # Errors
///
/// Returns an error when binding the configured listener address, initializing storage,
/// or serving HTTP fails.
pub async fn serve(listen_addr: SocketAddr, database_url: &str) -> Result<()> {
    serve_with_state(listen_addr, router_for_database(database_url).await?).await
}

/// Run the unified HTTP listener with prebuilt application state.
///
/// # Errors
///
/// Returns an error when binding the configured listener address or serving HTTP fails.
pub async fn serve_with_state(listen_addr: SocketAddr, router: Router) -> Result<()> {
    let listener = TcpListener::bind(listen_addr)
        .await
        .with_context(|| format!("failed to bind {listen_addr}"))?;

    info!(addr = %listen_addr, "tikee HTTP server listening");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("tikee HTTP server failed")
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    uptime_seconds: u64,
}

async fn healthz(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    health_response(&state)
}

async fn readyz(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    health_response(&state)
}

fn health_response(state: &AppState) -> (StatusCode, Json<HealthResponse>) {
    let uptime_seconds = state
        .started_at
        .elapsed()
        .map_or(0, |duration| duration.as_secs());

    (
        StatusCode::OK,
        Json(HealthResponse {
            status: HealthState::Ok.as_str(),
            uptime_seconds,
        }),
    )
}

async fn shutdown_signal() {
    if let Err(error) = signal::ctrl_c().await {
        tracing::warn!(%error, "failed to listen for shutdown signal");
    }
}

#[cfg(test)]
mod tests {
    use crate::cluster::{
        ClusterMode, ClusterRole, ClusterStatus, StandaloneCoordinator, StaticCoordinator,
        coordinator_from_config_with_storage,
    };
    use axum::{body::Body, http::Request};
    use serde_json::Value;
    use tikee_config::{ClusterConfig, ClusterModeConfig, ClusterPeerConfig};
    use tikee_core::{ExecutionMode, TriggerType};
    use tikee_proto::worker::v1::RegisterWorker;
    use tikee_storage::{
        AppendJobInstanceLog, AuditLogRepository, CreateAuditLog, CreateJob, CreateJobInstance,
        JobInstanceAttemptRepository, JobInstanceLogRepository, JobInstanceRepository,
        JobRepository, RaftRepository, ScriptRepository, UserRepository, WorkflowRepository,
        connect_and_migrate,
    };

    const ADMIN_LOGIN: &str = r#"{"username":"tikee_init","password":"Tikee@2026!"}"#;
    use tower::ServiceExt;

    use super::{AppState, router_with_state};

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
    async fn system_info_returns_tikee_metadata() {
        let json = get_json("/api/v1/system/info").await;

        assert_eq!(json["code"], 0);
        assert_eq!(json["message"], "success");
        assert_eq!(json["data"]["name"], "tikee");
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
        let mut auth = tikee_config::AuthConfig::default();
        auth.oidc.enabled = true;
        auth.oidc.issuer_url = Some("https://idp.example.com/realms/tikee".to_owned());
        auth.oidc.client_id = Some("tikee-web".to_owned());
        auth.oidc.client_secret = Some("super-secret".to_owned());
        auth.oidc.scopes = vec![
            "openid".to_owned(),
            "profile".to_owned(),
            "email".to_owned(),
        ];
        let app = router_with_state(
            AppState::new(
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
            "https://idp.example.com/realms/tikee"
        );
        assert_eq!(json["data"]["oidc"]["client_id"], "tikee-web");
        assert_eq!(json["data"]["oidc"]["client_secret_configured"], true);
        assert_eq!(json["data"]["oidc"]["scopes"][0], "openid");
        assert!(json["data"]["oidc"].get("client_secret").is_none());
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
            json["data"]["leader_fencing_token"],
            serde_json::Value::Null
        );
    }

    #[tokio::test]
    async fn cluster_diagnostics_exposes_runtime_boundary_without_fake_leader() {
        let json = get_json("/api/v1/cluster/diagnostics").await;

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["status"]["role"], "standalone");
        assert_eq!(json["data"]["scheduling_gated"], false);
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
            json["data"]["runtime_boundary"],
            "tikv/raft-rs runtime can tick, accept inbound messages, emit gated membership proposals, and apply committed ConfChange with persisted ConfState; leader fencing remains required for scheduling/proposals"
        );
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
                    r#"{"from":1,"to":2,"term":1,"message_type":"MsgAppend","index":0,"log_term":0,"commit":0,"snapshot_index":null,"snapshot_term":null,"entries":[],"context":null,"reject":false,"reject_hint":null,"leader_fencing_token":"candidate"}"#,
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
            json["data"]["leader_fencing_token"],
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
                    r#"{"from":1,"to":2,"term":-1,"message_type":"MsgAppend","index":0,"log_term":0,"commit":0,"snapshot_index":null,"snapshot_term":null,"entries":[],"context":null,"reject":false,"reject_hint":null,"leader_fencing_token":null}"#,
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

    #[tokio::test]
    async fn raft_append_entries_enqueues_when_runtime_exists_without_leadership() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let cluster = coordinator_from_config_with_storage(
            &ClusterConfig {
                mode: ClusterModeConfig::Raft,
                node_id: "tikee-0".to_owned(),
                peers: vec![
                    ClusterPeerConfig {
                        node_id: "tikee-0".to_owned(),
                        endpoint: "http://tikee-0.tikee-headless:9998".to_owned(),
                    },
                    ClusterPeerConfig {
                        node_id: "tikee-1".to_owned(),
                        endpoint: "http://tikee-1.tikee-headless:9998".to_owned(),
                    },
                ],
                transport_token: None,
            },
            &RaftRepository::new(db.clone()),
        )
        .await
        .unwrap_or_else(|error| panic!("raft coordinator should start: {error}"));
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
            cluster,
        ));

        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    "/api/v1/raft/append-entries",
                    r#"{"from":1,"to":2,"term":1,"message_type":"MsgHeartbeat","index":0,"log_term":0,"commit":0,"snapshot_index":null,"snapshot_term":null,"entries":[],"context":null,"reject":false,"reject_hint":null,"leader_fencing_token":null}"#,
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
            json["data"]["leader_fencing_token"],
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
                node_id: "tikee-0".to_owned(),
                peers: vec![
                    ClusterPeerConfig {
                        node_id: "tikee-0".to_owned(),
                        endpoint: "http://tikee-0.tikee-headless:9998".to_owned(),
                    },
                    ClusterPeerConfig {
                        node_id: "tikee-1".to_owned(),
                        endpoint: "http://tikee-1.tikee-headless:9998".to_owned(),
                    },
                ],
                transport_token: Some("secret-raft-token".to_owned()),
            },
            &RaftRepository::new(db.clone()),
        )
        .await
        .unwrap_or_else(|error| panic!("raft coordinator should start: {error}"));
        let app = router_with_state(
            AppState::new(
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
        let body = r#"{"from":1,"to":2,"term":1,"message_type":"MsgHeartbeat","index":0,"log_term":0,"commit":0,"snapshot_index":null,"snapshot_term":null,"entries":[],"context":null,"reject":false,"reject_hint":null,"leader_fencing_token":null}"#;

        let accepted = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/raft/append-entries")
                    .header("content-type", "application/json")
                    .header("x-tikee-raft-token", "secret-raft-token")
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
        assert_eq!(accepted_json["data"]["leader_fencing_token"], Value::Null);

        let rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/raft/append-entries")
                    .header("content-type", "application/json")
                    .header("x-tikee-raft-token", "wrong-token")
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
                    r#"{"proposal_id":"prop-1","action":"add_voter","node_id":"tikee-2","endpoint":"http://tikee-2.tikee-headless:9998"}"#,
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
                    r#"{"proposal_id":"prop-bad","action":"add_voter","node_id":"tikee-2","endpoint":"ftp://tikee-2"}"#,
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
        let request = r#"{"proposal_id":"prop-add-2","action":"add_voter","node_id":"tikee-2","endpoint":"http://tikee-2.tikee-headless:9998"}"#;

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
    #[allow(clippy::too_many_lines)]
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
        let app = router_with_state(AppState::new(
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
        assert_eq!(json["code"], 0);
        assert_eq!(
            json["data"]["condition"]["type"],
            "script_governance_failure"
        );

        crate::tunnel::governance::materialize_script_governance_audit(
            &AuditLogRepository::new(db.clone()),
            "tikee-dispatcher",
            "inst-alert-1",
            "script_runtime_unavailable",
            "runtime missing",
        )
        .await
        .unwrap_or_else(|error| panic!("governance materialization should append: {error}"));
        crate::tunnel::governance::materialize_script_governance_audit(
            &AuditLogRepository::new(db.clone()),
            "tikee-dispatcher",
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
                    app,
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
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn alert_event_recovery_appends_resolved_history_entry() {
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
            "tikee-dispatcher",
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
    #[allow(clippy::too_many_lines)]
    async fn alert_event_summary_rolls_up_history_by_rule_and_resource() {
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
            "tikee-dispatcher",
            "inst-alert-summary",
            "script_runtime_unavailable",
            "runtime missing",
        )
        .await
        .unwrap_or_else(|error| panic!("governance materialization should append: {error}"));
        crate::tunnel::governance::materialize_script_governance_audit(
            &AuditLogRepository::new(db.clone()),
            "tikee-dispatcher",
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
    #[allow(clippy::too_many_lines)]
    async fn metrics_summary_reports_storage_registry_and_alert_counts() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "metrics-job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let pending = instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        let succeeded = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        instances
            .update_status(&succeeded.id, tikee_core::InstanceStatus::Succeeded)
            .await
            .unwrap_or_else(|error| panic!("instance should update: {error}"));
        assert_eq!(pending.status, tikee_core::InstanceStatus::Pending);

        let registry = crate::tunnel::WorkerRegistry::default();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        registry
            .register(worker("metrics-worker", "billing"), tx)
            .await;

        let app = router_with_state(AppState::new(
            jobs,
            instances,
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            ScriptRepository::new(db.clone()),
            WorkflowRepository::new(db.clone()),
            AuditLogRepository::new(db.clone()),
            registry,
            StandaloneCoordinator::shared("test-node"),
        ));

        app.clone()
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
        crate::tunnel::governance::materialize_script_governance_audit(
            &AuditLogRepository::new(db.clone()),
            "tikee-dispatcher",
            "inst-metrics",
            "script_runtime_unavailable",
            "runtime missing",
        )
        .await
        .unwrap_or_else(|error| panic!("governance materialization should append: {error}"));

        let summary = app
            .clone()
            .oneshot(admin_request_builder(app, "GET", "/api/v1/metrics/summary").await)
            .await
            .unwrap_or_else(|error| panic!("metrics summary route should respond: {error}"));
        let status = summary.status();
        let body = axum::body::to_bytes(summary.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert!(status.is_success(), "unexpected status {status}: {json}");
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["workers"]["online"], 1);
        assert_eq!(json["data"]["instances"]["total"], 2);
        assert_eq!(json["data"]["instances"]["by_status"]["pending"], 1);
        assert_eq!(json["data"]["instances"]["by_status"]["succeeded"], 1);
        assert_eq!(json["data"]["alerts"]["total_events"], 1);
        assert_eq!(json["data"]["alerts"]["by_status"]["firing"], 1);
        assert_eq!(json["data"]["governance"]["script_failure_events"], 1);
        assert_eq!(
            json["data"]["governance"]["by_failure_class"]["script_runtime_unavailable"],
            1
        );
    }

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
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "governed-script".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: Some("script:script-missing-runtime".to_owned()),
                enabled: true,
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
            "tikee-dispatcher",
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
        assert!(response.status().is_success());
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
        let login = post_json_without_auth(
            app.clone(),
            "/api/v1/auth/login",
            r#"{"username":"tikee_init","password":"Tikee@2026!"}"#,
        )
        .await;

        assert_eq!(login["code"], 0);
        let token = login["data"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("login should return token"))
            .to_owned();
        assert!(token.starts_with("atk_"));
        assert_eq!(login["data"]["roles"][0], "admin");

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
        assert!(response.status().is_success());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let me: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(me["code"], 0);
        assert_eq!(me["data"]["username"], "tikee_init");
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
                        r#"{"username":"tikee_init","password":"wrong"}"#,
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
            app,
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
        let registry = crate::tunnel::WorkerRegistry::default();
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
            r#"{"trigger_type":"api","execution_mode":"broadcast"}"#,
        )
        .await;
        let instance_id = triggered["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("triggered instance should contain id"));
        assert_eq!(triggered["data"]["execution_mode"], "broadcast");

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
            r#"{"trigger_type":"api"}"#,
        )
        .await;

        assert_eq!(triggered["code"], 0);
        assert_eq!(triggered["data"]["job_id"], job_id);
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
            r#"{"trigger_type":"api"}"#,
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
                worker_id: "tikee-dispatcher".to_owned(),
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
            json["data"]["items"][1]["governance_event"],
            "script_execution_governance"
        );
        assert_eq!(
            json["data"]["items"][1]["governance_failure_class"],
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
            json["data"]["items"][0]["governance_failure_class"],
            "script_runtime_unavailable"
        );
    }

    #[tokio::test]
    async fn create_job_accepts_processor_binding() {
        let app = router().await;
        let json = post_json(
            app,
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"invoice-sync","schedule_type":"api","processor_name":"billing.invoice-sync"}"#,
        )
        .await;

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["processor_name"], "billing.invoice-sync");
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
        let login = post_json_raw(app, "/api/v1/auth/login", ADMIN_LOGIN, None).await;
        login["data"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("admin login should return token"))
            .to_owned()
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
            labels: std::collections::HashMap::default(),
        }
    }

    #[tokio::test]
    async fn broadcast_trigger_filters_workers_by_namespace_and_app() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let registry = crate::tunnel::WorkerRegistry::default();
        let (tx1, _rx1) = tokio::sync::mpsc::channel(1);
        let (tx2, _rx2) = tokio::sync::mpsc::channel(1);
        let worker_a = registry.register(worker("worker-a", "billing"), tx1).await;
        registry
            .register(worker("worker-b", "analytics"), tx2)
            .await;
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

        let created = post_json(
            app.clone(),
            "/api/v1/jobs",
            r#"{"namespace":"default","app":"billing","name":"broadcast-filter"}"#,
        )
        .await;
        let job_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("created job should contain id"));

        let triggered = post_json(
            app.clone(),
            &format!("/api/v1/jobs/{job_id}:trigger"),
            r#"{"trigger_type":"api","execution_mode":"broadcast"}"#,
        )
        .await;
        let instance_id = triggered["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("triggered instance should contain id"));
        assert_eq!(triggered["data"]["execution_mode"], "broadcast");

        let attempts =
            request_with(app, &format!("/api/v1/instances/{instance_id}/attempts")).await;
        let body = axum::body::to_bytes(attempts.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["data"]["items"][0]["worker_id"], worker_a.worker_id);
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_create_validate_run_and_advance_returns_envelopes() {
        let app = router().await;
        let create = post_json(
            app.clone(),
            "/api/v1/workflows",
            r#"{"name":"demo-flow","definition":{"nodes":[{"key":"start","name":"Start","kind":"job","job_id":"job-demo"},{"key":"fanout","name":"Fanout","kind":"map","map_items":[{"shard":1},{"shard":2}]},{"key":"child","name":"Child","kind":"sub_workflow","child_workflow_id":"wf_child"}],"edges":[{"from":"start","to":"fanout","condition":"on_success"},{"from":"fanout","to":"child","condition":"always"}]}}"#,
        )
        .await;
        assert_eq!(create["code"], 0);
        let workflow_id = create["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("workflow id should exist"));

        let validate = post_json(
            app.clone(),
            &format!("/api/v1/workflows/{workflow_id}/validate"),
            "{}",
        )
        .await;
        assert_eq!(validate["data"]["valid"], true);

        let dry_run = post_json(
            app.clone(),
            "/api/v1/workflows/dry-run",
            r#"{"nodes":[{"key":"start","kind":"job","job_id":"job-demo"}],"edges":[]}"#,
        )
        .await;
        assert_eq!(dry_run["data"]["validation"]["valid"], true);
        assert_eq!(dry_run["data"]["start_nodes"][0], "start");

        let run = post_json(
            app.clone(),
            &format!("/api/v1/workflows/{workflow_id}/run"),
            r#"{"trigger_type":"api"}"#,
        )
        .await;
        assert_eq!(run["code"], 0);
        assert_eq!(run["data"]["status"], "pending");
        assert_eq!(run["data"]["nodes"][0]["status"], "queued");
        let instance_id = run["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("workflow instance id should exist"));

        let materialized_job = post_json(
            app.clone(),
            "/api/v1/workflow-instances/materialize-next",
            "{}",
        )
        .await;
        assert_eq!(materialized_job["code"], 0);
        assert_eq!(materialized_job["data"]["node"]["node_key"], "start");
        assert!(materialized_job["data"]["node"]["job_instance_id"].is_string());

        let advanced = post_json(
            app.clone(),
            &format!("/api/v1/workflow-instances/{instance_id}/advance"),
            r#"{"node_key":"start","status":"succeeded","message":"ok"}"#,
        )
        .await;
        assert_eq!(advanced["code"], 0);
        assert_eq!(advanced["data"]["queued_nodes"][0], "fanout");
        assert_eq!(advanced["data"]["instance"]["status"], "running");

        let materialized_map = post_json(
            app.clone(),
            "/api/v1/workflow-instances/materialize-next",
            "{}",
        )
        .await;
        assert_eq!(materialized_map["data"]["node"]["node_key"], "fanout");
        assert_eq!(
            materialized_map["data"]["shards"].as_array().map(Vec::len),
            Some(2)
        );

        let shards = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    format!("/api/v1/workflow-instances/{instance_id}/shards"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        let body = axum::body::to_bytes(shards.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"].as_array().map(Vec::len), Some(2));
        let shard_id = json["data"][0]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("shard id should exist"));
        let shard_completed = post_json(
            app.clone(),
            &format!("/api/v1/workflow-shards/{shard_id}/complete"),
            r#"{"status":"succeeded","output":{"ok":true},"message":"done"}"#,
        )
        .await;
        assert_eq!(shard_completed["code"], 0);
        assert_eq!(shard_completed["data"]["shard"]["status"], "succeeded");

        let queue = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/dispatch-queue").await)
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        let body = axum::body::to_bytes(queue.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(json["data"]["items"].as_array().is_some());

        assert_workflow_audit_actions(app.clone()).await;
    }

    async fn assert_workflow_audit_actions(app: axum::Router) {
        let audit = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/audit-logs").await)
            .await
            .unwrap_or_else(|error| panic!("audit logs request should succeed: {error}"));
        let body = axum::body::to_bytes(audit.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("audit body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("audit body should be JSON: {error}"));
        let actions: Vec<_> = json["data"]["items"]
            .as_array()
            .unwrap_or_else(|| panic!("audit items should exist"))
            .iter()
            .filter(|item| {
                item["resource_type"] == "workflow"
                    || item["resource_type"] == "workflow_instance"
                    || item["resource_type"] == "workflow_node_instance"
            })
            .map(|item| item["action"].as_str().unwrap_or_default().to_owned())
            .collect();
        for expected in [
            "create",
            "validate",
            "dry-run",
            "run",
            "advance",
            "materialize",
        ] {
            assert!(
                actions.iter().any(|action| action == expected),
                "missing workflow audit action {expected}; got {actions:?}"
            );
        }
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn user_management_and_rbac_integration() {
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

        // 1. Get users list (should only contain seeded admin)
        let response = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/users").await)
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(json["data"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["data"][0]["username"], "tikee_init");

        // 2. Create an operator user
        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/users",
                    r#"{"username":"test_operator","password":"Password@123","role":"operator"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        let user_id = json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("expected JSON string"))
            .to_owned();

        // 3. Authenticate with newly created user
        let login = post_json_without_auth(
            app.clone(),
            "/api/v1/auth/login",
            r#"{"username":"test_operator","password":"Password@123"}"#,
        )
        .await;
        assert_eq!(login["code"], 0);
        let operator_token = login["data"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("expected JSON string"))
            .to_owned();

        // 4. Verification: Operator is not allowed to create users (Admin only) -> Should return 403 Forbidden
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/users")
                    .header("authorization", format!("Bearer {operator_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("test operation should succeed: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);

        // 5. Update user role to admin
        let response = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "PATCH",
                    format!("/api/v1/users/{user_id}"),
                    r#"{"role":"admin"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        // 6. Perform a fresh login to fetch new token (the old token was invalidated on role change)
        let login_again = post_json_without_auth(
            app.clone(),
            "/api/v1/auth/login",
            r#"{"username":"test_operator","password":"Password@123"}"#,
        )
        .await;
        assert_eq!(login_again["code"], 0);
        let new_operator_token = login_again["data"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("expected JSON string"))
            .to_owned();

        // Verify that updated user now HAS access to user list (returns 200 OK)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/users")
                    .header("authorization", format!("Bearer {new_operator_token}"))
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("test operation should succeed: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        // 7. Delete user
        let response = app
            .clone()
            .oneshot(
                admin_request_builder(app.clone(), "DELETE", format!("/api/v1/users/{user_id}"))
                    .await,
            )
            .await
            .unwrap_or_else(|error| panic!("test operation should succeed: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    async fn router() -> axum::Router {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        router_with_state(AppState::new(
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
        ))
    }

    async fn router_with_leader_cluster() -> axum::Router {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        router_with_state(AppState::new(
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
                node_id: "tikee-0".to_owned(),
                nodes: 3,
                can_schedule: true,
                leader_fencing_token: Some("raft:term:7:node:tikee-0".to_owned()),
                detail: "test leader with persisted fencing token".to_owned(),
            }),
        ))
    }
}
