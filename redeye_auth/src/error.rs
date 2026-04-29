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
}

impl AppError {
    /// Get the HTTP status code for this error
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
        }
    }

    /// Get the error code string for this error
    fn error_code(&self) -> ErrorCode {
        match self {
            AppError::Internal(_) => ErrorCode::Internal,
            AppError::BadRequest(_) => ErrorCode::BadRequest,
            AppError::Unauthorized(_) => ErrorCode::Unauthorized,
            AppError::Conflict(_) => ErrorCode::Conflict,
            AppError::NotFound(_) => ErrorCode::NotFound,
            AppError::RateLimited(_) => ErrorCode::RateLimited,
        }
    }

    /// Get a safe user-facing message (never exposes internal details)
    fn user_message(&self) -> String {
        match self {
            AppError::Internal(_) => {
                "An unexpected error occurred. Please try again later.".to_string()
            }
            AppError::BadRequest(msg) => msg.clone(),
            AppError::Unauthorized(msg) => msg.clone(),
            AppError::Conflict(msg) => msg.clone(),
            AppError::NotFound(msg) => msg.clone(),
            AppError::RateLimited(msg) => msg.clone(),
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
                    "Internal error occurred"
                );
            }
            _ => {
                tracing::warn!(
                    error_code = %code.as_str(),
                    status = %status.as_u16(),
                    message = %message,
                    "Client error occurred"
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

// Convert sqlx errors to AppError - internal details are logged but not exposed
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        use sqlx::Error as SqlxError;

        // Log the full error details for internal debugging
        tracing::error!(sqlx_error = %err, "Database error occurred");

        match err {
            SqlxError::RowNotFound => AppError::NotFound("Resource not found".into()),
            SqlxError::Database(db_err) => {
                // Check for unique constraint violations (PostgreSQL error code 23505)
                if db_err.code().as_deref() == Some("23505") {
                    AppError::Conflict("Resource already exists".into())
                } else {
                    AppError::Internal(format!("Database error: {}", db_err))
                }
            }
            _ => AppError::Internal("Database operation failed".into()),
        }
    }
}

// Convert validation errors to BadRequest
impl From<std::num::ParseIntError> for AppError {
    fn from(_: std::num::ParseIntError) -> Self {
        AppError::BadRequest("Invalid number format".into())
    }
}

impl From<std::num::ParseFloatError> for AppError {
    fn from(_: std::num::ParseFloatError) -> Self {
        AppError::BadRequest("Invalid number format".into())
    }
}

impl From<std::str::Utf8Error> for AppError {
    fn from(_: std::str::Utf8Error) -> Self {
        AppError::BadRequest("Invalid UTF-8 encoding".into())
    }
}
