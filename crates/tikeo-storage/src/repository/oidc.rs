use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::entities::oidc_auth_state;

use super::util::{new_id, now_rfc3339};

/// Persisted OIDC state creation input.
#[derive(Debug, Clone)]
pub struct CreateOidcAuthState {
    /// SHA-256 hash of the opaque state value.
    pub state_hash: String,
    /// Redirect URI associated with the authorization URL.
    pub redirect_uri: String,
    /// Time-to-live in seconds.
    pub ttl_seconds: i64,
}

/// OIDC authorization state summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcAuthStateSummary {
    /// State row id.
    pub id: String,
    /// SHA-256 state hash.
    pub state_hash: String,
    /// Redirect URI associated with this state.
    pub redirect_uri: String,
    /// Expiration timestamp.
    pub expires_at: String,
    /// Consumption timestamp, if any.
    pub consumed_at: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
}

/// OIDC authorization state repository.
#[derive(Debug, Clone)]
pub struct OidcAuthStateRepository {
    db: DatabaseConnection,
}

impl OidcAuthStateRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Persist a new OIDC state.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn create_state(
        &self,
        input: CreateOidcAuthState,
    ) -> Result<OidcAuthStateSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let expires_at = (OffsetDateTime::now_utc() + Duration::seconds(input.ttl_seconds))
            .format(&Rfc3339)
            .unwrap_or_else(|_| now.clone());
        let model = oidc_auth_state::ActiveModel {
            id: Set(new_id("oidc-state")),
            state_hash: Set(input.state_hash),
            redirect_uri: Set(input.redirect_uri),
            expires_at: Set(expires_at),
            consumed_at: Set(None),
            created_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(OidcAuthStateSummary::from_model(model))
    }

    /// Consume one valid OIDC state exactly once.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn consume_state(
        &self,
        state_hash: &str,
    ) -> Result<Option<OidcAuthStateSummary>, sea_orm::DbErr> {
        let Some(model) = oidc_auth_state::Entity::find()
            .filter(oidc_auth_state::Column::StateHash.eq(state_hash.to_owned()))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        if model.consumed_at.is_some() || is_expired_rfc3339(&model.expires_at) {
            return Ok(None);
        }
        let mut active: oidc_auth_state::ActiveModel = model.into();
        active.consumed_at = Set(Some(now_rfc3339()));
        let updated = active.update(&self.db).await?;
        Ok(Some(OidcAuthStateSummary::from_model(updated)))
    }

    /// Delete expired OIDC states.
    ///
    /// # Errors
    ///
    /// Returns an error when database access fails.
    pub async fn delete_expired(&self) -> Result<u64, sea_orm::DbErr> {
        let now = now_rfc3339();
        let result = oidc_auth_state::Entity::delete_many()
            .filter(oidc_auth_state::Column::ExpiresAt.lte(now))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected)
    }
}

impl OidcAuthStateSummary {
    fn from_model(model: oidc_auth_state::Model) -> Self {
        Self {
            id: model.id,
            state_hash: model.state_hash,
            redirect_uri: model.redirect_uri,
            expires_at: model.expires_at,
            consumed_at: model.consumed_at,
            created_at: model.created_at,
        }
    }
}

fn is_expired_rfc3339(value: &str) -> bool {
    OffsetDateTime::parse(value, &Rfc3339)
        .map_or(true, |expires_at| expires_at <= OffsetDateTime::now_utc())
}
