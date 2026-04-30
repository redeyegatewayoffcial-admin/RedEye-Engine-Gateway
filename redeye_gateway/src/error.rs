use thiserror::Error;

/// Typed errors for the gateway.
#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("Missing or invalid API key")]
    Unauthorized,
    #[error("Failed to build proxy request/response: {0}")]
    ResponseBuild(String),
    #[error("LLM Provider unreachable: {0}")]
    UpstreamUnreachable(reqwest::Error),
    #[error("Compliance block: {0}")]
    ComplianceFailure(String),
    #[error("Rate Limit Exceeded: {0}")]
    RateLimitExceeded(String),
    #[error("Agent loop detected: {0}")]
    LoopDetected(String),
    #[error("Burn rate exceeded: {0}")]
    BurnRateExceeded(String),
    #[error("Gateway internal error: {0}")]
    Proxy(reqwest::Error),
    #[error("Model not configured for this tenant")]
    ModelNotConfigured,
    #[error("Routing state missing")]
    RoutingStateMissing,
    #[error("No active keys available for routing")]
    NoActiveKeys,
}

impl axum::response::IntoResponse for GatewayError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;

        let (status, code, message) = match &self {
            GatewayError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "Missing or invalid API key.",
            ),
            GatewayError::ResponseBuild(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "An internal error occurred while building the response.",
            ),
            GatewayError::UpstreamUnreachable(e) => {
                if e.is_timeout() {
                    (
                        StatusCode::GATEWAY_TIMEOUT,
                        "GATEWAY_TIMEOUT",
                        "The upstream request timed out.",
                    )
                } else {
                    (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "UPSTREAM_ERROR",
                        "The upstream LLM service is currently unreachable.",
                    )
                }
            },
            GatewayError::ComplianceFailure(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "COMPLIANCE_ERROR",
                "Request blocked: the compliance service is unavailable or rejected this payload.",
            ),
            GatewayError::RateLimitExceeded(_) => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                "Rate limit exceeded. Please try again later.",
            ),
            GatewayError::LoopDetected(_) => (
                StatusCode::TOO_MANY_REQUESTS,
                "AGENT_LOOP_DETECTED",
                "Agent recursive loop detected. This session has been blocked to prevent runaway costs.",
            ),
            GatewayError::BurnRateExceeded(_) => (
                StatusCode::TOO_MANY_REQUESTS,
                "BURN_RATE_EXCEEDED",
                "Session burn rate exceeded. Spending has been paused to prevent runaway costs.",
            ),
            GatewayError::Proxy(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "An internal error occurred while communicating with backend cluster services.",
            ),
            GatewayError::ModelNotConfigured => (
                StatusCode::BAD_REQUEST,
                "MODEL_NOT_CONFIGURED",
                "The requested model is not configured for this tenant.",
            ),
            GatewayError::RoutingStateMissing => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "ROUTING_STATE_MISSING",
                "Routing state is missing or unavailable.",
            ),
            GatewayError::NoActiveKeys => (
                StatusCode::SERVICE_UNAVAILABLE, // or 429 based on prompt ("503 Service Unavailable or 429 JSON explicitly stating 'All configured keys exhausted'")
                "ALL_KEYS_EXHAUSTED",
                "All configured keys exhausted.",
            ),
        };

        // Log internal errors with full details
        match &self {
            GatewayError::ResponseBuild(_) | GatewayError::Proxy(_) | GatewayError::RoutingStateMissing => {
                tracing::error!(
                    error_code = %code,
                    status = %status.as_u16(),
                    internal_details = %self,
                    "Internal gateway error occurred"
                );
            }
            _ => {
                tracing::warn!(
                    error_code = %code,
                    status = %status.as_u16(),
                    message = %message,
                    "Gateway client error occurred"
                );
            }
        }

        let body = Json(serde_json::json!({
            "error": {
                "code": code,
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}
