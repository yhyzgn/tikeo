//! HTTP error mapping.

use axum::{Json, http::StatusCode, response::IntoResponse};

use super::dto::{ApiResponse, ErrorData};

/// Business code for endpoints that are intentionally not implemented yet.
pub const NOT_IMPLEMENTED_CODE: i32 = 10_001;

/// API error variants returned by management handlers.
#[derive(Debug, Clone)]
pub enum ApiError {
    /// The requested operation is part of the API contract but not implemented yet.
    NotImplemented {
        /// Human-readable not implemented reason.
        message: String,
    },
}

impl ApiError {
    const fn status_code(&self) -> StatusCode {
        match self {
            Self::NotImplemented { .. } => StatusCode::NOT_IMPLEMENTED,
        }
    }

    const fn code(&self) -> i32 {
        match self {
            Self::NotImplemented { .. } => NOT_IMPLEMENTED_CODE,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::NotImplemented { message } => message.clone(),
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
