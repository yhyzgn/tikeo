//! HTTP management gateway for scheduler.

pub mod dto;
pub mod error;
pub mod openapi;
pub mod routes;

use std::{sync::Arc, time::SystemTime};

use anyhow::{Context, Result};
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use scheduler_config::SchedulerConfig;
use scheduler_core::HealthState;
use serde::Serialize;
use tokio::{net::TcpListener, signal};
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use self::openapi::ApiDoc;

/// Shared HTTP application state.
#[derive(Debug)]
pub struct AppState {
    started_at: SystemTime,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            started_at: SystemTime::now(),
        }
    }
}

/// Construct the HTTP router.
pub fn router() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .nest("/api/v1", api_router())
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(Arc::new(AppState::default()))
}

fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/system/info", get(routes::system_info))
        .route("/cluster", get(routes::cluster_status))
        .route("/jobs", get(routes::list_jobs).post(routes::create_job))
}

/// Run the unified HTTP listener.
///
/// # Errors
///
/// Returns an error when binding the configured listener address or serving HTTP fails.
pub async fn serve(config: SchedulerConfig) -> Result<()> {
    let listener = TcpListener::bind(config.server.listen_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.server.listen_addr))?;

    info!(addr = %config.server.listen_addr, "scheduler server listening");

    axum::serve(listener, router())
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
    use serde_json::Value;
    use tower::ServiceExt;

    use super::router;

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
        assert!(json["paths"]["/api/v1/jobs"].is_object());
    }

    #[tokio::test]
    async fn create_job_returns_problem_details_placeholder() {
        let response = router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/jobs")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"nightly"}"#))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"));

        assert_eq!(response.status(), axum::http::StatusCode::NOT_IMPLEMENTED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));

        assert_eq!(json["code"], 10_001);
        assert_eq!(json["message"], "job persistence is not implemented yet");
        assert!(json.get("data").is_some());
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
        router()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("router should respond: {error}"))
    }
}
