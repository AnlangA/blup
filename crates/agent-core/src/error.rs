use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Session not found")]
    NotFound,

    #[error("Invalid state transition: {0}")]
    InvalidTransition(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Service unavailable: at session capacity")]
    ServiceUnavailable,

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::InvalidTransition(_) => StatusCode::CONFLICT,
            ApiError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Agent(_) | ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_code(&self) -> &str {
        match self {
            ApiError::NotFound => "NOT_FOUND",
            ApiError::InvalidTransition(_) => "INVALID_STATE_TRANSITION",
            ApiError::Validation(_) => "VALIDATION_ERROR",
            ApiError::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            ApiError::Agent(_) => "AGENT_ERROR",
            ApiError::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ErrorBody {
            error: ErrorDetail {
                code: self.error_code().to_string(),
                message: self.to_string(),
            },
        };
        (status, Json(body)).into_response()
    }
}

impl From<blup_agent::AgentError> for ApiError {
    fn from(e: blup_agent::AgentError) -> Self {
        ApiError::Agent(e.to_string())
    }
}

impl From<crate::state::types::StateError> for ApiError {
    fn from(e: crate::state::types::StateError) -> Self {
        ApiError::InvalidTransition(e.to_string())
    }
}

impl From<storage::StorageError> for ApiError {
    fn from(e: storage::StorageError) -> Self {
        ApiError::Internal(format!("Storage error: {e}"))
    }
}
