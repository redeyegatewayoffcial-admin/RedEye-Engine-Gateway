//! Shared error types for RedEye Compliance microservice.
//! Provides standardized error handling with safe user-facing messages.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Standardized error codes for API responses.
#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    Internal,
    BadRequest,
    Unauthorized,
    Conflict,
    NotFound,
    RateLimited,
    PolicyViolation,
    ServiceUnavailable,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::Internal => "INTERNAL_ERROR",
            ErrorCode::BadRequest => "BAD_REQUEST",
            ErrorCode::Unauthorized => "UNAUTHORIZED",
            ErrorCode::Conflict => "CONFLICT",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::RateLimited => "RATE_LIMITED",
            ErrorCode::PolicyViolation => "POLICY_VIOLATION",
            ErrorCode::ServiceUnavailable => "SERVICE_UNAVAILABLE",
        }
    }
}

/// Application-wide error type with safe user-facing messages.
/// Internal errors are logged but never exposed to clients.
#[derive(Debug)]
pub enum AppError {
    /// Internal server error - details are logged but generic message sent to client
    Internal(String),
    /// Bad request - client sent invalid data
    BadRequest(String),
    /// Unauthorized - authentication required or failed
    Unauthorized(String),
    /// Conflict - resource already exists or state conflict
    Conflict(String),
    /// Not found - requested resource doesn't exist
    NotFound(String),
    /// Rate limited - too many requests
    RateLimited(String),
    /// Policy violation - compliance/OPA policy blocked the request
    PolicyViolation(String),
    /// Service unavailable - upstream dependency failed
    ServiceUnavailable(String),
}

impl AppError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::PolicyViolation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// Get the error code string for this error
    pub fn error_code(&self) -> ErrorCode {
        match self {
            AppError::Internal(_) => ErrorCode::Internal,
            AppError::BadRequest(_) => ErrorCode::BadRequest,
            AppError::Unauthorized(_) => ErrorCode::Unauthorized,
            AppError::Conflict(_) => ErrorCode::Conflict,
            AppError::NotFound(_) => ErrorCode::NotFound,
            AppError::RateLimited(_) => ErrorCode::RateLimited,
            AppError::PolicyViolation(_) => ErrorCode::PolicyViolation,
            AppError::ServiceUnavailable(_) => ErrorCode::ServiceUnavailable,
        }
    }

    /// Get a safe user-facing message (never exposes internal details)
    pub fn user_message(&self) -> String {
        match self {
            AppError::Internal(_) => "An unexpected error occurred. Please try again later.".to_string(),
            AppError::BadRequest(msg) => msg.clone(),
            AppError::Unauthorized(msg) => msg.clone(),
            AppError::Conflict(msg) => msg.clone(),
            AppError::NotFound(msg) => msg.clone(),
            AppError::RateLimited(msg) => msg.clone(),
            AppError::PolicyViolation(msg) => msg.clone(),
            AppError::ServiceUnavailable(msg) => msg.clone(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let code = self.error_code();
        let message = self.user_message();

        // Log internal errors with full details for debugging
        match &self {
            AppError::Internal(internal_msg) => {
                tracing::error!(
                    error_code = %code.as_str(),
                    status = %status.as_u16(),
                    internal_details = %internal_msg,
                    "Internal compliance error occurred"
                );
            }
            AppError::ServiceUnavailable(internal_msg) => {
                tracing::error!(
                    error_code = %code.as_str(),
                    status = %status.as_u16(),
                    service_details = %internal_msg,
                    "Service unavailable error occurred"
                );
            }
            _ => {
                tracing::warn!(
                    error_code = %code.as_str(),
                    status = %status.as_u16(),
                    message = %message,
                    "Compliance client error occurred"
                );
            }
        }

        let body = Json(json!({
            "error": {
                "code": code.as_str(),
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error_code().as_str(), self.user_message())
    }
}

impl std::error::Error for AppError {}

// Convert serde_json errors to BadRequest
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::BadRequest(format!("Invalid JSON: {}", err))
    }
}

// Convert std::io::Error to Internal
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Internal(format!("IO error: {}", err))
    }
}
