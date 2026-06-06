//! Encoded auth-session metadata helpers.
//!
//! Local login state remains an opaque `auth_sessions` row. These helpers encode
//! non-secret session classifications and scope bindings into device metadata so
//! the bearer token itself stays opaque and non-JWT.

use uuid::Uuid;

use super::dto::{AccessScopeBinding, ApiTokenSummary};

const API_TOKEN_DEVICE_PREFIX: &str = "api-token:";
const API_TOKEN_SCOPE_SEPARATOR: &str = ";scopes=";
const SESSION_BINDING_SEPARATOR: &str = ";bindings=";
const OIDC_SESSION_DEVICE_PREFIX: &str = "oidc-session:";

pub(super) fn is_api_token_session(session: &tikeo_storage::AuthSessionSummary) -> bool {
    session
        .device_id
        .as_ref()
        .is_some_and(|value| value.starts_with(API_TOKEN_DEVICE_PREFIX))
}

pub(super) fn api_token_summary(session: tikeo_storage::AuthSessionSummary) -> ApiTokenSummary {
    let scopes = api_token_scopes(&session);
    let scope_bindings = session_scope_bindings(&session);
    ApiTokenSummary {
        id: session.id,
        name: session
            .device_name
            .unwrap_or_else(|| "api-token".to_owned()),
        username: session.username,
        scopes,
        scope_bindings,
        expires_at: session.expires_at,
        created_at: session.created_at,
    }
}

pub(super) fn encode_session_device_id(
    requested: Option<String>,
    bindings: &[AccessScopeBinding],
) -> Option<String> {
    if bindings.is_empty() {
        return requested;
    }
    let base = requested
        .unwrap_or_else(|| format!("{OIDC_SESSION_DEVICE_PREFIX}{}", Uuid::new_v4().as_simple()));
    Some(format!(
        "{base}{SESSION_BINDING_SEPARATOR}{}",
        bindings
            .iter()
            .map(encode_scope_binding)
            .collect::<Vec<_>>()
            .join(",")
    ))
}

pub(super) fn encode_api_token_device_id(
    scopes: &[String],
    bindings: &[AccessScopeBinding],
) -> String {
    let id = format!("{API_TOKEN_DEVICE_PREFIX}{}", Uuid::new_v4().as_simple());
    let scope_suffix = if scopes.is_empty() {
        String::new()
    } else {
        format!("{API_TOKEN_SCOPE_SEPARATOR}{}", scopes.join(","))
    };
    let binding_suffix = if bindings.is_empty() {
        String::new()
    } else {
        format!(
            "{SESSION_BINDING_SEPARATOR}{}",
            bindings
                .iter()
                .map(encode_scope_binding)
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    format!("{id}{scope_suffix}{binding_suffix}")
}

pub(super) fn api_token_scopes(session: &tikeo_storage::AuthSessionSummary) -> Vec<String> {
    let Some(device_id) = session.device_id.as_deref() else {
        return Vec::new();
    };
    let Some((_, tail)) = device_id.split_once(API_TOKEN_SCOPE_SEPARATOR) else {
        return Vec::new();
    };
    let scopes = tail.split_once(';').map_or(tail, |(scopes, _)| scopes);
    scopes
        .split(',')
        .map(str::trim)
        .filter(|scope| !scope.is_empty())
        .map(str::to_owned)
        .collect()
}

pub(super) fn session_scope_bindings(
    session: &tikeo_storage::AuthSessionSummary,
) -> Vec<AccessScopeBinding> {
    let Some(device_id) = session.device_id.as_deref() else {
        return Vec::new();
    };
    let Some((_, bindings)) = device_id.split_once(SESSION_BINDING_SEPARATOR) else {
        return Vec::new();
    };
    bindings
        .split(',')
        .filter_map(decode_scope_binding)
        .collect()
}

fn encode_scope_binding(binding: &AccessScopeBinding) -> String {
    [
        binding.namespace.as_deref().unwrap_or("*"),
        binding.app.as_deref().unwrap_or("*"),
        binding.worker_pool.as_deref().unwrap_or("*"),
    ]
    .join("|")
}

fn decode_scope_binding(value: &str) -> Option<AccessScopeBinding> {
    let mut parts = value.split('|');
    let namespace = decode_scope_binding_part(parts.next()?);
    let app = decode_scope_binding_part(parts.next()?);
    let worker_pool = decode_scope_binding_part(parts.next()?);
    Some(AccessScopeBinding {
        namespace,
        app,
        worker_pool,
    })
}

fn decode_scope_binding_part(value: &str) -> Option<String> {
    if value == "*" || value.trim().is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}
