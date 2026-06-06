//! OIDC authorization-code token exchange helpers.
//!
//! This module owns the network boundary to the configured `IdP`. tikeo never
//! uses provider tokens as local login state: successful external identity mapping
//! must still issue an opaque tikeo session persisted in `auth_sessions` and cached
//! through moka.

use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

use super::error::ApiError;

/// Query parameters used to build an OIDC authorization URL.
#[derive(Debug, Deserialize)]
pub struct OidcAuthorizeQuery {
    /// Optional UI callback URL.
    pub redirect_uri: Option<String>,
}

/// Query parameters received by the OIDC callback.
#[derive(Debug, Deserialize)]
pub struct OidcCallbackQuery {
    /// Authorization code returned by the provider.
    pub code: Option<String>,
    /// CSRF state returned by the provider.
    pub state: Option<String>,
    /// Optional redirect URI echoed for local callback tests.
    pub redirect_uri: Option<String>,
    /// Provider-side error code, when authorization failed.
    pub error: Option<String>,
}

/// Minimal token response data needed to call the provider user-info endpoint.
#[derive(Debug, Deserialize)]
pub struct OidcTokenResponse {
    /// Provider access token used only for the upstream user-info request.
    pub access_token: Option<String>,
    /// Provider token type, usually `Bearer`.
    pub token_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OidcDiscoveryDocument {
    userinfo_endpoint: Option<String>,
}

/// Minimal external identity data returned by the provider user-info endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct OidcUserInfo {
    /// Stable provider subject identifier.
    pub sub: String,
    /// Optional preferred username.
    pub preferred_username: Option<String>,
    /// Optional email address.
    pub email: Option<String>,
}

/// Build the configured provider token endpoint URL.
///
/// # Errors
///
/// Returns a bad request when the issuer URL is malformed.
pub fn token_endpoint(issuer: &str) -> Result<Url, ApiError> {
    Url::parse(&format!(
        "{}/protocol/openid-connect/token",
        issuer.trim_end_matches('/')
    ))
    .map_err(|_| ApiError::bad_request("auth.oidc.issuer_url must be a valid URL"))
}

/// Exchange an authorization code for provider token response data.
///
/// # Errors
///
/// Returns a bad request if the `IdP` request fails, rejects the code, or returns invalid JSON.
pub async fn exchange_authorization_code(
    token_url: Url,
    code: &str,
    redirect_uri: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<OidcTokenResponse, ApiError> {
    let response = reqwest::Client::new()
        .post(token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ])
        .send()
        .await
        .map_err(|error| ApiError::bad_request(format!("OIDC token exchange failed: {error}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::bad_request(format!(
            "OIDC token exchange rejected with status {status}"
        )));
    }
    response
        .json::<OidcTokenResponse>()
        .await
        .map_err(|error| ApiError::bad_request(format!("OIDC token response is invalid: {error}")))
}

/// Discover the provider user-info endpoint from `OpenID` Provider Configuration.
///
/// # Errors
///
/// Returns a bad request when discovery fails or the provider omits `userinfo_endpoint`.
pub async fn discover_userinfo_endpoint(issuer: &str) -> Result<Url, ApiError> {
    let discovery_url = Url::parse(&format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/')
    ))
    .map_err(|_| ApiError::bad_request("auth.oidc.issuer_url must be a valid URL"))?;
    let response = reqwest::Client::new()
        .get(discovery_url)
        .send()
        .await
        .map_err(|error| ApiError::bad_request(format!("OIDC discovery failed: {error}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::bad_request(format!(
            "OIDC discovery rejected with status {status}"
        )));
    }
    let discovery = response
        .json::<OidcDiscoveryDocument>()
        .await
        .map_err(|error| {
            ApiError::bad_request(format!("OIDC discovery response is invalid: {error}"))
        })?;
    let endpoint = discovery
        .userinfo_endpoint
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            ApiError::bad_request("OIDC discovery response missing userinfo_endpoint")
        })?;
    Url::parse(endpoint)
        .map_err(|_| ApiError::bad_request("OIDC discovery userinfo_endpoint must be a valid URL"))
}

/// Fetch external identity attributes from the provider user-info endpoint.
///
/// # Errors
///
/// Returns a bad request when user-info cannot be fetched or omits `sub`.
pub async fn fetch_userinfo(
    userinfo_endpoint: Url,
    access_token: &str,
) -> Result<OidcUserInfo, ApiError> {
    let response = reqwest::Client::new()
        .get(userinfo_endpoint)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| ApiError::bad_request(format!("OIDC userinfo fetch failed: {error}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::bad_request(format!(
            "OIDC userinfo fetch rejected with status {status}"
        )));
    }
    let userinfo = response.json::<OidcUserInfo>().await.map_err(|error| {
        ApiError::bad_request(format!("OIDC userinfo response is invalid: {error}"))
    })?;
    if userinfo.sub.trim().is_empty() {
        return Err(ApiError::bad_request("OIDC userinfo response missing sub"));
    }
    Ok(userinfo)
}

/// Generate an opaque OIDC state value.
#[must_use]
pub fn generate_state() -> String {
    format!("oidc-state-{}", Uuid::now_v7())
}

/// Hash a state value before persistence.
#[must_use]
pub fn hash_state(state: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(state.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize())
}
