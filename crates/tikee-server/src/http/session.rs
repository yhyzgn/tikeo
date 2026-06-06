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
use tikee_storage::{AuthSessionRepository, CreateAuthSession, PermissionSummary, RbacRepository};

use super::{
    dto::{AccessScopeBinding, ApiTokenSummary, AuthSession, CreatedApiToken, MeResponse},
    error::ApiError,
    opaque_token::generate_base62,
    session_metadata::{
        api_token_scopes, api_token_summary, encode_api_token_device_id, encode_session_device_id,
        is_api_token_session, session_scope_bindings,
    },
};

const ACCESS_TOKEN_LENGTH: usize = 48;
const HUMAN_SESSION_TTL_DAYS: i64 = 7;

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
    /// Optional API-token scope allow-list in `resource:action` form.
    pub token_scopes: Vec<String>,
    /// Optional API-token namespace/app/worker-pool bindings.
    pub scope_bindings: Vec<AccessScopeBinding>,
    /// Optional API-token lifetime override in seconds.
    pub expires_in_seconds: Option<i64>,
}

/// Pluggable session store contract.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Create a new session and return the bearer token to the caller.
    async fn create_session(&self, input: SessionCreate) -> Result<AuthSession, ApiError>;

    /// Create a durable API token and return the raw bearer token once.
    async fn create_api_token(&self, input: SessionCreate) -> Result<CreatedApiToken, ApiError>;

    /// List durable API token metadata for one principal.
    async fn list_api_tokens(&self, username: &str) -> Result<Vec<ApiTokenSummary>, ApiError>;

    /// Revoke one API token owned by the principal.
    async fn revoke_api_token(&self, username: &str, token_id: &str) -> Result<bool, ApiError>;

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

    /// Create a durable API token.
    ///
    /// # Errors
    ///
    /// Returns an API error when the configured store cannot persist the token.
    pub async fn create_api_token(
        &self,
        input: SessionCreate,
    ) -> Result<CreatedApiToken, ApiError> {
        self.inner.create_api_token(input).await
    }

    /// List durable API tokens for a username.
    ///
    /// # Errors
    ///
    /// Returns an API error when the configured store cannot list tokens.
    pub async fn list_api_tokens(&self, username: &str) -> Result<Vec<ApiTokenSummary>, ApiError> {
        self.inner.list_api_tokens(username).await
    }

    /// Revoke one durable API token.
    ///
    /// # Errors
    ///
    /// Returns an API error when the configured store cannot revoke the token.
    pub async fn revoke_api_token(&self, username: &str, token_id: &str) -> Result<bool, ApiError> {
        self.inner.revoke_api_token(username, token_id).await
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
    cache: Cache<String, CachedPrincipal>,
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
            session_ttl: ChronoDuration::days(HUMAN_SESSION_TTL_DAYS),
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

    async fn renew_human_session_expiry(&self, token_hash: &str) -> Result<bool, ApiError> {
        let expires_at = (Utc::now() + self.session_ttl).to_rfc3339();
        self.repo
            .renew_expires_at(token_hash, expires_at)
            .await
            .map_err(|error| ApiError::storage(&error))
    }
}

#[derive(Debug, Clone)]
struct CachedPrincipal {
    principal: MeResponse,
    renewable: bool,
}

#[async_trait]
impl SessionStore for DbMokaSessionStore {
    async fn create_session(&self, input: SessionCreate) -> Result<AuthSession, ApiError> {
        self.prune_expired_sessions().await?;
        let token = generate_access_token()?;
        let token_hash = hash_token(&token);
        let expires_at = (Utc::now() + self.session_ttl).to_rfc3339();

        let summary = self
            .repo
            .create_session(CreateAuthSession {
                user_id: input.user_id,
                token_hash: token_hash.clone(),
                device_id: encode_session_device_id(input.device_id, &input.scope_bindings),
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
        let scope_bindings = session_scope_bindings(&summary);
        let principal = MeResponse {
            username: summary.username.clone(),
            roles,
            permissions,
            scope_limited: !scope_bindings.is_empty(),
            token_scopes: Vec::new(),
            scope_bindings,
        };
        self.cache
            .insert(
                token_hash,
                CachedPrincipal {
                    principal: principal.clone(),
                    renewable: true,
                },
            )
            .await;

        Ok(AuthSession {
            token,
            username: principal.username,
            roles: principal.roles,
            permissions: principal.permissions,
            scope_limited: principal.scope_limited,
            token_scopes: principal.token_scopes,
            scope_bindings: principal.scope_bindings,
        })
    }

    async fn create_api_token(&self, input: SessionCreate) -> Result<CreatedApiToken, ApiError> {
        self.prune_expired_sessions().await?;
        let token = generate_access_token()?;
        let token_hash = hash_token(&token);
        let ttl = input
            .expires_in_seconds
            .map_or(self.session_ttl, ChronoDuration::seconds);
        let expires_at = (Utc::now() + ttl).to_rfc3339();
        let token_name = input
            .device_name
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or("api-token")
            .to_owned();

        let summary = self
            .repo
            .create_session(CreateAuthSession {
                user_id: input.user_id,
                token_hash: token_hash.clone(),
                device_id: Some(encode_api_token_device_id(
                    &input.token_scopes,
                    &input.scope_bindings,
                )),
                device_name: Some(token_name),
                expires_at,
            })
            .await
            .map_err(|error| ApiError::storage(&error))?;

        let roles = vec![summary.role.clone()];
        let role_permissions = self
            .rbac
            .permissions_for_roles(&roles)
            .await
            .map_err(|error| ApiError::storage(&error))?;
        let token_scopes = api_token_scopes(&summary);
        let scope_bindings = session_scope_bindings(&summary);
        let permissions = if token_scopes.is_empty() {
            role_permissions
        } else {
            permissions_from_scopes(&token_scopes)
        };
        let principal = MeResponse {
            username: summary.username.clone(),
            roles,
            permissions,
            scope_limited: !token_scopes.is_empty() || !scope_bindings.is_empty(),
            token_scopes,
            scope_bindings,
        };
        self.cache
            .insert(
                token_hash,
                CachedPrincipal {
                    principal,
                    renewable: false,
                },
            )
            .await;

        Ok(CreatedApiToken {
            access_token: token,
            token: api_token_summary(summary),
        })
    }

    async fn list_api_tokens(&self, username: &str) -> Result<Vec<ApiTokenSummary>, ApiError> {
        self.prune_expired_sessions().await?;
        let sessions = self
            .repo
            .list_by_username(username)
            .await
            .map_err(|error| ApiError::storage(&error))?;
        Ok(sessions
            .into_iter()
            .filter(is_api_token_session)
            .map(api_token_summary)
            .collect())
    }

    async fn revoke_api_token(&self, username: &str, token_id: &str) -> Result<bool, ApiError> {
        let sessions = self
            .repo
            .list_by_username(username)
            .await
            .map_err(|error| ApiError::storage(&error))?;
        let Some(session) = sessions
            .into_iter()
            .find(|session| session.id == token_id && is_api_token_session(session))
        else {
            return Ok(false);
        };
        let revoked = self
            .repo
            .delete_by_id_for_username(token_id, username)
            .await
            .map_err(|error| ApiError::storage(&error))?;
        if revoked {
            self.cache.invalidate(&session.token_hash).await;
        }
        Ok(revoked)
    }

    async fn get_principal(&self, token: &str) -> Result<Option<MeResponse>, ApiError> {
        self.prune_expired_sessions().await?;
        let token_hash = hash_token(token);
        if let Some(cached) = self.cache.get(&token_hash).await {
            if cached.renewable && !self.renew_human_session_expiry(&token_hash).await? {
                self.cache.invalidate(&token_hash).await;
                return Ok(None);
            }
            return Ok(Some(cached.principal));
        }

        let Some(summary) = self
            .repo
            .get_by_token_hash(&token_hash)
            .await
            .map_err(|error| ApiError::storage(&error))?
        else {
            return Ok(None);
        };

        let renewable = !is_api_token_session(&summary);
        if renewable && !self.renew_human_session_expiry(&token_hash).await? {
            self.cache.invalidate(&token_hash).await;
            return Ok(None);
        }

        let roles = vec![summary.role.clone()];
        let role_permissions = self
            .rbac
            .permissions_for_roles(&roles)
            .await
            .map_err(|error| ApiError::storage(&error))?;
        let token_scopes = api_token_scopes(&summary);
        let scope_bindings = session_scope_bindings(&summary);
        let permissions = if token_scopes.is_empty() {
            role_permissions
        } else {
            permissions_from_scopes(&token_scopes)
        };
        let principal = MeResponse {
            username: summary.username,
            roles,
            permissions,
            scope_limited: !token_scopes.is_empty() || !scope_bindings.is_empty(),
            token_scopes,
            scope_bindings,
        };
        self.cache
            .insert(
                token_hash,
                CachedPrincipal {
                    principal: principal.clone(),
                    renewable,
                },
            )
            .await;
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

fn permissions_from_scopes(scopes: &[String]) -> Vec<PermissionSummary> {
    scopes
        .iter()
        .filter_map(|scope| {
            let (resource, action) = scope.split_once(':')?;
            Some(PermissionSummary {
                resource: resource.to_owned(),
                action: action.to_owned(),
            })
        })
        .collect()
}

fn generate_access_token() -> Result<String, ApiError> {
    generate_base62(ACCESS_TOKEN_LENGTH)
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use tikee_storage::{
        AuthSessionRepository, CreateUser, RbacRepository, UserRepository, connect_and_migrate,
    };

    use super::{
        ACCESS_TOKEN_LENGTH, DbMokaSessionStore, SessionCreate, SessionStore,
        generate_access_token, hash_token,
    };

    #[test]
    fn access_tokens_are_opaque_and_hashable() {
        let token = generate_access_token().unwrap_or_else(|error| {
            panic!("access token generation should use OS randomness: {error:?}")
        });
        assert_eq!(token.len(), ACCESS_TOKEN_LENGTH);
        assert!(token.chars().all(|value| value.is_ascii_alphanumeric()));
        assert_eq!(hash_token(&token).len(), 64);
        assert_ne!(hash_token(&token), token);
    }

    #[tokio::test]
    async fn human_sessions_slide_expiry_on_authenticated_request() {
        let fixture = session_store_fixture().await;
        let session = fixture
            .store
            .create_session(session_create(fixture.user_id.clone(), Vec::new(), None))
            .await
            .unwrap_or_else(|error| panic!("session should create: {error:?}"));
        let token_hash = hash_token(&session.token);
        let old_expiry = "2099-01-01T00:00:00Z".to_owned();
        fixture
            .sessions
            .renew_expires_at(&token_hash, old_expiry.clone())
            .await
            .unwrap_or_else(|error| panic!("test expiry should update: {error}"));

        let principal = fixture
            .store
            .get_principal(&session.token)
            .await
            .unwrap_or_else(|error| panic!("principal should resolve: {error:?}"));
        assert!(principal.is_some());

        let renewed = fixture
            .sessions
            .get_by_token_hash(&token_hash)
            .await
            .unwrap_or_else(|error| panic!("renewed session should load: {error}"))
            .unwrap_or_else(|| panic!("renewed session should exist"));
        assert_ne!(renewed.expires_at, old_expiry);
        let renewed_at = DateTime::parse_from_rfc3339(&renewed.expires_at)
            .unwrap_or_else(|error| panic!("renewed expiry should be rfc3339: {error}"))
            .with_timezone(&Utc);
        assert!(renewed_at > Utc::now() + chrono::Duration::days(6));
    }

    #[tokio::test]
    async fn api_tokens_do_not_slide_expiry_on_authenticated_request() {
        let fixture = session_store_fixture().await;
        let created = fixture
            .store
            .create_api_token(session_create(
                fixture.user_id.clone(),
                vec!["users:read".to_owned()],
                Some(900),
            ))
            .await
            .unwrap_or_else(|error| panic!("api token should create: {error:?}"));
        let token_hash = hash_token(&created.access_token);
        let fixed_expiry = "2099-01-01T00:00:00Z".to_owned();
        fixture
            .sessions
            .renew_expires_at(&token_hash, fixed_expiry.clone())
            .await
            .unwrap_or_else(|error| panic!("test expiry should update: {error}"));

        let principal = fixture
            .store
            .get_principal(&created.access_token)
            .await
            .unwrap_or_else(|error| panic!("principal should resolve: {error:?}"));
        assert!(principal.is_some());

        let loaded = fixture
            .sessions
            .get_by_token_hash(&token_hash)
            .await
            .unwrap_or_else(|error| panic!("api token should load: {error}"))
            .unwrap_or_else(|| panic!("api token should exist"));
        assert_eq!(loaded.expires_at, fixed_expiry);
    }

    struct SessionStoreFixture {
        store: DbMokaSessionStore,
        sessions: AuthSessionRepository,
        user_id: String,
    }

    async fn session_store_fixture() -> SessionStoreFixture {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let users = UserRepository::new(db.clone());
        let user = users
            .create_user(CreateUser {
                username: "session_admin".to_owned(),
                email: "session-admin@example.com".to_owned(),
                password: "$2b$10$sessionhash".to_owned(),
                role: "admin".to_owned(),
                bootstrap_admin: true,
            })
            .await
            .unwrap_or_else(|error| panic!("test user should create: {error}"));
        let sessions = AuthSessionRepository::new(db.clone());
        let store = DbMokaSessionStore::new(sessions.clone(), RbacRepository::new(db));
        SessionStoreFixture {
            store,
            sessions,
            user_id: user.id,
        }
    }

    fn session_create(
        user_id: String,
        token_scopes: Vec<String>,
        expires_in_seconds: Option<i64>,
    ) -> SessionCreate {
        SessionCreate {
            user_id,
            username: "session_admin".to_owned(),
            role: "admin".to_owned(),
            device_id: None,
            device_name: Some("unit-test".to_owned()),
            token_scopes,
            scope_bindings: Vec::new(),
            expires_in_seconds,
        }
    }
}
