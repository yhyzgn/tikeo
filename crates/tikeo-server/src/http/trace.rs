//! Local tracing utilities for HTTP request correlation.

use std::time::Instant;

use axum::{
    body::{Body, Bytes, to_bytes},
    http::{HeaderMap, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use tracing::{Instrument, error, info, warn};
use uuid::Uuid;

const TRACE_ID_HEADER: &str = "x-trace-id";

/// Resolve a trace id from incoming headers or create a local deterministic shape.
#[must_use]
/// Resolve trace id.
pub fn resolve_trace_id(headers: &HeaderMap) -> String {
    explicit_trace_header(headers)
        .or_else(|| traceparent_id(headers))
        .unwrap_or_else(|| format!("trc-{}", Uuid::new_v4()))
}

/// HTTP middleware that makes a trace id available to handlers and response readers.
pub async fn trace_http(mut request: Request<Body>, next: Next) -> Response {
    let trace_id = resolve_trace_id(request.headers());
    let Ok(value) = HeaderValue::from_str(&trace_id) else {
        return next.run(request).await;
    };

    request.headers_mut().insert(TRACE_ID_HEADER, value.clone());
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_owned();
    let query = uri.query().unwrap_or_default().to_owned();
    let request_headers = format_headers(request.headers());
    let request_size = request
        .headers()
        .get("content-length")
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let (request_parts, request_body) = request.into_parts();
    let request_bytes = match to_bytes(request_body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(error) => {
            error!(
                trace_id = %trace_id,
                method = %method,
                path = %path,
                %error,
                "HTTP request body read failed"
            );
            Bytes::new()
        }
    };
    let request_body_text = body_text(&request_bytes);
    let request_body_size = request_bytes.len();
    let request = Request::from_parts(request_parts, Body::from(request_bytes));
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
        request_headers = %request_headers,
        request_body = %request_body_text,
        "HTTP request received"
    );

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
) -> Response {
    let status = response.status();
    let status_code = status.as_u16();
    let response_headers = format_headers(response.headers());
    let response_size = response
        .headers()
        .get("content-length")
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let (mut parts, body) = response.into_parts();
    let response_bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(error) => {
            error!(
                trace_id,
                method,
                path,
                status = status_code,
                latency_ms,
                %error,
                "HTTP response body read failed"
            );
            Bytes::new()
        }
    };
    let response_body_text = body_text(&response_bytes);
    let response_body_size = response_bytes.len();
    if status.is_server_error() {
        error!(
            trace_id,
            method,
            path,
            status = status_code,
            latency_ms,
            response_size,
            response_body_size,
            response_headers = %response_headers,
            response_body = %response_body_text,
            "HTTP request completed with server error"
        );
    } else if status.is_client_error() {
        warn!(
            trace_id,
            method,
            path,
            status = status_code,
            latency_ms,
            response_size,
            response_body_size,
            response_headers = %response_headers,
            response_body = %response_body_text,
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
            response_headers = %response_headers,
            response_body = %response_body_text,
            "HTTP request completed"
        );
    }
    parts.headers.insert(TRACE_ID_HEADER, trace_header);
    Response::from_parts(parts, Body::from(response_bytes))
}

fn format_headers(headers: &HeaderMap) -> String {
    headers
        .iter()
        .map(|(name, value)| {
            let value = value.to_str().map_or_else(
                |_| format!("<{} binary bytes>", value.as_bytes().len()),
                ToOwned::to_owned,
            );
            format!("{name}: {value}")
        })
        .collect::<Vec<_>>()
        .join("; ")
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
    use axum::http::{HeaderMap, HeaderValue};

    use super::resolve_trace_id;

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
    async fn trace_http_logs_full_request_and_response_details() {
        use std::{
            sync::{Arc, Mutex},
            time::Duration,
        };

        use axum::{Router, body::Body, http::Request, middleware, routing::post};
        use tower::ServiceExt as _;
        use tracing_subscriber::{Layer as _, fmt::MakeWriter, layer::SubscriberExt as _};

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

        let writer = SharedWriter::default();
        let captured = writer.0.clone();
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(writer)
                .with_filter(tracing_subscriber::filter::LevelFilter::INFO),
        );
        let _guard = tracing::subscriber::set_default(subscriber);
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
            .layer(middleware::from_fn(super::trace_http));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/echo?debug=true")
                    .header("x-request-id", "trace-full-http")
                    .header("x-demo", "request-header-value")
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

        tokio::time::sleep(Duration::from_millis(10)).await;
        let logs = String::from_utf8(
            captured
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
        )
        .unwrap_or_else(|error| panic!("logs should be utf8: {error}"));
        assert!(logs.contains("HTTP request received"));
        assert!(logs.contains("HTTP request completed"));
        assert!(logs.contains("trace-full-http"));
        assert!(logs.contains("x-demo: request-header-value"));
        assert!(logs.contains("\\\"hello\\\":\\\"world\\\""));
        assert!(logs.contains("x-response-demo: response-header-value"));
        assert!(logs.contains("\\\"echo\\\""));
        assert!(logs.contains("latency_ms"));
    }
}
