use serde::{Deserialize, Serialize};

/// Inbound trace + audit payload from the gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceIngestPayload {
    pub trace_id: String,
    pub session_id: String,
    pub parent_trace_id: Option<String>,
    pub tenant_id: String,
    pub model: String,
    pub status: u16,
    pub latency_ms: u32,
    pub total_tokens: u32,
    pub cache_hit: bool,
    pub prompt_content: String,
    pub response_content: String,
}

/// Query parameters for trace lookups.
#[derive(Debug, Deserialize)]
pub struct TraceQuery {
    pub session_id: Option<String>,
    pub tenant_id: Option<String>,
    pub limit: Option<u32>,
}

/// Query parameters for audit lookups.
#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub tenant_id: Option<String>,
    pub session_id: Option<String>,
    pub limit: Option<u32>,
}
