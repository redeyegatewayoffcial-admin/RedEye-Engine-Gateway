//! Structured error types for the `redeye_config` service.
//!
//! All variants map to a deterministic HTTP status code via [`IntoResponse`].
//! Internal details (raw DB errors, Redis messages) are **never** forwarded to
//! callers; they are logged at `tracing::error!` level, and a sanitised
//! user-facing message is returned instead.
//!
//! # Design decisions
//!
//! * [`thiserror`] provides the `Display` and `Error` impls.
//! * Manual [`From`] impls for `sqlx::Error` and `redis::RedisError` perform
//!   eager classification (e.g. RowNotFound → NotFound, PG-23505 → Conflict)
//!   at the infrastructure boundary, so handler code never inspects raw DB errors.
//! * The `#[error("...")]` messages on variants are the `Display` / `source`
//!   strings — they appear only in **internal** tracing, never in HTTP responses.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

// =============================================================================
// Error Code
// =============================================================================

/// Stable, machine-readable error codes embedded in every error response body.
///
/// Clients should branch on `error.code`, not on `error.message`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Internal,
    BadRequest,
    Unauthorized,
    NotFound,
    Conflict,
    UnprocessableEntity,
}

impl ErrorCode {
    /// Returns the canonical string representation used in JSON responses.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Internal => "INTERNAL_ERROR",
            Self::BadRequest => "BAD_REQUEST",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::UnprocessableEntity => "UNPROCESSABLE_ENTITY",
        }
    }
}

// =============================================================================
// ConfigError
// =============================================================================

/// Central, exhaustive error type for the `redeye_config` service.
///
/// Every variant is designed to carry enough context for **internal** structured
/// logging while keeping the public API surface clean and auditable.
#[derive(Debug, Error)]
pub enum ConfigError {
    // ── Client Errors (4xx) ────────────────────────────────────────────────
    /// The requested resource could not be found in the backing store.
    #[error("not found: {0}")]
    NotFound(String),

    /// The operation conflicts with existing state (e.g. duplicate key).
    #[error("conflict: {0}")]
    Conflict(String),

    /// The request payload failed domain-level validation.
    /// The inner message IS safe to forward to the client.
    #[error("validation error: {0}")]
    Validation(String),

    /// The caller lacks permission to perform this action.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    // ── Server Errors (5xx) ────────────────────────────────────────────────
    /// A database operation failed. Details are logged but opaque to clients.
    /// The inner string is a sanitised description, never raw SQL or stack trace.
    #[error("database error: {0}")]
    Database(String),

    /// A Redis operation failed. Details are logged but opaque to clients.
    #[error("redis error: {0}")]
    Redis(String),

    /// A catch-all for unexpected internal failures.
    #[error("internal error: {0}")]
    Internal(String),
}

// =============================================================================
// From impls — infrastructure boundaries
// =============================================================================

impl From<sqlx::Error> for ConfigError {
    fn from(err: sqlx::Error) -> Self {
        // Classify and log the raw error at the boundary.
        // Only the classification reaches callers; raw DB details stay here.
        tracing::error!(
            sqlx_error = %err,
            "Database layer error encountered"
        );
        match &err {
            // Transparent mapping: a missing row is a domain-level 404.
            sqlx::Error::RowNotFound => {
                Self::NotFound("The requested resource was not found.".into())
            }
            // PG error code 23505 = unique_violation → expose as a 409 Conflict.
            sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505") => {
                Self::Conflict("A resource with this identifier already exists.".into())
            }
            // PG error code 23503 = foreign_key_violation → safe surface message.
            sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23503") => {
                Self::Validation(
                    "The referenced tenant does not exist. Ensure the tenant is \
                     provisioned via redeye_auth before configuring it."
                        .into(),
                )
            }
            // All other DB errors are opaque internal failures.
            _ => Self::Database("A database operation failed unexpectedly.".into()),
        }
    }
}

impl From<redis::RedisError> for ConfigError {
    fn from(err: redis::RedisError) -> Self {
        tracing::error!(
            redis_error = %err,
            "Redis layer error encountered"
        );
        Self::Redis("A cache synchronisation operation failed.".into())
    }
}

// =============================================================================
// HTTP mapping — IntoResponse
// =============================================================================

impl ConfigError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Database(_) | Self::Redis(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn error_code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::Validation(_) => ErrorCode::UnprocessableEntity,
            Self::Unauthorized(_) => ErrorCode::Unauthorized,
            Self::Database(_) | Self::Redis(_) | Self::Internal(_) => ErrorCode::Internal,
        }
    }

    /// Returns a safe, user-facing message.
    ///
    /// Internal error details are **never** included — see logging in
    /// [`IntoResponse::into_response`] for the full details.
    fn user_message(&self) -> &str {
        match self {
            Self::NotFound(msg) => msg.as_str(),
            Self::Conflict(msg) => msg.as_str(),
            Self::Validation(msg) => msg.as_str(),
            Self::Unauthorized(msg) => msg.as_str(),
            Self::Database(_) => "An unexpected database error occurred. Please retry.",
            Self::Redis(_) => "A caching error occurred. Please retry.",
            Self::Internal(_) => "An unexpected internal error occurred. Please retry.",
        }
    }
}

impl IntoResponse for ConfigError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let code = self.error_code();
        let message = self.user_message().to_owned();

        // Emit structured telemetry before consuming `self`.
        if status.is_server_error() {
            tracing::error!(
                error_code   = code.as_str(),
                http_status  = status.as_u16(),
                detail       = %self,
                "redeye_config internal error"
            );
        } else {
            tracing::warn!(
                error_code   = code.as_str(),
                http_status  = status.as_u16(),
                message      = %message,
                "redeye_config client error"
            );
        }

        (
            status,
            Json(json!({
                "error": {
                    "code":    code.as_str(),
                    "message": message,
                }
            })),
        )
            .into_response()
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    // ── HTTP status mapping ─────────────────────────────────────────────────

    #[test]
    fn not_found_maps_to_404() {
        let err = ConfigError::NotFound("tenant config missing".into());
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(err.error_code(), ErrorCode::NotFound);
    }

    #[test]
    fn conflict_maps_to_409() {
        let err = ConfigError::Conflict("key exists".into());
        assert_eq!(err.status_code(), StatusCode::CONFLICT);
        assert_eq!(err.error_code(), ErrorCode::Conflict);
    }

    #[test]
    fn validation_maps_to_422() {
        let err = ConfigError::Validation("rate_limit_rpm must be positive".into());
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(err.error_code(), ErrorCode::UnprocessableEntity);
    }

    #[test]
    fn unauthorized_maps_to_401() {
        let err = ConfigError::Unauthorized("missing token".into());
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(err.error_code(), ErrorCode::Unauthorized);
    }

    #[test]
    fn database_maps_to_500() {
        let err = ConfigError::Database("connection refused".into());
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.error_code(), ErrorCode::Internal);
    }

    #[test]
    fn redis_maps_to_500() {
        let err = ConfigError::Redis("timeout".into());
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.error_code(), ErrorCode::Internal);
    }

    // ── Display (thiserror) ─────────────────────────────────────────────────

    #[test]
    fn display_includes_inner_message_for_not_found() {
        let err = ConfigError::NotFound("tenant config missing".into());
        assert!(err.to_string().contains("tenant config missing"));
    }

    #[test]
    fn display_includes_inner_message_for_validation() {
        let msg = "rate_limit_rpm must be positive";
        let err = ConfigError::Validation(msg.into());
        assert!(err.to_string().contains(msg));
    }

    // ── User-facing messages — no internal leakage ─────────────────────────

    #[test]
    fn database_user_message_is_generic() {
        let err = ConfigError::Database("SELECT * exploded due to null pointer".into());
        // The user must NOT see raw DB internals.
        let msg = err.user_message();
        assert!(!msg.contains("SELECT"));
        assert!(!msg.contains("null pointer"));
    }

    #[test]
    fn internal_user_message_is_generic() {
        let err = ConfigError::Internal("secret key: sk-proj-xxx leaked".into());
        let msg = err.user_message();
        assert!(!msg.contains("sk-proj"));
        assert!(!msg.contains("secret key"));
    }

    // ── From<sqlx::Error> classification ───────────────────────────────────

    #[test]
    fn sqlx_row_not_found_becomes_config_not_found() {
        let sqlx_err = sqlx::Error::RowNotFound;
        let config_err = ConfigError::from(sqlx_err);
        assert!(
            matches!(config_err, ConfigError::NotFound(_)),
            "expected NotFound, got {config_err:?}"
        );
    }

    // ── ErrorCode display ───────────────────────────────────────────────────

    #[test]
    fn error_code_as_str_is_stable() {
        assert_eq!(ErrorCode::Internal.as_str(), "INTERNAL_ERROR");
        assert_eq!(ErrorCode::BadRequest.as_str(), "BAD_REQUEST");
        assert_eq!(ErrorCode::Unauthorized.as_str(), "UNAUTHORIZED");
        assert_eq!(ErrorCode::NotFound.as_str(), "NOT_FOUND");
        assert_eq!(ErrorCode::Conflict.as_str(), "CONFLICT");
        assert_eq!(
            ErrorCode::UnprocessableEntity.as_str(),
            "UNPROCESSABLE_ENTITY"
        );
    }
}
