//! Local tracing utilities for HTTP request correlation.

use std::time::Instant;

use axum::{
    body::{Body, Bytes, to_bytes},
    extract::State,
    http::{HeaderMap, HeaderValue, Request, header},
    middleware::Next,
    response::Response,
};
use tikeo_config::HttpLogConfig;
use tracing::{Instrument, debug, error, info, warn};
use uuid::Uuid;

const TRACE_ID_HEADER: &str = "x-trace-id";
const REDACTED: &str = "<redacted>";

/// Resolve a trace id from incoming headers or create a local deterministic shape.
#[must_use]
/// Resolve trace id.
pub fn resolve_trace_id(headers: &HeaderMap) -> String {
    explicit_trace_header(headers)
        .or_else(|| traceparent_id(headers))
        .unwrap_or_else(|| format!("trc-{}", Uuid::new_v4()))
}

/// HTTP middleware that makes a trace id available to handlers and response readers.
pub async fn trace_http(
    State(config): State<std::sync::Arc<HttpLogConfig>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let trace_id = resolve_trace_id(request.headers());
    let Ok(value) = HeaderValue::from_str(&trace_id) else {
        return next.run(request).await;
    };

    request.headers_mut().insert(TRACE_ID_HEADER, value.clone());
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_owned();
    let query = uri.query().unwrap_or_default().to_owned();
    let request_size = content_length(request.headers());
    let request_detail = RequestDetail::capture(&config, request).await;
    let request_body_size = request_detail.body_size;
    let span = tracing::info_span!(
        "http.request",
        trace_id = %trace_id,
        method = %method,
        path = %path
    );
    info!(
        trace_id = %trace_id,
        method = %method,
        path = %path,
        query = %query,
        request_size,
        request_body_size,
        request_detail = %request_detail.detail_status,
        "HTTP request received"
    );
    request_detail.log(&trace_id, method.as_ref(), &path);
    let request = request_detail.request;

    let started = Instant::now();
    let response = next.run(request).instrument(span).await;
    let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
    response_with_logged_body(
        response,
        value,
        &trace_id,
        method.as_ref(),
        &path,
        latency_ms,
        &config,
    )
    .await
}

async fn response_with_logged_body(
    response: Response,
    trace_header: HeaderValue,
    trace_id: &str,
    method: &str,
    path: &str,
    latency_ms: f64,
    config: &HttpLogConfig,
) -> Response {
    let status = response.status();
    let status_code = status.as_u16();
    let response_size = content_length(response.headers());
    let response_detail = ResponseDetail::capture(config, response).await;
    let response_body_size = response_detail.body_size;
    log_response_summary(&ResponseLogSummary {
        trace_id,
        method,
        path,
        status_code,
        is_client_error: status.is_client_error(),
        is_server_error: status.is_server_error(),
        latency_ms,
        response_size,
        response_body_size,
        response_detail: &response_detail.detail_status,
    });
    response_detail.log(trace_id, method, path, status_code, latency_ms);
    let mut response = response_detail.response;
    response.headers_mut().insert(TRACE_ID_HEADER, trace_header);
    response
}

struct ResponseLogSummary<'a> {
    trace_id: &'a str,
    method: &'a str,
    path: &'a str,
    status_code: u16,
    is_client_error: bool,
    is_server_error: bool,
    latency_ms: f64,
    response_size: Option<u64>,
    response_body_size: Option<usize>,
    response_detail: &'a str,
}

fn log_response_summary(summary: &ResponseLogSummary<'_>) {
    let trace_id = summary.trace_id;
    let method = summary.method;
    let path = summary.path;
    let status_code = summary.status_code;
    let latency_ms = summary.latency_ms;
    let response_size = summary.response_size;
    let response_body_size = summary.response_body_size;
    let response_detail = summary.response_detail;
    if summary.is_server_error {
        error!(
            trace_id,
            method,
            path,
            status = status_code,
            latency_ms,
            response_size,
            response_body_size,
            response_detail,
            "HTTP request completed with server error"
        );
    } else if summary.is_client_error {
        warn!(
            trace_id,
            method,
            path,
            status = status_code,
            latency_ms,
            response_size,
            response_body_size,
            response_detail,
            "HTTP request completed with client error"
        );
    } else {
        info!(
            trace_id,
            method,
            path,
            status = status_code,
            latency_ms,
            response_size,
            response_body_size,
            response_detail,
            "HTTP request completed"
        );
    }
}

struct RequestDetail {
    request: Request<Body>,
    body_size: Option<usize>,
    headers: Option<String>,
    body: Option<String>,
    detail_status: String,
}

impl RequestDetail {
    async fn capture(config: &HttpLogConfig, request: Request<Body>) -> Self {
        let headers = config
            .include_headers
            .then(|| format_headers(request.headers()));
        if !config.include_body {
            return Self {
                request,
                body_size: None,
                headers,
                body: None,
                detail_status: "body-disabled".to_owned(),
            };
        }
        if body_should_skip(request.headers()) {
            return Self {
                request,
                body_size: None,
                headers,
                body: Some("<body skipped: streaming or binary content>".to_owned()),
                detail_status: "body-skipped".to_owned(),
            };
        }
        let (parts, body) = request.into_parts();
        let bytes = match to_bytes(body, config.max_body_bytes).await {
            Ok(bytes) => bytes,
            Err(error) => {
                let request = Request::from_parts(parts, Body::empty());
                return Self {
                    request,
                    body_size: None,
                    headers,
                    body: Some(format!("<body read failed: {error}>")),
                    detail_status: "body-read-failed".to_owned(),
                };
            }
        };
        let body_size = bytes.len();
        let body_text = body_text(&bytes);
        let request = Request::from_parts(parts, Body::from(bytes));
        Self {
            request,
            body_size: Some(body_size),
            headers,
            body: Some(body_text),
            detail_status: "body-captured".to_owned(),
        }
    }

    fn log(&self, trace_id: &str, method: &str, path: &str) {
        if self.headers.is_some() || self.body.is_some() {
            debug!(
                trace_id,
                method,
                path,
                request_headers = %self.headers.as_deref().unwrap_or_default(),
                request_body = %self.body.as_deref().unwrap_or_default(),
                "HTTP request detail"
            );
        }
    }
}

struct ResponseDetail {
    response: Response,
    body_size: Option<usize>,
    headers: Option<String>,
    body: Option<String>,
    detail_status: String,
}

impl ResponseDetail {
    async fn capture(config: &HttpLogConfig, response: Response) -> Self {
        let headers = config
            .include_headers
            .then(|| format_headers(response.headers()));
        if !config.include_body {
            return Self {
                response,
                body_size: None,
                headers,
                body: None,
                detail_status: "body-disabled".to_owned(),
            };
        }
        if body_should_skip(response.headers()) {
            return Self {
                response,
                body_size: None,
                headers,
                body: Some("<body skipped: streaming or binary content>".to_owned()),
                detail_status: "body-skipped".to_owned(),
            };
        }
        let (parts, body) = response.into_parts();
        let bytes = match to_bytes(body, config.max_body_bytes).await {
            Ok(bytes) => bytes,
            Err(error) => {
                let response = Response::from_parts(parts, Body::empty());
                return Self {
                    response,
                    body_size: None,
                    headers,
                    body: Some(format!("<body read failed: {error}>")),
                    detail_status: "body-read-failed".to_owned(),
                };
            }
        };
        let body_size = bytes.len();
        let body_text = body_text(&bytes);
        let response = Response::from_parts(parts, Body::from(bytes));
        Self {
            response,
            body_size: Some(body_size),
            headers,
            body: Some(body_text),
            detail_status: "body-captured".to_owned(),
        }
    }

    fn log(&self, trace_id: &str, method: &str, path: &str, status: u16, latency_ms: f64) {
        if self.headers.is_some() || self.body.is_some() {
            debug!(
                trace_id,
                method,
                path,
                status,
                latency_ms,
                response_headers = %self.headers.as_deref().unwrap_or_default(),
                response_body = %self.body.as_deref().unwrap_or_default(),
                "HTTP response detail"
            );
        }
    }
}

fn content_length(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(header::CONTENT_LENGTH)
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
}

fn body_should_skip(headers: &HeaderMap) -> bool {
    if headers.get(header::CONTENT_LENGTH).is_none()
        && headers
            .get(header::TRANSFER_ENCODING)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.to_ascii_lowercase().contains("chunked"))
    {
        return true;
    }
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            let value = value.to_ascii_lowercase();
            value.contains("text/event-stream")
                || value.contains("octet-stream")
                || value.contains("multipart/")
                || value.contains("application/grpc")
        })
}

fn format_headers(headers: &HeaderMap) -> String {
    headers
        .iter()
        .map(|(name, value)| {
            let value = if sensitive_header(name.as_str()) {
                REDACTED.to_owned()
            } else {
                value.to_str().map_or_else(
                    |_| format!("<{} binary bytes>", value.as_bytes().len()),
                    ToOwned::to_owned,
                )
            };
            format!("{name}: {value}")
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn sensitive_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization"
            | "proxy-authorization"
            | "cookie"
            | "set-cookie"
            | "x-api-key"
            | "x-auth-token"
    )
}

fn body_text(bytes: &Bytes) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| format!("<{} binary bytes>", bytes.len()))
}

fn explicit_trace_header(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .or_else(|| headers.get(TRACE_ID_HEADER))
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn traceparent_id(headers: &HeaderMap) -> Option<String> {
    let value = headers.get("traceparent")?.to_str().ok()?.trim();
    let mut parts = value.split('-');
    let version = parts.next()?;
    let trace_id = parts.next()?;
    let parent_id = parts.next()?;
    let flags = parts.next()?;
    if parts.next().is_some()
        || version.len() != 2
        || trace_id.len() != 32
        || parent_id.len() != 16
        || flags.len() != 2
        || trace_id.chars().all(|item| item == '0')
        || !trace_id.chars().all(|item| item.is_ascii_hexdigit())
    {
        return None;
    }
    Some(trace_id.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::{
        Router,
        body::Body,
        http::{HeaderMap, HeaderValue, Request},
        middleware,
        routing::{get, post},
    };
    use tikeo_config::HttpLogConfig;
    use tower::ServiceExt as _;
    use tracing_subscriber::{Layer as _, fmt::MakeWriter, layer::SubscriberExt as _};

    use super::resolve_trace_id;

    #[derive(Clone, Default)]
    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl<'a> MakeWriter<'a> for SharedWriter {
        type Writer = SharedBuffer;

        fn make_writer(&'a self) -> Self::Writer {
            SharedBuffer(self.0.clone())
        }
    }

    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for SharedBuffer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn capture_logs() -> (Arc<Mutex<Vec<u8>>>, tracing::dispatcher::DefaultGuard) {
        let writer = SharedWriter::default();
        let captured = writer.0.clone();
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(writer)
                .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG),
        );
        (captured, tracing::subscriber::set_default(subscriber))
    }

    fn captured_text(captured: &Arc<Mutex<Vec<u8>>>) -> String {
        String::from_utf8(
            captured
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
        )
        .unwrap_or_else(|error| panic!("logs should be utf8: {error}"))
    }

    #[test]
    fn resolve_trace_id_prefers_request_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static("trace-a"));

        assert_eq!(resolve_trace_id(&headers), "trace-a");
    }

    #[test]
    fn resolve_trace_id_reads_w3c_traceparent() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "traceparent",
            HeaderValue::from_static("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"),
        );

        assert_eq!(
            resolve_trace_id(&headers),
            "4bf92f3577b34da6a3ce929d0e0e4736"
        );
    }

    #[tokio::test]
    async fn trace_http_defaults_to_summary_without_full_exchange_details() {
        let (captured, _guard) = capture_logs();
        let app = Router::new()
            .route("/echo", post(|body: String| async move { body }))
            .layer(middleware::from_fn_with_state(
                Arc::new(HttpLogConfig::default()),
                super::trace_http,
            ));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/echo")
                    .header("x-request-id", "trace-summary")
                    .header("authorization", "Bearer secret-token")
                    .body(Body::from(r#"{"hello":"world"}"#))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("request should complete: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let logs = captured_text(&captured);
        assert!(logs.contains("HTTP request received"));
        assert!(logs.contains("HTTP request completed"));
        assert!(logs.contains("latency_ms"));
        assert!(!logs.contains("request_headers"));
        assert!(!logs.contains("request_body"));
        assert!(!logs.contains("secret-token"));
    }

    #[tokio::test]
    async fn trace_http_logs_full_request_and_response_details_when_enabled() {
        let (captured, _guard) = capture_logs();
        let config = HttpLogConfig {
            include_headers: true,
            include_body: true,
            max_body_bytes: 64 * 1024,
        };
        let app = Router::new()
            .route(
                "/echo",
                post(|body: String| async move {
                    (
                        [
                            ("x-response-demo", "response-header-value"),
                            ("content-type", "application/json"),
                        ],
                        format!(r#"{{"echo":{body:?}}}"#),
                    )
                }),
            )
            .layer(middleware::from_fn_with_state(
                Arc::new(config),
                super::trace_http,
            ));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/echo?debug=true")
                    .header("x-request-id", "trace-full-http")
                    .header("x-demo", "request-header-value")
                    .header("authorization", "Bearer secret-token")
                    .body(Body::from(r#"{"hello":"world"}"#))
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("request should complete: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should read: {error}"));
        assert!(String::from_utf8_lossy(&body).contains("hello"));
        let logs = captured_text(&captured);
        assert!(logs.contains("HTTP request detail"));
        assert!(logs.contains("HTTP response detail"));
        assert!(logs.contains("trace-full-http"));
        assert!(logs.contains("x-demo: request-header-value"));
        assert!(logs.contains("authorization: <redacted>"));
        assert!(!logs.contains("secret-token"));
        assert!(logs.contains("\\\"hello\\\":\\\"world\\\""));
        assert!(logs.contains("x-response-demo: response-header-value"));
        assert!(logs.contains("\\\"echo\\\""));
        assert!(logs.contains("latency_ms"));
    }

    #[tokio::test]
    async fn trace_http_skips_streaming_or_binary_body_capture() {
        let (captured, _guard) = capture_logs();
        let config = HttpLogConfig {
            include_headers: true,
            include_body: true,
            max_body_bytes: 64 * 1024,
        };
        let app = Router::new()
            .route(
                "/events",
                get(|| async { ([("content-type", "text/event-stream")], "data: hello\n\n") }),
            )
            .layer(middleware::from_fn_with_state(
                Arc::new(config),
                super::trace_http,
            ));
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/events")
                    .header("x-request-id", "trace-stream")
                    .body(Body::empty())
                    .unwrap_or_else(|error| panic!("request should build: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("request should complete: {error}"));
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let logs = captured_text(&captured);
        assert!(logs.contains("body skipped: streaming or binary content"));
        assert!(logs.contains("response_detail"));
        assert!(logs.contains("body-skipped"));
    }
}
