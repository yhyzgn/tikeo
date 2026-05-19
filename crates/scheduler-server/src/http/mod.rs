//! HTTP management gateway for scheduler.

pub mod auth;
pub mod dto;
pub mod error;
pub mod openapi;
pub mod routes;

use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::SystemTime};

use anyhow::{Context, Result};
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use scheduler_core::HealthState;
use scheduler_storage::{
    JobInstanceAttemptRepository, JobInstanceLogRepository, JobInstanceRepository, JobRepository,
    UserRepository, connect_and_migrate,
};
use serde::Serialize;

use tokio::{net::TcpListener, signal, sync::RwLock};
use tracing::info;
use utoipa::OpenApi;

use self::openapi::ApiDoc;

/// Shared HTTP application state.
#[derive(Debug, Clone)]
pub struct AppState {
    started_at: SystemTime,
    jobs: JobRepository,
    instances: JobInstanceRepository,
    logs: JobInstanceLogRepository,
    attempts: JobInstanceAttemptRepository,
    users: UserRepository,
    sessions: Arc<RwLock<HashMap<String, dto::MeResponse>>>,
    registry: crate::tunnel::WorkerRegistry,
}

impl AppState {
    /// Create shared HTTP state.
    #[must_use]
    pub fn new(
        jobs: JobRepository,
        instances: JobInstanceRepository,
        logs: JobInstanceLogRepository,
        attempts: JobInstanceAttemptRepository,
        users: UserRepository,
        registry: crate::tunnel::WorkerRegistry,
    ) -> Self {
        Self {
            started_at: SystemTime::now(),
            jobs,
            instances,
            logs,
            attempts,
            users,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            registry,
        }
    }
}

/// Construct the HTTP router with an explicit application state.
pub fn router_with_state(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .nest("/api/v1", api_router())
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
        UserRepository::new(db),
        crate::tunnel::WorkerRegistry::default(),
    )))
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/system/info", get(routes::system_info))
        .route("/cluster", get(routes::cluster_status))
        .route("/auth/login", axum::routing::post(auth::login))
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", axum::routing::post(auth::logout))
        .route("/users", axum::routing::get(routes::list_users).post(routes::create_user))
        .route("/users/{id}", axum::routing::patch(routes::update_user).delete(routes::delete_user))
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
    use axum::{body::Body, http::Request};
    use scheduler_proto::worker::v1::RegisterWorker;
    use scheduler_storage::{
        AppendJobInstanceLog, JobInstanceAttemptRepository, JobInstanceLogRepository,
        JobInstanceRepository, JobRepository, UserRepository, connect_and_migrate,
    };
    use serde_json::Value;
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
        assert_eq!(login["data"]["token"], "scheduler-init-token");
        assert_eq!(login["data"]["roles"][0], "admin");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("authorization", "Bearer scheduler-init-token")
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
                    .header("authorization", "Bearer scheduler-init-token")
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
            registry,
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
            crate::tunnel::WorkerRegistry::default(),
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

    async fn post_json(app: axum::Router, uri: &str, body: &str) -> Value {
        post_json_with_auth(app, uri, body, true).await
    }

    async fn post_json_without_auth(app: axum::Router, uri: &str, body: &str) -> Value {
        post_json_with_auth(app, uri, body, false).await
    }

    async fn post_json_with_auth(app: axum::Router, uri: &str, body: &str, auth: bool) -> Value {
        let mut builder = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json");
        if auth {
            builder = builder.header("authorization", "Bearer scheduler-init-token");
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

    fn worker(worker_id: &str, app: &str) -> RegisterWorker {
        RegisterWorker {
            worker_id: worker_id.to_owned(),
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
        registry.register(worker("worker-a", "billing"), tx1).await;
        registry
            .register(worker("worker-b", "analytics"), tx2)
            .await;
        let app = router_with_state(AppState::new(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db.clone()),
            registry,
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
        assert_eq!(json["data"]["items"][0]["worker_id"], "worker-a");
    }

    #[tokio::test]
    async fn user_management_and_rbac_integration() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let app = router_with_state(AppState::new(
            JobRepository::new(db.clone()),
            JobInstanceRepository::new(db.clone()),
            JobInstanceLogRepository::new(db.clone()),
            JobInstanceAttemptRepository::new(db.clone()),
            UserRepository::new(db),
            crate::tunnel::WorkerRegistry::default(),
        ));

        // 1. Get users list (should only contain seeded admin)
        let response = app.clone().oneshot(
            Request::builder()
                .uri("/api/v1/users")
                .header("authorization", "Bearer scheduler-init-token")
                .body(Body::empty())
                .unwrap()
        ).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["data"][0]["username"], "admin");

        // 2. Create an operator user
        let response = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header("authorization", "Bearer scheduler-init-token")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"test_operator","password":"Password@123","role":"operator"}"#))
                .unwrap()
        ).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let user_id = json["data"]["id"].as_str().unwrap().to_owned();

        // 3. Authenticate with newly created user
        let login = post_json_without_auth(
            app.clone(),
            "/api/v1/auth/login",
            r#"{"username":"test_operator","password":"Password@123"}"#,
        ).await;
        assert_eq!(login["code"], 0);
        let operator_token = login["data"]["token"].as_str().unwrap().to_owned();

        // 4. Verification: Operator is not allowed to create users (Admin only) -> Should return 403 Forbidden
        let response = app.clone().oneshot(
            Request::builder()
                .uri("/api/v1/users")
                .header("authorization", format!("Bearer {operator_token}"))
                .body(Body::empty())
                .unwrap()
        ).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);

        // 5. Update user role to admin
        let response = app.clone().oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/users/{user_id}"))
                .header("authorization", "Bearer scheduler-init-token")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"role":"admin"}"#))
                .unwrap()
        ).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        // 6. Perform a fresh login to fetch new token (the old token was invalidated on role change)
        let login_again = post_json_without_auth(
            app.clone(),
            "/api/v1/auth/login",
            r#"{"username":"test_operator","password":"Password@123"}"#,
        ).await;
        assert_eq!(login_again["code"], 0);
        let new_operator_token = login_again["data"]["token"].as_str().unwrap().to_owned();

        // Verify that updated user now HAS access to user list (returns 200 OK)
        let response = app.clone().oneshot(
            Request::builder()
                .uri("/api/v1/users")
                .header("authorization", format!("Bearer {new_operator_token}"))
                .body(Body::empty())
                .unwrap()
        ).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        // 7. Delete user
        let response = app.clone().oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/users/{user_id}"))
                .header("authorization", "Bearer scheduler-init-token")
                .body(Body::empty())
                .unwrap()
        ).await.unwrap();
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
            UserRepository::new(db),
            crate::tunnel::WorkerRegistry::default(),
        ))
    }
}
