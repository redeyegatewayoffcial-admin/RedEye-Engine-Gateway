//! handlers.rs — Axum request handlers for NexusAI Gateway.
//!
//! This module contains all handler functions. Each handler:
//!   1. Extracts validated data from the request
//!   2. Applies any pre-processing (policy hooks will be added in Phase 3+)
//!   3. Forwards the request upstream via the shared `reqwest::Client`
//!   4. Streams the upstream response directly back to the caller

use serde::Deserialize;
use serde_json::{json, Value};
use std::{sync::Arc, time::Instant};

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use tokio_stream::StreamExt;
use tracing::{error, info, instrument};

use crate::AppState;

/// The upstream OpenAI chat completions endpoint.
const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

// ─────────────────────────────────────────────────────────────────────────────
// Health Check
// ─────────────────────────────────────────────────────────────────────────────

/// GET /health
///
/// Lightweight liveness probe. Returns HTTP 200 with a JSON body.
/// No database checks yet — those will be added in Phase 3+ as readiness probes.
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "nexus_gateway",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Chat Completions Proxy
// ─────────────────────────────────────────────────────────────────────────────

/// POST /v1/chat/completions
///
/// The core reverse-proxy handler. Flow:
///
/// ```text
///  Client ──POST──► Axum Handler
///                        │
///                        ├─ [Phase 3] Rate-limit check (Redis)
///                        ├─ [Phase 3] PII redaction policy
///                        │
///                        └──► OpenAI API
///                                  │
///                        ◄─ stream ┘
///                        │
///  Client ◄─ streaming ──┘
/// ```
///
/// For now (Phase 2) we do pure transparent proxying with minimal overhead.
/// The `#[instrument]` macro automatically attaches tracing spans to this function.
#[instrument(skip(state, body))]
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    // We accept raw `Value` so we forward the exact JSON the client sent —
    // no deserializing into a rigid struct that might reject unknown fields.
    Json(body): Json<Value>,
) -> Result<Response, GatewayError> {
    let start_time = Instant::now();
    info!("Received chat completion request");

    // Extract telemetry metadata before forwarding
    let model_name = body.get("model").and_then(|m| m.as_str()).unwrap_or("unknown").to_string();
    let tenant_id = headers
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string();

    // ── Build upstream request ─────────────────────────────────────────────
    // We inject the server-side API key, so the client never needs to supply one.
    let upstream_response = state
        .http_client
        .post(OPENAI_CHAT_URL)
        .header("Authorization", format!("Bearer {}", state.openai_api_key))
        .header("Content-Type", "application/json")
        // Forward the Accept header if present (e.g., "text/event-stream" for streaming)
        .header(
            "Accept",
            headers
                .get("accept")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json"),
        )
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to reach OpenAI upstream");
            GatewayError::UpstreamUnreachable(e)
        })?;

    // ── Capture upstream status ────────────────────────────────────────────
    let upstream_status = upstream_response.status();
    let upstream_headers = upstream_response.headers().clone();

    info!(
        status = upstream_status.as_u16(),
        "Received response from OpenAI"
    );

    // ── Stream response back ───────────────────────────────────────────────
    // We convert `reqwest`'s bytes stream into an `axum` `Body` stream.
    // This means we NEVER buffer the entire response in memory —
    // critical for large LLM responses and Server-Sent Events (SSE) streaming.
    let byte_stream = upstream_response
        .bytes_stream()
        .map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));

    let body = Body::from_stream(byte_stream);

    // ── Build response, forwarding relevant upstream headers ──────────────
    let mut response_builder = Response::builder().status(upstream_status);

    // Forward content-type so the client knows if it's JSON or SSE
    if let Some(content_type) = upstream_headers.get("content-type") {
        response_builder = response_builder.header("content-type", content_type);
    }

    let response = response_builder.body(body).map_err(|e| {
        error!(error = %e, "Failed to construct proxy response");
        GatewayError::ResponseBuild(e.to_string())
    })?;

    // ── Async Telemetry Logging ────────────────────────────────────────────
    let latency_ms = start_time.elapsed().as_millis() as u32;
    let status_code = upstream_status.as_u16();
    
    // Extracted payload token estimation (simplified for Phase 4)
    // In production we would use `tiktoken-rs` or read the OpenAI usage block.
    let estimated_tokens = 50; 

    // We must perfectly clone all strings to cross the thread boundary safely.
    let log_state = state.clone();
    
    // Spawn a detached background task so Axum returns instantly to the client!
    tokio::spawn(async move {
        // Construct the ClickHouse JSONEachRow payload
        let log_entry = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "tenant_id": tenant_id,
            "status": status_code,
            "latency_ms": latency_ms,
            "model": model_name,
            "tokens": estimated_tokens
        });

        // Use our existing keep-alive HTTP pool to POST the insert.
        // ClickHouse async_insert=1 will batch this rapidly in memory.
        let result = log_state
            .http_client
            .post(format!("{}/?query=INSERT INTO nexusai_telemetry.request_logs FORMAT JSONEachRow", log_state.clickhouse_url))
            .json(&log_entry)
            .send()
            .await;

        match result {
            Ok(r) if !r.status().is_success() => {
                let err_text = r.text().await.unwrap_or_default();
                error!(error = ?err_text, "ClickHouse insertion rejected");
            }
            Err(e) => {
                error!(error = %e, "ClickHouse network failure during async log");
            }
            _ => {
                tracing::debug!("Successfully wrote async telemetry row to ClickHouse");
            }
        }
    });

    Ok(response)
}

// ==============================================================================
// ── Admin Handlers (Phase 5)                                                 ──
// ==============================================================================

/// Fetches aggregated metrics from ClickHouse for the React dashboard.
#[instrument(skip(state))]
pub async fn admin_metrics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, GatewayError> {
    info!("Fetching live metrics from ClickHouse");

    // We use ClickHouse's highly optimized JSON format structure
    let query = "
        SELECT 
            count() as total_requests,
            avg(latency_ms) as avg_latency_ms,
            sum(tokens) as total_tokens,
            countIf(status = 429) as rate_limited_requests
        FROM nexusai_telemetry.request_logs
        FORMAT JSON
    ";

    let response = state
        .http_client
        .post(&state.clickhouse_url)
        .body(query)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to connect to ClickHouse metrics API");
            GatewayError::Proxy(e)
        })?;

    if !response.status().is_success() {
        let err = response.text().await.unwrap_or_default();
        error!(error = %err, "ClickHouse metrics query failed");
        return Err(GatewayError::ResponseBuild("Metrics query failed".to_string()));
    }

    let mut clickhouse_json: Value = response.json().await.map_err(|e| {
        error!(error = %e, "Failed to parse ClickHouse JSON");
        GatewayError::ResponseBuild(e.to_string())
    })?;

    // ClickHouse returns an array of rows under `.data`
    let row = clickhouse_json.get_mut("data")
        .and_then(|data| data.as_array_mut())
        .and_then(|arr| arr.pop())
        .unwrap_or_else(|| json!({
            "total_requests": "0",
            "avg_latency_ms": 0.0,
            "total_tokens": "0",
            "rate_limited_requests": "0"
        }));

    Ok(Json(row))
}

// ─────────────────────────────────────────────────────────────────────────────
// Error Types
// ─────────────────────────────────────────────────────────────────────────────

/// Typed errors for the gateway.
///
/// Using `thiserror` keeps error definitions declarative and ensures every
/// variant automatically implements `std::error::Error` with `.source()` chains.
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("Upstream LLM API unreachable: {0}")]
    UpstreamUnreachable(#[from] reqwest::Error),

    #[error("Failed to build proxy response: {0}")]
    ResponseBuild(String),

    #[error("Failed to execute internal proxy request: {0}")]
    Proxy(reqwest::Error),
}

/// Convert `GatewayError` into an Axum HTTP response.
///
/// This is the single place where internal errors become client-facing JSON.
/// We deliberately hide internal details to avoid leaking implementation info.
impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            GatewayError::UpstreamUnreachable(_) => (
                StatusCode::BAD_GATEWAY,
                "The upstream LLM service is currently unreachable.",
            ),
            GatewayError::ResponseBuild(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal error occurred while building the response.",
            ),
            GatewayError::Proxy(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal error occurred while communicating with backend cluster services.",
            ),
        };

        error!(error = %self, "Returning error response to client");

        let body = Json(serde_json::json!({
            "error": {
                "code": status.as_u16(),
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}
