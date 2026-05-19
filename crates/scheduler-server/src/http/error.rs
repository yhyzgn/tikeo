//! HTTP error mapping.

use axum::{Json, http::StatusCode, response::IntoResponse};

use super::dto::{ApiResponse, ErrorData};

/// Business code for endpoints that are intentionally not implemented yet.
pub const NOT_IMPLEMENTED_CODE: i32 = 10_001;
/// Business code for storage failures.
pub const STORAGE_ERROR_CODE: i32 = 20_001;

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
}

impl ApiError {
    /// Build a storage API error.
    #[must_use]
    pub fn storage(error: &scheduler_storage::DbErr) -> Self {
        Self::Storage {
            message: format!("storage operation failed: {error}"),
        }
    }

    const fn status_code(&self) -> StatusCode {
        match self {
            Self::NotImplemented { .. } => StatusCode::NOT_IMPLEMENTED,
            Self::Storage { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    const fn code(&self) -> i32 {
        match self {
            Self::NotImplemented { .. } => NOT_IMPLEMENTED_CODE,
            Self::Storage { .. } => STORAGE_ERROR_CODE,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::NotImplemented { message } | Self::Storage { message } => message.clone(),
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
