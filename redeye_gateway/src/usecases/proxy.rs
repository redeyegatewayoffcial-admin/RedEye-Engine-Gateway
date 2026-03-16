//! usecases/proxy.rs — Core proxy orchestration logic.
//!
//! This is the heart of the gateway: cache check → upstream call → async telemetry.
//! It is intentionally free of Axum types so it can be tested or reused independently.

use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info};

use crate::domain::models::{AppState, GatewayError, TraceContext};
use crate::infrastructure::{cache_client, clickhouse_logger, openai_client};

/// Result of a proxy execution — either a cached response or an upstream response.
pub struct ProxyResult {
    pub status: u16,
    pub content_type: String,
    pub body_bytes: Vec<u8>,
    pub cache_hit: bool,
}

/// Execute the full proxy pipeline: cache → upstream → async telemetry.
pub async fn execute_proxy(
    state: &Arc<AppState>,
    body: &Value,
    tenant_id: &str,
    model_name: &str,
    raw_prompt: &str,
    accept_header: &str,
    trace_ctx: &TraceContext,
) -> Result<ProxyResult, GatewayError> {
    let start_time = std::time::Instant::now();

    // ── 1. Semantic Cache Lookup ────────────────────────────────────────────
    if let Some(cached_content) = cache_client::lookup_cache(
        &state.http_client, tenant_id, model_name, raw_prompt
    ).await {
        let mock_response = json!({
            "id": "chatcmpl-cached",
            "object": "chat.completion",
            "created": 0,
            "model": model_name,
            "choices": [{"index": 0, "message": {"role": "assistant", "content": cached_content}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        });

        let bytes = serde_json::to_vec(&mock_response).unwrap_or_default();

        // Fire async telemetry even for cache hits
        fire_async_telemetry(
            state, tenant_id, model_name, raw_prompt, trace_ctx,
            200, start_time.elapsed().as_millis() as u32, 0, true, None,
        );

        return Ok(ProxyResult {
            status: 200,
            content_type: "application/json".to_string(),
            body_bytes: bytes,
            cache_hit: true,
        });
    }

    // ── 2. Forward to OpenAI ────────────────────────────────────────────────
    let upstream_response = openai_client::forward_chat_completion(
        &state.http_client, &state.openai_api_key, body, accept_header,
    ).await?;

    let upstream_status = upstream_response.status().as_u16();
    let content_type = upstream_response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    info!(status = upstream_status, "Received response from OpenAI");

    // Buffer the response for telemetry + cache storage
    let body_bytes = upstream_response.bytes().await.unwrap_or_default().to_vec();
    let latency_ms = start_time.elapsed().as_millis() as u32;

    // Extract token usage from OpenAI response
    let tokens = serde_json::from_slice::<Value>(&body_bytes)
        .ok()
        .and_then(|v| v["usage"]["total_tokens"].as_u64())
        .unwrap_or(0) as u32;

    // ── 3. Fire async telemetry ─────────────────────────────────────────────
    fire_async_telemetry(
        state, tenant_id, model_name, raw_prompt, trace_ctx,
        upstream_status, latency_ms, tokens, false,
        Some(body_bytes.clone()),
    );

    Ok(ProxyResult {
        status: upstream_status,
        content_type,
        body_bytes,
        cache_hit: false,
    })
}

/// Spawns a detached background task for ClickHouse logging, tracer ingestion, and cache storage.
fn fire_async_telemetry(
    state: &Arc<AppState>,
    tenant_id: &str,
    model_name: &str,
    raw_prompt: &str,
    trace_ctx: &TraceContext,
    status_code: u16,
    latency_ms: u32,
    tokens: u32,
    cache_hit: bool,
    response_bytes: Option<Vec<u8>>,
) {
    let s = state.clone();
    let tid = tenant_id.to_string();
    let model = model_name.to_string();
    let prompt = raw_prompt.to_string();
    let ctx = trace_ctx.clone();

    tokio::spawn(async move {
        // 3a. ClickHouse request_logs
        clickhouse_logger::log_request(
            &s.http_client, &s.clickhouse_url,
            &tid, &ctx.trace_id, &ctx.session_id,
            status_code, latency_ms, &model, tokens, cache_hit,
        ).await;

        // 3b. Send to redeye_tracer for deep tracing + compliance audit
        let response_content = response_bytes.as_ref()
            .and_then(|b| serde_json::from_slice::<Value>(b).ok())
            .and_then(|v| v["choices"][0]["message"]["content"].as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let trace_payload = json!({
            "trace_id": ctx.trace_id,
            "session_id": ctx.session_id,
            "parent_trace_id": ctx.parent_trace_id,
            "tenant_id": tid,
            "model": model,
            "status": status_code,
            "latency_ms": latency_ms,
            "total_tokens": tokens,
            "cache_hit": cache_hit,
            "prompt_content": prompt,
            "response_content": response_content
        });

        clickhouse_logger::send_trace_to_tracer(&s.http_client, &s.tracer_url, &trace_payload).await;

        // 3c. Cache storage (only on successful non-cached JSON responses)
        if !cache_hit && status_code == 200 && !response_content.is_empty() {
            cache_client::store_in_cache(&s.http_client, &tid, &model, &prompt, &response_content).await;
        }
    });
}
