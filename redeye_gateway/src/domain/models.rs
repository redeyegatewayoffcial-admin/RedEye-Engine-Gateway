use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Application-wide shared state passed to every handler via Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub http_client: reqwest::Client,
    /// gRPC client to the L2 semantic cache (redeye_cache:50051).
    /// Replaces the old `cache_url` String — the channel is pooled at startup.
    pub cache_grpc_client: crate::infrastructure::cache_client::CacheGrpcClient,
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
    pub l1_cache: std::sync::Arc<crate::infrastructure::l1_cache::L1Cache>,
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
    pub requested_provider: String,
    pub executed_provider: String,
    pub is_hot_swapped: u8,
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
        };

        // Log internal errors with full details
        match &self {
            GatewayError::ResponseBuild(_) | GatewayError::Proxy(_) => {
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

/// The role of the message sender in the Universal Middleman Schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RedEyeRole {
    System,
    User,
    Assistant,
    Tool,
}

/// The content block within a message, supporting text, images, and tool invocations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RedEyeContent {
    Text {
        text: String,
    },
    ImageUrl {
        url: String,
    },
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    ToolResult {
        tool_id: String,
        content: String,
    },
}

/// A specific message in the conversation history, associating a role with a list of content blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedEyeMessage {
    pub role: RedEyeRole,
    pub content: Vec<RedEyeContent>,
}

/// The Universal Middleman Schema encompassing the whole conversation state for parsing on-the-fly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedEyeConversation {
    pub system_prompt: Option<String>,
    pub messages: Vec<RedEyeMessage>,
    pub tools: Option<Vec<serde_json::Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_universal_schema_serialization() {
        let json_str = r#"{
            "system_prompt": "You are a helpful assistant.",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": "Hello, what's in this image?"
                        },
                        {
                            "type": "image_url",
                            "url": "https://example.com/image.jpg"
                        }
                    ]
                },
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "tool_call",
                            "id": "call_123",
                            "name": "get_weather",
                            "arguments": { "location": "San Francisco" }
                        }
                    ]
                },
                {
                    "role": "tool",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_id": "call_123",
                            "content": "Sunny and 70 degrees"
                        }
                    ]
                }
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "description": "Get current weather"
                    }
                }
            ]
        }"#;

        // Verify Deserialization from JSON String
        let conversation: RedEyeConversation = serde_json::from_str(json_str).expect("Failed to deserialize Universal Schema JSON");
        
        assert_eq!(conversation.system_prompt.as_deref(), Some("You are a helpful assistant."));
        assert_eq!(conversation.messages.len(), 3);
        assert_eq!(conversation.messages[0].role, RedEyeRole::User);
        assert_eq!(conversation.messages[1].role, RedEyeRole::Assistant);
        assert_eq!(conversation.messages[2].role, RedEyeRole::Tool);

        // Verify Serialization back to JSON with no data loss
        let serialized_str = serde_json::to_string(&conversation).expect("Failed to serialize back to json");
        
        // Parse both as serde_json::Value to ignore whitespace formatting differences
        let original_val: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let serialized_val: serde_json::Value = serde_json::from_str(&serialized_str).unwrap();
        
        assert_eq!(original_val, serialized_val, "Serialized JSON did not match original cleanly");
    }
}

// ==============================================================================
// Virtual API Key Phase 1: Multi-LLM Architecture Models
// ==============================================================================

/// Account type for tenant workspaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    /// Individual user account (default)
    Individual,
    /// Team workspace account supporting multiple users and API keys
    Team,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Individual
    }
}

/// A tenant represents an organization or individual workspace.
/// All resources are scoped to a tenant.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
    pub onboarding_status: bool,
    /// Account type: 'individual' or 'team'
    pub account_type: AccountType,
}

/// A virtual API key issued to tenant applications for gateway authentication.
/// The raw key is never stored; only a SHA-256 hash is persisted.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub tenant_id: Uuid,
    /// SHA-256 hash of the raw key (hex-encoded)
    pub key_hash: String,
    /// Human-readable name for the key (e.g., "Default Project", "Dev Key")
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Supported LLM providers for the provider_keys table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ProviderName {
    OpenAI,
    Anthropic,
    Gemini,
    Groq,
}

/// An encrypted upstream LLM provider API key.
/// Each tenant can store multiple provider keys for multi-LLM support.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProviderKey {
    pub id: Uuid,
    pub tenant_id: Uuid,
    /// The LLM provider (e.g., 'openai', 'anthropic')
    pub provider_name: ProviderName,
    /// AES-256-GCM encrypted provider API key
    #[serde(skip_serializing)]
    pub encrypted_key: Vec<u8>,
    pub created_at: DateTime<Utc>,
}
