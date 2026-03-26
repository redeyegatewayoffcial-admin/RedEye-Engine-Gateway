use serde::{Deserialize, Serialize};

/// Application-wide shared state passed to every handler via Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub http_client: reqwest::Client,
    pub cache_url: String,
    pub compliance_url: String,
    pub redis_conn: redis::aio::MultiplexedConnection,
    pub db_pool: sqlx::PgPool,
    pub rate_limit_max: u32,
    pub rate_limit_window: u32,
    pub clickhouse_url: String,
    pub tracer_url: String,
    pub dashboard_url: String,
    pub llm_api_base_url: Option<String>,
    pub telemetry_tx: tokio::sync::mpsc::Sender<serde_json::Value>,
}

/// Trace context propagated through every request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: String,
    pub session_id: String,
    pub parent_trace_id: Option<String>,
}

/// Strongly-typed payload for telemetry ingestion to guarantee valid JSON serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogPayload {
    pub id: String,
    pub trace_id: String,
    pub session_id: String,
    pub parent_trace_id: Option<String>,
    pub tenant_id: String,
    pub model: String,
    pub status: u16,
    pub latency_ms: u32,
    pub tokens: u32,
    pub total_tokens: u32,
    pub cache_hit: bool,
    pub prompt_content: String,
    pub response_content: String,
    pub error_message: String,
}

/// Typed errors for the gateway.
#[derive(Debug, thiserror::Error)]
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
}

impl axum::response::IntoResponse for GatewayError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;

        let status = match self {
            GatewayError::Unauthorized => StatusCode::UNAUTHORIZED,
            GatewayError::ResponseBuild(_) => StatusCode::INTERNAL_SERVER_ERROR,
            GatewayError::UpstreamUnreachable(_) => StatusCode::BAD_GATEWAY,
            GatewayError::ComplianceFailure(_) => StatusCode::SERVICE_UNAVAILABLE,
            GatewayError::RateLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
            GatewayError::LoopDetected(_) => StatusCode::TOO_MANY_REQUESTS,
            GatewayError::BurnRateExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
            GatewayError::Proxy(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let message = match &self {
            GatewayError::Unauthorized => "Missing or invalid API key.",
            GatewayError::ResponseBuild(_) => "An internal error occurred while building the response.",
            GatewayError::UpstreamUnreachable(_) => "The upstream LLM service is currently unreachable.",
            GatewayError::ComplianceFailure(_) => "Request blocked: the compliance service is unavailable or rejected this payload.",
            GatewayError::RateLimitExceeded(_) => "Rate limit exceeded. Please try again later.",
            GatewayError::LoopDetected(_) => "Agent recursive loop detected. This session has been blocked to prevent runaway costs.",
            GatewayError::BurnRateExceeded(_) => "Session burn rate exceeded. Spending has been paused to prevent runaway costs.",
            GatewayError::Proxy(_) => "An internal error occurred while communicating with backend cluster services.",
        };

        tracing::error!(error = %self, "Returning error response to client");

        let body = Json(serde_json::json!({
            "error": {
                "code": status.as_u16(),
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}
