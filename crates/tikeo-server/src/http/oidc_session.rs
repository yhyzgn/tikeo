//! OIDC callback completion into local opaque tikeo sessions.
//!
//! Provider tokens are used only to fetch external identity. The local login state
//! remains an opaque `auth_sessions` bearer token cached by the session manager.

use axum::http::HeaderMap;
use tikeo_storage::{CreateAuditLog, OidcAuthStateRepository, OidcIdentityRepository};
use tracing::warn;

use super::{
    AppState,
    auth::redact_token_for_audit,
    dto::{AccessScopeBinding, AuthSession},
    error::ApiError,
    oidc::OidcCallbackQuery,
    routes::{client_ip, trace_id},
    session::SessionCreate,
};

/// Complete oidc callback.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub(super) async fn complete_oidc_callback(
    state: &AppState,
    headers: &HeaderMap,
    query: &OidcCallbackQuery,
) -> Result<AuthSession, ApiError> {
    let oidc = &state.auth_config.oidc;
    if !oidc.enabled {
        return Err(ApiError::bad_request("OIDC login is not enabled"));
    }
    reject_provider_error(query)?;

    let oidc_state = consume_oidc_state(state, query).await?;
    let issuer = configured_value(oidc.issuer_url.as_ref(), "auth.oidc.issuer_url")?;
    let client_id = configured_value(oidc.client_id.as_ref(), "auth.oidc.client_id")?;
    let client_secret = configured_value(oidc.client_secret.as_ref(), "auth.oidc.client_secret")?;
    let redirect_uri = callback_redirect_uri(query, &oidc_state.redirect_uri)?;
    let userinfo =
        exchange_and_fetch_userinfo(issuer, client_id, client_secret, redirect_uri, query).await?;
    let mapping = mapped_identity(state, issuer, &userinfo.sub).await?;
    let user = local_user_for_mapping(state, &mapping, &userinfo.sub).await?;
    let session = issue_local_session(state, user.clone(), &mapping).await?;
    audit_oidc_login(
        state,
        headers,
        &user.username,
        issuer,
        &userinfo.sub,
        &mapping.id,
        &session,
    )
    .await;
    Ok(session)
}

fn reject_provider_error(query: &OidcCallbackQuery) -> Result<(), ApiError> {
    if let Some(error) = query
        .error
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        return Err(ApiError::bad_request(format!(
            "OIDC provider returned error: {error}"
        )));
    }
    Ok(())
}

async fn consume_oidc_state(
    state: &AppState,
    query: &OidcCallbackQuery,
) -> Result<tikeo_storage::OidcAuthStateSummary, ApiError> {
    let state_value = configured_value(query.state.as_ref(), "OIDC callback state")?;
    OidcAuthStateRepository::new(state.users.db())
        .consume_state(&super::oidc::hash_state(state_value))
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| {
            ApiError::bad_request("OIDC callback state is invalid, expired, or already used")
        })
}

fn callback_redirect_uri<'a>(
    query: &'a OidcCallbackQuery,
    expected: &'a str,
) -> Result<&'a str, ApiError> {
    let redirect_uri = query
        .redirect_uri
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(expected);
    if redirect_uri != expected {
        return Err(ApiError::bad_request(
            "OIDC callback redirect_uri does not match authorization state",
        ));
    }
    Ok(redirect_uri)
}

async fn exchange_and_fetch_userinfo(
    issuer: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    query: &OidcCallbackQuery,
) -> Result<super::oidc::OidcUserInfo, ApiError> {
    let code = configured_value(query.code.as_ref(), "OIDC callback code")?;
    let token = super::oidc::exchange_authorization_code(
        super::oidc::token_endpoint(issuer)?,
        code,
        redirect_uri,
        client_id,
        client_secret,
    )
    .await?;
    let access_token = configured_value(
        token.access_token.as_ref(),
        "OIDC token response access_token",
    )?;
    let _token_type =
        configured_value(token.token_type.as_ref(), "OIDC token response token_type")?;
    super::oidc::fetch_userinfo(
        super::oidc::discover_userinfo_endpoint(issuer).await?,
        access_token,
    )
    .await
}

async fn mapped_identity(
    state: &AppState,
    issuer: &str,
    subject: &str,
) -> Result<tikeo_storage::OidcIdentitySummary, ApiError> {
    OidcIdentityRepository::new(state.users.db())
        .get_by_issuer_subject(issuer, subject)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| {
            ApiError::bad_request(format!(
                "OIDC external identity subject {subject} has no local session mapping yet; no tikeo session was created"
            ))
        })
}

async fn local_user_for_mapping(
    state: &AppState,
    mapping: &tikeo_storage::OidcIdentitySummary,
    subject: &str,
) -> Result<tikeo_storage::entities::user::Model, ApiError> {
    state
        .users
        .get_by_username(&mapping.username)
        .await
        .map_err(|error| ApiError::storage(&error))?
        .ok_or_else(|| {
            ApiError::bad_request(format!(
                "OIDC external identity subject {subject} maps to missing local user {username}; no tikeo session was created",
                username = mapping.username
            ))
        })
}

async fn issue_local_session(
    state: &AppState,
    user: tikeo_storage::entities::user::Model,
    mapping: &tikeo_storage::OidcIdentitySummary,
) -> Result<AuthSession, ApiError> {
    state
        .sessions
        .create_session(SessionCreate {
            user_id: user.id,
            username: user.username,
            role: user.role,
            device_id: Some(format!("oidc-session:{}", mapping.id)),
            device_name: Some("oidc".to_owned()),
            token_scopes: Vec::new(),
            scope_bindings: oidc_scope_bindings(mapping),
            expires_in_seconds: None,
        })
        .await
}

async fn audit_oidc_login(
    state: &AppState,
    headers: &HeaderMap,
    username: &str,
    issuer: &str,
    subject: &str,
    mapping_id: &str,
    session: &AuthSession,
) {
    let result = state
        .audit
        .append(CreateAuditLog {
            actor: username.to_owned(),
            action: "oidc_login".to_owned(),
            resource_type: "session".to_owned(),
            resource_id: redact_token_for_audit(&session.token),
            detail: Some(format!(
                "issuer={issuer}; subject={subject}; mapping_id={mapping_id}"
            )),
            before: None,
            after: None,
            trace_id: Some(trace_id(headers)),
            result: "success".to_owned(),
            failure_reason: None,
            ip_address: client_ip(headers),
        })
        .await;
    if let Err(error) = result {
        warn!(%error, "failed to append oidc login audit log");
    }
}

fn oidc_scope_bindings(mapping: &tikeo_storage::OidcIdentitySummary) -> Vec<AccessScopeBinding> {
    if mapping.namespace.is_none() && mapping.app.is_none() && mapping.worker_pool.is_none() {
        return Vec::new();
    }
    vec![AccessScopeBinding {
        namespace: mapping.namespace.clone(),
        app: mapping.app.clone(),
        worker_pool: mapping.worker_pool.clone(),
    }]
}

fn configured_value<'a>(value: Option<&'a String>, field: &str) -> Result<&'a str, ApiError> {
    value
        .map(String::as_str)
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request(format!("{field} is required when OIDC is enabled")))
}
