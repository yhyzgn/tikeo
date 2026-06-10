//! HTTP error mapping.

use axum::{Json, http::StatusCode, response::IntoResponse};

use super::dto::{ApiResponse, ErrorData};

/// Business code for endpoints that are intentionally not implemented yet.
pub const NOT_IMPLEMENTED_CODE: i32 = 10_001;
/// Business code for storage failures.
pub const STORAGE_ERROR_CODE: i32 = 20_001;
/// Business code for malformed requests.
pub const BAD_REQUEST_CODE: i32 = 40_001;
/// Business code for missing resources.
pub const NOT_FOUND_CODE: i32 = 40_004;
/// Business code for authentication failures.
pub const UNAUTHORIZED_CODE: i32 = 40_101;
/// Business code for authorization failures (forbidden).
pub const FORBIDDEN_CODE: i32 = 40_301;
/// Business code for state conflicts such as protected references.
pub const CONFLICT_CODE: i32 = 40_409;

/// API error variants returned by management handlers.
#[derive(Debug, Clone)]
pub enum ApiError {
    /// The requested operation is part of the API contract but not implemented yet.
    NotImplemented {
        /// Human-readable not implemented reason.
        message: String,
    },
    /// Database or repository operation failed.
    Storage {
        /// Human-readable storage error.
        message: String,
    },
    /// Request payload or parameter is invalid.
    BadRequest {
        /// Human-readable validation error.
        message: String,
    },
    /// Requested resource does not exist.
    NotFound {
        /// Human-readable missing-resource error.
        message: String,
    },
    /// Authentication failed or credentials are missing.
    Unauthorized {
        /// Human-readable authentication error.
        message: String,
    },
    /// Authorization failed.
    Forbidden {
        /// Human-readable authorization error.
        message: String,
    },
    /// Request conflicts with current resource state.
    Conflict {
        /// Human-readable conflict error.
        message: String,
    },
}

impl ApiError {
    /// Build a storage API error.
    #[must_use]
    pub fn storage(error: &tikeo_storage::DbErr) -> Self {
        Self::Storage {
            message: format!("storage operation failed: {error}"),
        }
    }

    /// Build a bad request API error.
    #[must_use]
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest {
            message: message.into(),
        }
    }

    /// Build a not found API error.
    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    /// Build an unauthorized API error.
    #[must_use]
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized {
            message: message.into(),
        }
    }

    /// Build a forbidden API error.
    #[must_use]
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden {
            message: message.into(),
        }
    }

    /// Build a conflict API error.
    #[must_use]
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    const fn status_code(&self) -> StatusCode {
        match self {
            Self::NotImplemented { .. } => StatusCode::NOT_IMPLEMENTED,
            Self::Storage { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            Self::Forbidden { .. } => StatusCode::FORBIDDEN,
            Self::Conflict { .. } => StatusCode::CONFLICT,
        }
    }

    const fn code(&self) -> i32 {
        match self {
            Self::NotImplemented { .. } => NOT_IMPLEMENTED_CODE,
            Self::Storage { .. } => STORAGE_ERROR_CODE,
            Self::BadRequest { .. } => BAD_REQUEST_CODE,
            Self::NotFound { .. } => NOT_FOUND_CODE,
            Self::Unauthorized { .. } => UNAUTHORIZED_CODE,
            Self::Forbidden { .. } => FORBIDDEN_CODE,
            Self::Conflict { .. } => CONFLICT_CODE,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::NotImplemented { message }
            | Self::Storage { message }
            | Self::BadRequest { message }
            | Self::NotFound { message }
            | Self::Unauthorized { message }
            | Self::Forbidden { message }
            | Self::Conflict { message } => message.clone(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code();
        let body = ApiResponse {
            code: self.code(),
            message: self.message(),
            data: Some(ErrorData {
                trace_id: "unavailable".to_owned(),
                details: None,
            }),
        };

        (status, Json(body)).into_response()
    }
}
