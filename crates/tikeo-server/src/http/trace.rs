//! Local tracing utilities for HTTP request correlation.

use std::time::Instant;

use axum::{
    body::Body,
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
    if let Ok(value) = HeaderValue::from_str(&trace_id) {
        request.headers_mut().insert(TRACE_ID_HEADER, value.clone());
        let method = request.method().clone();
        let path = request.uri().path().to_owned();
        let query_present = request.uri().query().is_some();
        let user_agent = request
            .headers()
            .get("user-agent")
            .and_then(|header| header.to_str().ok())
            .unwrap_or("-")
            .to_owned();
        let request_size = request
            .headers()
            .get("content-length")
            .and_then(|header| header.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
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
            query_present,
            request_size,
            user_agent = %user_agent,
            "HTTP request received"
        );
        let started = Instant::now();
        let mut response = next.run(request).instrument(span).await;
        let status = response.status();
        let status_code = status.as_u16();
        let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
        let response_size = response
            .headers()
            .get("content-length")
            .and_then(|header| header.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        if status.is_server_error() {
            error!(
                trace_id = %trace_id,
                method = %method,
                path = %path,
                status = status_code,
                latency_ms,
                response_size,
                "HTTP request completed with server error"
            );
        } else if status.is_client_error() {
            warn!(
                trace_id = %trace_id,
                method = %method,
                path = %path,
                status = status_code,
                latency_ms,
                response_size,
                "HTTP request completed with client error"
            );
        } else {
            info!(
                trace_id = %trace_id,
                method = %method,
                path = %path,
                status = status_code,
                latency_ms,
                response_size,
                "HTTP request completed"
            );
        }
        response.headers_mut().insert(TRACE_ID_HEADER, value);
        response
    } else {
        next.run(request).await
    }
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
}
