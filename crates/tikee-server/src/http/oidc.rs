//! OIDC authorization-code token exchange helpers.
//!
//! This module owns the network boundary to the configured `IdP`. Identity is still
//! fail-closed at the caller until `id_token` verification and JWKS validation are
//! wired, so the server never creates a session from unverified provider data.

use serde::Deserialize;
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
