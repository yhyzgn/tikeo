//! OTLP exporter smoke coverage for the OpenTelemetry runtime.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use anyhow::{Context, Result};
use axum::{Router, body::Bytes, extract::State, http::HeaderMap, routing::post};
use tokio::{net::TcpListener, sync::oneshot};

#[derive(Clone)]
struct CollectorState {
    hits: Arc<AtomicUsize>,
    headers_tx: Arc<Mutex<Option<oneshot::Sender<HeaderMap>>>>,
}

async fn collect_traces(
    State(state): State<CollectorState>,
    headers: HeaderMap,
    body: Bytes,
) -> axum::http::StatusCode {
    if body.is_empty() {
        return axum::http::StatusCode::BAD_REQUEST;
    }
    state.hits.fetch_add(1, Ordering::SeqCst);
    let sender = state
        .headers_tx
        .lock()
        .map_or(None, |mut headers_tx| headers_tx.take());
    if let Some(sender) = sender {
        let _ = sender.send(headers);
    }
    axum::http::StatusCode::OK
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn otlp_http_exporter_posts_spans_to_local_collector() -> Result<()> {
    let hits = Arc::new(AtomicUsize::new(0));
    let (headers_tx, headers_rx) = oneshot::channel::<HeaderMap>();
    let app = Router::new()
        .route("/v1/traces", post(collect_traces))
        .with_state(CollectorState {
            hits: hits.clone(),
            headers_tx: Arc::new(Mutex::new(Some(headers_tx))),
        });
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("bind collector")?;
    let endpoint = format!(
        "http://{}/v1/traces",
        listener.local_addr().context("collector addr")?
    );
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let mut tracing =
        tikee_server::observability::tracing::TracingRuntime::start(&tikee_config::TracingConfig {
            enabled: true,
            otlp_endpoint: Some(endpoint),
            headers: vec!["x-tikee-tenant".to_owned()],
        })
        .context("tracing runtime should start")?;

    tracing
        .emit_smoke_span("tikee.otel.smoke")
        .context("smoke span should export")?;

    let headers = tokio::time::timeout(std::time::Duration::from_secs(5), headers_rx)
        .await
        .context("collector should receive an OTLP request")?
        .context("collector should send headers")?;
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert_eq!(
        headers
            .get("x-tikee-tenant")
            .and_then(|value| value.to_str().ok()),
        Some("configured")
    );
    tokio::task::spawn_blocking(move || tracing.shutdown())
        .await
        .context("shutdown task should join")?
        .context("tracing shutdown should flush")?;
    server.abort();
    Ok(())
}
