//! HTTP management gateway for scheduler.

pub mod auth;
pub mod dto;
pub mod error;
pub mod openapi;
pub mod routes;
pub mod services;
pub mod session;

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
use scheduler_core::HealthState;
use scheduler_storage::{
    AuditLogRepository, AuthSessionRepository, JobInstanceAttemptRepository,
    JobInstanceLogRepository, JobInstanceRepository, JobRepository, RbacRepository,
    ScriptRepository, UserRepository, WorkflowRepository, connect_and_migrate,
};
use serde::Serialize;

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
    sessions: SessionManager,
    pub(crate) rbac: RbacService,
    pub(crate) registry: crate::tunnel::WorkerRegistry,
    pub(crate) cluster: SharedClusterCoordinator,
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
            sessions,
            rbac,
            registry,
            cluster,
        }
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
            api_router().layer(middleware::from_fn(record_http_metrics)),
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
    metrics::counter!("scheduler_http_requests_total", &labels).increment(1);
    metrics::histogram!("scheduler_http_request_duration_seconds", &labels)
        .record(started.elapsed().as_secs_f64());
    response
}

fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/system/info", get(routes::system_info))
        .route("/cluster", get(routes::cluster_status))
        .route("/auth/login", axum::routing::post(auth::login))
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

    info!(addr = %listen_addr, "scheduler HTTP server listening");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("scheduler HTTP server failed")
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
    use crate::cluster::StandaloneCoordinator;
    use axum::{body::Body, http::Request};
    use scheduler_proto::worker::v1::RegisterWorker;
    use scheduler_storage::{
        AppendJobInstanceLog, AuditLogRepository, JobInstanceAttemptRepository,
        JobInstanceLogRepository, JobInstanceRepository, JobRepository, ScriptRepository,
        UserRepository, WorkflowRepository, connect_and_migrate,
    };
    use serde_json::Value;

    const ADMIN_LOGIN: &str = r#"{"username":"scheduler_init","password":"Scheduler@2026!"}"#;
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
    async fn system_info_returns_scheduler_metadata() {
        let json = get_json("/api/v1/system/info").await;

        assert_eq!(json["code"], 0);
        assert_eq!(json["message"], "success");
        assert_eq!(json["data"]["name"], "scheduler");
    }

    #[tokio::test]
    async fn cluster_status_reports_explicit_standalone_role() {
        let json = get_json("/api/v1/cluster").await;

        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["mode"], "standalone");
        assert_eq!(json["data"]["role"], "standalone");
        assert_eq!(json["data"]["nodes"], 1);
        assert_eq!(json["data"]["can_schedule"], true);
    }

    #[tokio::test]
    async fn openapi_json_contains_management_paths() {
        let json = get_json("/api-docs/openapi.json").await;

        assert!(json["paths"]["/api/v1/system/info"].is_object());
        assert!(json["paths"]["/api/v1/auth/login"].is_object());
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
    async fn login_succeeds_and_me_returns_principal() {
        let app = router().await;
        let login = post_json_without_auth(
            app.clone(),
            "/api/v1/auth/login",
            r#"{"username":"scheduler_init","password":"Scheduler@2026!"}"#,
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
        assert_eq!(me["data"]["username"], "scheduler_init");
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
                        r#"{"username":"scheduler_init","password":"wrong"}"#,
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
        JobInstanceLogRepository::new(db)
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
        let logs = request_with(app, &format!("/api/v1/instances/{instance_id}/logs")).await;
        let body = axum::body::to_bytes(logs.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(json["data"]["items"][0]["message"], "hello");
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
            AuditLogRepository::new(db.clone()),
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
            AuditLogRepository::new(db),
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
        assert_eq!(json["data"][0]["username"], "scheduler_init");

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
}
