use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use sha2::{Digest, Sha256};
use tikee_core::{ExecutionMode, TriggerType};
use tikee_storage::{AppendJobInstanceLog, CreateAuditLog, CreateJobInstance};

use crate::http::{
    AppState, auth,
    dto::{
        ApiResponse, ErrorResponse, InboundWebhookTriggerApiResponse, InboundWebhookTriggerRequest,
        InboundWebhookTriggerResponse,
    },
    error::ApiError,
};

/// Trigger a job from an inbound webhook/event-source payload.
///
/// # Errors
///
/// Returns authorization, not-found, or storage errors when trigger creation fails.
#[utoipa::path(
    post,
    path = "/api/v1/events/webhooks/{job}:trigger",
    tag = "event-sources",
    params(("job" = String, Path, description = "Job identifier")),
    request_body = InboundWebhookTriggerRequest,
    responses(
        (status = 200, description = "Accepted inbound webhook event", body = InboundWebhookTriggerApiResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    )
)]
pub async fn trigger_inbound_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job_action): Path<String>,
    Json(request): Json<InboundWebhookTriggerRequest>,
) -> Result<Json<InboundWebhookTriggerApiResponse>, ApiError> {
    let principal = auth::require_permission(&headers, &state, "instances", "execute").await?;
    let job = job_action
        .strip_suffix(":trigger")
        .filter(|job| !job.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            ApiError::not_found(format!("unsupported webhook job action: {job_action}"))
        })?;
    verify_webhook_signature(&state, &headers, &request, &job, &principal.username).await?;
    let job_summary = state
        .jobs
        .get(&job)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;
    if !crate::http::access_scope::allows_resource(
        &principal.scope_bindings,
        &job_summary.namespace,
        &job_summary.app,
        None,
    ) {
        return Err(ApiError::forbidden(
            "api token scope binding does not allow this namespace/app",
        ));
    }
    let instance = state
        .instances
        .create_pending(CreateJobInstance {
            job_id: job.clone(),
            trigger_type: TriggerType::Webhook,
            execution_mode: ExecutionMode::Single,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| ApiError::not_found(format!("job not found: {job}")))?;

    append_webhook_log(&state, &instance.id, &request).await?;

    Ok(Json(ApiResponse::success(InboundWebhookTriggerResponse {
        accepted: true,
        instance_id: instance.id,
        job_id: instance.job_id,
        status: instance.status.to_string(),
        trigger_type: instance.trigger_type.to_string(),
    })))
}

async fn verify_webhook_signature(
    state: &AppState,
    headers: &HeaderMap,
    request: &InboundWebhookTriggerRequest,
    job_id: &str,
    actor: &str,
) -> Result<(), ApiError> {
    let secret_ref = request
        .secret_ref
        .as_deref()
        .or_else(|| header_str(headers, "x-tikee-webhook-secret-ref"));
    let signature = request
        .signature
        .as_deref()
        .or_else(|| header_str(headers, "x-tikee-webhook-signature"));
    let timestamp = request.timestamp.or_else(|| {
        header_str(headers, "x-tikee-webhook-timestamp").and_then(|value| value.parse().ok())
    });
    let nonce = request
        .nonce
        .as_deref()
        .or_else(|| header_str(headers, "x-tikee-webhook-nonce"));
    if secret_ref.is_none() && signature.is_none() && timestamp.is_none() && nonce.is_none() {
        return Ok(());
    }
    let Some(secret_ref) = secret_ref else {
        append_webhook_audit(
            state,
            actor,
            job_id,
            "failed",
            Some("webhook_signature_secret_ref_required"),
            headers,
        )
        .await;
        return Err(ApiError::bad_request(
            "webhook signature requires secretRef",
        ));
    };
    let Some(signature) = signature.map(str::trim).filter(|value| !value.is_empty()) else {
        append_webhook_audit(
            state,
            actor,
            job_id,
            "failed",
            Some("webhook_signature_required"),
            headers,
        )
        .await;
        return Err(ApiError::bad_request("webhook signature is required"));
    };
    let Some(timestamp) = timestamp else {
        append_webhook_audit(
            state,
            actor,
            job_id,
            "failed",
            Some("webhook_timestamp_required"),
            headers,
        )
        .await;
        return Err(ApiError::bad_request("webhook timestamp is required"));
    };
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    if (now - timestamp).abs() > 300 {
        append_webhook_audit(
            state,
            actor,
            job_id,
            "failed",
            Some("webhook_timestamp_out_of_window"),
            headers,
        )
        .await;
        return Err(ApiError::bad_request(
            "webhook timestamp is outside replay window",
        ));
    }
    let Some(nonce) = nonce.map(str::trim).filter(|value| !value.is_empty()) else {
        append_webhook_audit(
            state,
            actor,
            job_id,
            "failed",
            Some("webhook_nonce_required"),
            headers,
        )
        .await;
        return Err(ApiError::bad_request("webhook nonce is required"));
    };
    if nonce_was_used(state, job_id, nonce).await? {
        append_webhook_audit(
            state,
            actor,
            job_id,
            "failed",
            Some("webhook_replay_detected"),
            headers,
        )
        .await;
        return Err(ApiError::bad_request("webhook nonce was already used"));
    }
    let Some(secret) = resolve_env_secret_ref(secret_ref) else {
        append_webhook_audit(
            state,
            actor,
            job_id,
            "failed",
            Some("webhook_signature_secret_unresolved"),
            headers,
        )
        .await;
        return Err(ApiError::bad_request("webhook secretRef is not resolvable"));
    };
    let expected = webhook_signature(&secret, job_id, timestamp, nonce, request.payload.as_ref());
    if !constant_time_eq(signature.as_bytes(), expected.as_bytes()) {
        append_webhook_audit(
            state,
            actor,
            job_id,
            "failed",
            Some("webhook_signature_verification_failed"),
            headers,
        )
        .await;
        return Err(ApiError::bad_request(
            "webhook signature verification failed",
        ));
    }
    append_webhook_audit(
        state,
        actor,
        job_id,
        "success",
        Some(&format!("webhook_nonce={nonce}")),
        headers,
    )
    .await;
    Ok(())
}

async fn nonce_was_used(state: &AppState, job_id: &str, nonce: &str) -> Result<bool, ApiError> {
    let page = state
        .audit
        .list(tikee_storage::AuditLogFilters {
            action: Some("webhook-verify".to_owned()),
            resource_type: Some("job".to_owned()),
            resource_id: Some(job_id.to_owned()),
            ..Default::default()
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(page.iter().any(|item| {
        item.result == "success"
            && item
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains(&format!("webhook_nonce={nonce}")))
    }))
}

async fn append_webhook_audit(
    state: &AppState,
    actor: &str,
    job_id: &str,
    result: &str,
    reason_or_detail: Option<&str>,
    headers: &HeaderMap,
) {
    let _ = state
        .audit
        .append(CreateAuditLog {
            actor: actor.to_owned(),
            action: "webhook-verify".to_owned(),
            resource_type: "job".to_owned(),
            resource_id: job_id.to_owned(),
            detail: reason_or_detail.map(ToOwned::to_owned),
            before: None,
            after: None,
            trace_id: header_str(headers, "x-request-id").map(ToOwned::to_owned),
            result: result.to_owned(),
            failure_reason: if result == "failed" {
                reason_or_detail.map(ToOwned::to_owned)
            } else {
                None
            },
            ip_address: header_str(headers, "x-forwarded-for").map(ToOwned::to_owned),
        })
        .await;
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

fn resolve_env_secret_ref(secret_ref: &str) -> Option<String> {
    secret_ref
        .strip_prefix("env:")
        .and_then(|name| std::env::var(name).ok())
        .filter(|value| !value.is_empty())
}

fn webhook_signature(
    secret: &str,
    job_id: &str,
    timestamp: i64,
    nonce: &str,
    payload: Option<&serde_json::Value>,
) -> String {
    let payload = payload.map_or_else(|| "null".to_owned(), canonical_json);
    let canonical = format!(
        "tikee-webhook-v1\njob_id={job_id}\ntimestamp={timestamp}\nnonce={nonce}\npayload={payload}"
    );
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(b"\n");
    hasher.update(canonical.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

fn canonical_json(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_owned())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0_u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

async fn append_webhook_log(
    state: &AppState,
    instance_id: &str,
    request: &InboundWebhookTriggerRequest,
) -> Result<(), ApiError> {
    let source = request
        .source
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("webhook");
    let event_type = request
        .event_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("webhook.event");
    let payload = request.payload.clone().unwrap_or(serde_json::Value::Null);
    let message = serde_json::json!({
        "event": "webhook_event_source",
        "source": source,
        "event_type": event_type,
        "payload": payload,
        "signed": request.signature.as_ref().is_some_and(|value| !value.trim().is_empty()),
    });
    state
        .logs
        .append(AppendJobInstanceLog {
            instance_id: instance_id.to_owned(),
            worker_id: format!("event-source:{source}"),
            level: "info".to_owned(),
            message: message.to_string(),
            sequence: 0,
        })
        .await
        .map_err(|error| ApiError::storage(&error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::webhook_signature;

    #[test]
    fn webhook_signature_is_stable() {
        let payload = serde_json::json!({"sha":"abc123"});
        let signature =
            webhook_signature("secret", "job-1", 1_700_000_000, "nonce-1", Some(&payload));
        assert_eq!(
            signature,
            "sha256:c7ed68481e552835d570c3ea02f434bb1b4e4a0076e94d8e704ab4bc46e475b2"
        );
    }
}
