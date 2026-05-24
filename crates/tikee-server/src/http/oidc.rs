//! OIDC authorization-code token exchange helpers.
//!
//! This module owns the network boundary to the configured `IdP`. Identity is still
//! fail-closed at the caller until `id_token` verification and JWKS validation are
//! wired, so the server never creates a session from unverified provider data.

use serde::Deserialize;
use serde_json::Value;
use url::Url;

use super::error::ApiError;

/// Query parameters used to build an OIDC authorization URL.
#[derive(Debug, Deserialize)]
pub struct OidcAuthorizeQuery {
    /// Optional UI callback URL.
    pub redirect_uri: Option<String>,
    /// Optional caller-provided CSRF state value.
    pub state: Option<String>,
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

/// Minimal token response data required before local identity verification can proceed.
#[derive(Debug, Deserialize)]
pub struct OidcTokenResponse {
    /// Provider-issued ID token that still requires signature/JWKS validation.
    pub id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OidcDiscoveryDocument {
    jwks_uri: Option<String>,
}

/// Minimal JWKS metadata fetched before local signature verification can proceed.
#[derive(Debug, Clone)]
pub struct OidcJwks {
    /// Raw JWKS document.
    pub document: Value,
    /// Number of keys in the key set.
    pub key_count: usize,
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

/// Exchange an authorization code for token response data.
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

/// Discover the provider JWKS URI from the standard `OpenID` Provider Configuration document.
///
/// # Errors
///
/// Returns a bad request when discovery fails or the provider omits `jwks_uri`.
pub async fn discover_jwks_uri(issuer: &str) -> Result<Url, ApiError> {
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
    let jwks_uri = discovery
        .jwks_uri
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("OIDC discovery response missing jwks_uri"))?;
    Url::parse(jwks_uri)
        .map_err(|_| ApiError::bad_request("OIDC discovery jwks_uri must be a valid URL"))
}

/// Fetch the provider JWKS document.
///
/// # Errors
///
/// Returns a bad request when the key set cannot be fetched or is empty.
pub async fn fetch_jwks(jwks_uri: Url) -> Result<OidcJwks, ApiError> {
    let response = reqwest::Client::new()
        .get(jwks_uri)
        .send()
        .await
        .map_err(|error| ApiError::bad_request(format!("OIDC JWKS fetch failed: {error}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::bad_request(format!(
            "OIDC JWKS fetch rejected with status {status}"
        )));
    }
    let document = response.json::<Value>().await.map_err(|error| {
        ApiError::bad_request(format!("OIDC JWKS response is invalid: {error}"))
    })?;
    let key_count = document
        .get("keys")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if key_count == 0 {
        return Err(ApiError::bad_request("OIDC JWKS response contains no keys"));
    }
    Ok(OidcJwks {
        document,
        key_count,
    })
}
