//! Session management abstraction for HTTP authentication.
//!
//! The current production implementation uses a durable database table as the
//! source of truth plus a short-lived local moka cache. The trait boundary keeps
//! session reads/writes replaceable by a future Redis-backed distributed store
//! when the tikee server is deployed as a multi-node cluster.

use std::{fmt, sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use moka::future::Cache;
use sha2::{Digest, Sha256};
use tikee_storage::{AuthSessionRepository, CreateAuthSession, RbacRepository};
use uuid::Uuid;

use super::{
    dto::{AuthSession, MeResponse},
    error::ApiError,
};

/// Session creation input passed from the authentication boundary.
#[derive(Debug, Clone)]
pub struct SessionCreate {
    /// Persisted user identifier.
    pub user_id: String,
    /// Current username snapshot.
    pub username: String,
    /// Current role snapshot.
    pub role: String,
    /// Optional stable device identifier supplied by clients in the future.
    pub device_id: Option<String>,
    /// Optional human-readable device name supplied by clients in the future.
    pub device_name: Option<String>,
}

/// Pluggable session store contract.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Create a new session and return the bearer token to the caller.
    async fn create_session(&self, input: SessionCreate) -> Result<AuthSession, ApiError>;

    /// Resolve an opaque bearer token to the authenticated principal.
    async fn get_principal(&self, token: &str) -> Result<Option<MeResponse>, ApiError>;

    /// Revoke one bearer token.
    async fn revoke_token(&self, token: &str) -> Result<(), ApiError>;

    /// Revoke all sessions owned by a username.
    async fn revoke_user_sessions(&self, username: &str) -> Result<(), ApiError>;
}

/// Cloneable application handle around a pluggable session store.
#[derive(Clone)]
pub struct SessionManager {
    inner: Arc<dyn SessionStore>,
}

impl SessionManager {
    /// Wrap a concrete session store.
    #[must_use]
    pub fn new(store: impl SessionStore + 'static) -> Self {
        Self {
            inner: Arc::new(store),
        }
    }

    /// Create a new session.
    ///
    /// # Errors
    ///
    /// Returns an API error when the configured store cannot persist the session.
    pub async fn create_session(&self, input: SessionCreate) -> Result<AuthSession, ApiError> {
        self.inner.create_session(input).await
    }

    /// Resolve a bearer token.
    ///
    /// # Errors
    ///
    /// Returns an API error when the configured store cannot validate the token.
    pub async fn get_principal(&self, token: &str) -> Result<Option<MeResponse>, ApiError> {
        self.inner.get_principal(token).await
    }

    /// Revoke one token.
    ///
    /// # Errors
    ///
    /// Returns an API error when the configured store cannot revoke the token.
    pub async fn revoke_token(&self, token: &str) -> Result<(), ApiError> {
        self.inner.revoke_token(token).await
    }

    /// Revoke all sessions for a username.
    ///
    /// # Errors
    ///
    /// Returns an API error when the configured store cannot revoke sessions.
    pub async fn revoke_user_sessions(&self, username: &str) -> Result<(), ApiError> {
        self.inner.revoke_user_sessions(username).await
    }
}

impl fmt::Debug for SessionManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionManager(..)")
    }
}

/// Database-backed session store with local moka read-through cache.
#[derive(Debug, Clone)]
pub struct DbMokaSessionStore {
    repo: AuthSessionRepository,
    rbac: RbacRepository,
    cache: Cache<String, MeResponse>,
    session_ttl: ChronoDuration,
}

impl DbMokaSessionStore {
    /// Build the default DB+moka session store.
    #[must_use]
    pub fn new(repo: AuthSessionRepository, rbac: RbacRepository) -> Self {
        Self {
            repo,
            rbac,
            cache: Cache::builder()
                .time_to_live(Duration::from_mins(5))
                .max_capacity(16_384)
                .build(),
            session_ttl: ChronoDuration::hours(12),
        }
    }

    async fn prune_expired_sessions(&self) -> Result<(), ApiError> {
        let deleted = self
            .repo
            .delete_expired()
            .await
            .map_err(|error| ApiError::storage(&error))?;
        if deleted > 0 {
            self.cache.invalidate_all();
        }
        Ok(())
    }
}

#[async_trait]
impl SessionStore for DbMokaSessionStore {
    async fn create_session(&self, input: SessionCreate) -> Result<AuthSession, ApiError> {
        self.prune_expired_sessions().await?;
        let token = generate_access_token();
        let token_hash = hash_token(&token);
        let expires_at = (Utc::now() + self.session_ttl).to_rfc3339();

        let summary = self
            .repo
            .create_session(CreateAuthSession {
                user_id: input.user_id,
                token_hash: token_hash.clone(),
                device_id: input.device_id,
                device_name: input.device_name,
                expires_at,
            })
            .await
            .map_err(|error| ApiError::storage(&error))?;

        let roles = vec![summary.role.clone()];
        let permissions = self
            .rbac
            .permissions_for_roles(&roles)
            .await
            .map_err(|error| ApiError::storage(&error))?;
        let principal = MeResponse {
            username: summary.username.clone(),
            roles,
            permissions,
        };
        self.cache.insert(token_hash, principal.clone()).await;

        Ok(AuthSession {
            token,
            username: principal.username,
            roles: principal.roles,
            permissions: principal.permissions,
        })
    }

    async fn get_principal(&self, token: &str) -> Result<Option<MeResponse>, ApiError> {
        self.prune_expired_sessions().await?;
        let token_hash = hash_token(token);
        if let Some(principal) = self.cache.get(&token_hash).await {
            return Ok(Some(principal));
        }

        let Some(summary) = self
            .repo
            .get_by_token_hash(&token_hash)
            .await
            .map_err(|error| ApiError::storage(&error))?
        else {
            return Ok(None);
        };

        let roles = vec![summary.role];
        let permissions = self
            .rbac
            .permissions_for_roles(&roles)
            .await
            .map_err(|error| ApiError::storage(&error))?;
        let principal = MeResponse {
            username: summary.username,
            roles,
            permissions,
        };
        self.cache.insert(token_hash, principal.clone()).await;
        Ok(Some(principal))
    }

    async fn revoke_token(&self, token: &str) -> Result<(), ApiError> {
        let token_hash = hash_token(token);
        self.repo
            .delete_by_token_hash(&token_hash)
            .await
            .map_err(|error| ApiError::storage(&error))?;
        self.cache.invalidate(&token_hash).await;
        Ok(())
    }

    async fn revoke_user_sessions(&self, username: &str) -> Result<(), ApiError> {
        self.repo
            .delete_by_username(username)
            .await
            .map_err(|error| ApiError::storage(&error))?;
        // User-wide revocation may affect many cached token hashes. Clearing the
        // short local cache keeps correctness simple; DB remains authoritative.
        self.cache.invalidate_all();
        Ok(())
    }
}

fn generate_access_token() -> String {
    format!("atk_{}", Uuid::new_v4().as_simple())
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use super::{generate_access_token, hash_token};

    #[test]
    fn access_tokens_are_opaque_and_hashable() {
        let token = generate_access_token();
        assert!(token.starts_with("atk_"));
        assert_eq!(hash_token(&token).len(), 64);
        assert_ne!(hash_token(&token), token);
    }
}
