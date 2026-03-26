//! usecases/proxy.rs — Core proxy orchestration logic.
//!
//! Pipeline: compliance redaction → cache check → upstream call → async telemetry.
//! Intentionally free of Axum types so it can be tested or reused independently.
//!
//! ## Architecture
//! Each pipeline stage is isolated in its own `#[inline]` helper, keeping
//! `execute_proxy` readable as a top-level orchestrator.  All variables and
//! external interfaces are unchanged.

use std::sync::{Arc, OnceLock};

use axum::response::sse::Event;
use eventsource_stream::Eventsource;
use futures::stream::StreamExt;
use regex::Regex;
use serde_json::{json, Value};
use sqlx::Row;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info, warn};

use crate::domain::models::{AppState, GatewayError, TraceContext};
use crate::infrastructure::{cache_client, clickhouse_logger, llm_router};

// ── Public types ─────────────────────────────────────────────────────────────

pub enum ProxyBody {
    Buffered(Vec<u8>),
    SseStream(ReceiverStream<Result<Event, axum::Error>>),
}

pub struct ProxyResult {
    pub status: u16,
    pub content_type: String,
    pub body: ProxyBody,
    pub cache_hit: bool,
}

// ── Constants ─────────────────────────────────────────────────────────────────

/// Token-bucket capacity initialised for each new tenant (tokens / day).
const TOKEN_BUCKET_CAPACITY: u64 = 100_000;
/// Token-bucket TTL in seconds (24 h).
const TOKEN_BUCKET_TTL_SECS: u64 = 86_400;
/// Circuit-breaker error window (seconds).
const CB_ERROR_WINDOW_SECS: u64 = 10;
/// Circuit-breaker open TTL (seconds).
const CB_OPEN_TTL_SECS: u64 = 60;
/// Errors within the window that trip the circuit.
const CB_ERROR_THRESHOLD: i32 = 2;
/// Provider API key Redis TTL (seconds).
const API_KEY_CACHE_TTL_SECS: u64 = 300;
/// Rough chars-per-token estimate for prompt token counting.
const CHARS_PER_TOKEN: usize = 4;
/// Fallback model used when the primary circuit is open.
const FALLBACK_MODEL: &str = "llama3-8b-8192";

// ── PII regex (compiled once) ─────────────────────────────────────────────────

fn pii_regex() -> &'static Regex {
    static PII_REGEX: OnceLock<Regex> = OnceLock::new();
    PII_REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:\d[ -]*?){13,16}\b|\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b|\b(?:\+\d{1,2}\s)?\(?\d{3}\)?[\s.-]\d{3}[\s.-]\d{4}\b",
        )
        .unwrap()
    })
}

// ── Main entry point ──────────────────────────────────────────────────────────

/// Execute the full proxy pipeline: compliance → cache → upstream → telemetry.
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

    // ── Stage 0: PII Redaction (fail-closed) ─────────────────────────────────
    let raw_prompt_owned = raw_prompt.to_string();
    let is_pii_match = tokio::task::spawn_blocking(move || {
        pii_regex().is_match(&raw_prompt_owned)
    })
    .await
    .map_err(|e| GatewayError::ResponseBuild(format!("Regex task error: {}", e)))?;

    let sanitized_storage: Option<Value> = if is_pii_match {
        Some(call_compliance_redact(&state.http_client, &state.compliance_url, body).await?)
    } else {
        None
    };
    let body = sanitized_storage.as_ref().unwrap_or(body);

    // ── Stage 1: Semantic Cache Lookup ────────────────────────────────────────
    if let Some(cached_content) =
        cache_client::lookup_cache(&state.http_client, &state.cache_url, tenant_id, model_name, raw_prompt).await
    {
        return handle_cache_hit(state, cached_content, tenant_id, model_name, raw_prompt, trace_ctx, start_time);
    }

    // ── Stage 1.5: Token-Bucket Rate Limit ────────────────────────────────────
    enforce_token_bucket(state, tenant_id, raw_prompt).await?;

    // ── Stage 1.6: Behavior Control (Loop Detection) ─────────────────────────
    if let Err(e) = crate::usecases::behavior_guard::enforce_loop_detection(
        state,
        &trace_ctx.session_id,
        body,
    )
    .await
    {
        let latency_ms = start_time.elapsed().as_millis() as u32;
        let payload = serde_json::to_value(crate::domain::models::LogPayload {
            id: uuid::Uuid::new_v4().to_string(),
            trace_id: trace_ctx.trace_id.clone(),
            session_id: trace_ctx.session_id.clone(),
            parent_trace_id: trace_ctx.parent_trace_id.clone(),
            tenant_id: tenant_id.to_string(),
            model: "__loop_blocked".to_string(),
            status: 429,
            latency_ms,
            tokens: 0,
            total_tokens: 0,
            cache_hit: false,
            prompt_content: raw_prompt.to_string(),
            response_content: "".to_string(),
            error_message: e.to_string(),
        }).unwrap_or_else(|_| json!({}));
        
        let _ = state.telemetry_tx.send(payload.clone()).await;
        crate::infrastructure::clickhouse_logger::send_trace_to_tracer(&state.http_client, &state.tracer_url, &payload).await;

        return Err(e);
    }

    // ── Stage 1.7: Burn-Rate Control ─────────────────────────────────────────
    if let Err(e) = crate::usecases::behavior_guard::enforce_burn_rate(
        state,
        &trace_ctx.session_id,
    )
    .await
    {
        let latency_ms = start_time.elapsed().as_millis() as u32;
        let payload = serde_json::to_value(crate::domain::models::LogPayload {
            id: uuid::Uuid::new_v4().to_string(),
            trace_id: trace_ctx.trace_id.clone(),
            session_id: trace_ctx.session_id.clone(),
            parent_trace_id: trace_ctx.parent_trace_id.clone(),
            tenant_id: tenant_id.to_string(),
            model: "__burn_rate_blocked".to_string(),
            status: 429,
            latency_ms,
            tokens: 0,
            total_tokens: 0,
            cache_hit: false,
            prompt_content: raw_prompt.to_string(),
            response_content: "".to_string(),
            error_message: e.to_string(),
        }).unwrap_or_else(|_| json!({}));

        let _ = state.telemetry_tx.send(payload.clone()).await;
        crate::infrastructure::clickhouse_logger::send_trace_to_tracer(&state.http_client, &state.tracer_url, &payload).await;

        return Err(e);
    }

    // ── Stage 1.8: Circuit-Breaker / Adaptive Fallback ────────────────────────
    let (mut resolved_model, mut provider) = select_provider_with_fallback(state, tenant_id, model_name).await;

    // ── Stage 2: Resolve Provider API Key ─────────────────────────────────────
    let resolved_key = resolve_api_key(state, tenant_id, provider.as_str()).await?;

    // ── Stage 3: Route to LLM (with circuit-breaker retry) ────────────────────
    let upstream_response = route_with_circuit_breaker(
        state, tenant_id, body, accept_header,
        &mut resolved_model, &mut provider, &resolved_key,
    )
    .await?;

    let upstream_status = upstream_response.status().as_u16();
    let content_type = upstream_response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    info!(status = upstream_status, provider = provider.as_str(), "Received response from upstream LLM provider");

    let is_streaming = body.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);

    // ── Stage 4a: SSE Streaming Path ─────────────────────────────────────────
    if is_streaming {
        return handle_streaming_response(
            state, upstream_response, upstream_status, content_type,
            tenant_id, model_name, raw_prompt, trace_ctx, start_time,
        );
    }

    // ── Stage 4b: Buffered (non-streaming) Path ───────────────────────────────
    handle_buffered_response(
        state, upstream_response, upstream_status, content_type,
        tenant_id, model_name, raw_prompt, trace_ctx, start_time,
    )
    .await
}

// ── Pipeline stage helpers ────────────────────────────────────────────────────

/// Returns a `ProxyResult` from a semantic-cache hit and fires background telemetry.
fn handle_cache_hit(
    state: &Arc<AppState>,
    cached_content: String,
    tenant_id: &str,
    model_name: &str,
    raw_prompt: &str,
    trace_ctx: &TraceContext,
    start_time: std::time::Instant,
) -> Result<ProxyResult, GatewayError> {
    let mock_response = json!({
        "id": "chatcmpl-cached",
        "object": "chat.completion",
        "created": 0,
        "model": model_name,
        "choices": [{"index": 0, "message": {"role": "assistant", "content": cached_content}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    });

    let bytes = serde_json::to_vec(&mock_response).unwrap_or_default();
    let latency_ms = start_time.elapsed().as_millis() as u32;

    spawn_telemetry(
        state, tenant_id, model_name, raw_prompt, trace_ctx,
        200, latency_ms, 0, true, None,
    );

    Ok(ProxyResult {
        status: 200,
        content_type: "application/json".to_string(),
        body: ProxyBody::Buffered(bytes),
        cache_hit: true,
    })
}

/// Enforces the per-tenant token-bucket rate limit via a Redis Lua script.
/// Returns `Err(RateLimitExceeded)` when the bucket is empty.
async fn enforce_token_bucket(
    state: &Arc<AppState>,
    tenant_id: &str,
    raw_prompt: &str,
) -> Result<(), GatewayError> {
    let estimated_tokens = (raw_prompt.len() / CHARS_PER_TOKEN).max(1);

    // Atomically check and decrement the tenant token bucket.
    let lua_script = r#"
        local current = redis.call('GET', KEYS[1])
        if current then
            local val = tonumber(current)
            if val < tonumber(ARGV[1]) then
                return 0
            else
                redis.call('DECRBY', KEYS[1], ARGV[1])
                return 1
            end
        else
            redis.call('SET', KEYS[1], ARGV[2], 'EX', ARGV[3])
            redis.call('DECRBY', KEYS[1], ARGV[1])
            return 1
        end
    "#;

    let bucket_key = format!("tb:tokens:{}", tenant_id);
    let mut conn = state.redis_conn.clone();

    let allowed: Option<i32> = redis::Script::new(lua_script)
        .key(&bucket_key)
        .arg(estimated_tokens)
        .arg(TOKEN_BUCKET_CAPACITY)
        .arg(TOKEN_BUCKET_TTL_SECS)
        .invoke_async(&mut conn)
        .await
        .ok();

    if allowed == Some(0) {
        warn!(tenant_id = %tenant_id, "Token bucket exhausted");
        return Err(GatewayError::RateLimitExceeded(
            "Token limit exceeded for this billing cycle.".into(),
        ));
    }

    Ok(())
}

/// Returns the `(model, provider)` pair to use, switching to the Groq fallback
/// when the primary provider's circuit is open in Redis.
async fn select_provider_with_fallback(
    state: &Arc<AppState>,
    tenant_id: &str,
    model_name: &str,
) -> (String, llm_router::LlmProvider) {
    let mut model = model_name.to_string();
    let mut provider = llm_router::LlmProvider::detect(&model);

    let cb_key = format!("cb:open:{}:{}", tenant_id, provider.as_str());
    let mut conn = state.redis_conn.clone();

    use redis::AsyncCommands;
    let is_open: Option<String> = conn.get(&cb_key).await.ok();

    if is_open.is_some() {
        warn!(provider = provider.as_str(), "Primary circuit is OPEN — falling back to Groq");
        model = FALLBACK_MODEL.to_string();
        provider = llm_router::LlmProvider::Groq;
    }

    (model, provider)
}

/// Looks up the API key for `(tenant_id, provider)`, checking Redis before the DB.
/// Caches DB results back into Redis with a short TTL.
async fn resolve_api_key(
    state: &Arc<AppState>,
    tenant_id: &str,
    provider_str: &str,
) -> Result<String, GatewayError> {
    use redis::AsyncCommands;

    let redis_key = format!("tenant_provider_key:{}:{}", tenant_id, provider_str);
    let mut conn = state.redis_conn.clone();

    // Fast path: Redis cache hit
    if let Ok(cached_key) = conn.get::<_, String>(&redis_key).await {
        return Ok(cached_key);
    }

    // Slow path: Postgres lookup
    let row = sqlx::query(
        "SELECT encrypted_api_key FROM llm_routes WHERE tenant_id = $1 AND provider = $2",
    )
    .bind(uuid::Uuid::parse_str(tenant_id).unwrap_or_default())
    .bind(provider_str)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!("DB error fetching provider key: {}", e);
        GatewayError::ResponseBuild("Internal server error".into())
    })?;

    let Some(r) = row else {
        warn!(tenant_id = %tenant_id, provider = provider_str, "Provider API key not configured");
        return Err(GatewayError::ResponseBuild("Provider API key not configured".into()));
    };

    let encrypted_data: Vec<u8> = r.get(0);
    let decrypted = crate::api::middleware::auth::decrypt_api_key(&encrypted_data)
        .map_err(|_| GatewayError::ResponseBuild("Failed to decrypt API key".into()))?;

    // Write-through to Redis
    let _: Result<(), _> = conn.set_ex(&redis_key, &decrypted, API_KEY_CACHE_TTL_SECS).await;

    Ok(decrypted)
}

/// Routes the request to the upstream LLM.  On 5xx / timeout:
/// - Increments the circuit-breaker error counter.
/// - Opens the circuit when the threshold is reached.
/// - Retries immediately on Groq (if not already Groq).
async fn route_with_circuit_breaker(
    state: &Arc<AppState>,
    tenant_id: &str,
    body: &Value,
    accept_header: &str,
    resolved_model: &mut String,
    provider: &mut llm_router::LlmProvider,
    resolved_key: &str,
) -> Result<reqwest::Response, GatewayError> {
    let mut response_res =
        llm_router::route_chat_completion(&state.http_client, resolved_key, body, accept_header, state.llm_api_base_url.as_deref()).await;

    let is_failure = match &response_res {
        Ok(r) if r.status().is_server_error() => true,
        Err(_) => true,
        _ => false,
    };

    if is_failure {
        record_circuit_breaker_error(state, tenant_id, provider.as_str()).await;

        // Immediate seamless fallback to Groq if primary was not already Groq.
        if *provider != llm_router::LlmProvider::Groq {
            info!("Attempting seamless immediate fallback to Groq");
            *resolved_model = FALLBACK_MODEL.to_string();

            let groq_key = resolve_provider_key_from_db(state, tenant_id, "groq").await?;

            let mut fallback_body = body.clone();
            fallback_body["model"] = json!(FALLBACK_MODEL);

            response_res =
                llm_router::route_chat_completion(&state.http_client, &groq_key, &fallback_body, accept_header, state.llm_api_base_url.as_deref()).await;
        }
    }

    response_res.map_err(Into::into)
}

/// Increments the per-`(tenant, provider)` error counter and opens the circuit
/// when the threshold is reached.
async fn record_circuit_breaker_error(state: &Arc<AppState>, tenant_id: &str, provider_str: &str) {
    use redis::AsyncCommands;

    error!(provider = provider_str, "Primary provider failed — tracking circuit-breaker error");

    let mut conn = state.redis_conn.clone();
    let err_key = format!("cb:errors:{}:{}", tenant_id, provider_str);

    let count: i32 = conn.incr(&err_key, 1).await.unwrap_or(0);
    let _: Result<(), _> = conn.expire(&err_key, CB_ERROR_WINDOW_SECS as i64).await;

    if count >= CB_ERROR_THRESHOLD {
        error!("Circuit breaker triggered — opening circuit for {}", provider_str);
        let open_key = format!("cb:open:{}:{}", tenant_id, provider_str);
        let _: Result<(), _> = conn.set_ex(&open_key, "1", CB_OPEN_TTL_SECS).await;
    }
}

/// Fetches and decrypts an API key directly from Postgres (skips Redis).
/// Used during fallback when the primary circuit has opened.
async fn resolve_provider_key_from_db(
    state: &Arc<AppState>,
    tenant_id: &str,
    provider_str: &str,
) -> Result<String, GatewayError> {
    let row = sqlx::query(
        "SELECT encrypted_api_key FROM llm_routes WHERE tenant_id = $1 AND provider = $2",
    )
    .bind(uuid::Uuid::parse_str(tenant_id).unwrap_or_default())
    .bind(provider_str)
    .fetch_optional(&state.db_pool)
    .await
    .ok()
    .flatten();

    let r = row.ok_or_else(|| GatewayError::ResponseBuild("Fallback provider key not found".into()))?;
    let encrypted_data: Vec<u8> = r.try_get(0).unwrap_or_default();
    crate::api::middleware::auth::decrypt_api_key(&encrypted_data)
        .map_err(|_| GatewayError::ResponseBuild("Failed to decrypt fallback API key".into()))
}

/// Handles SSE streaming: fans out events to a channel, fires telemetry on completion.
fn handle_streaming_response(
    state: &Arc<AppState>,
    upstream_response: reqwest::Response,
    upstream_status: u16,
    content_type: String,
    tenant_id: &str,
    model_name: &str,
    raw_prompt: &str,
    trace_ctx: &TraceContext,
    start_time: std::time::Instant,
) -> Result<ProxyResult, GatewayError> {
    let event_stream = upstream_response.bytes_stream().eventsource();
    let (tx, rx) = mpsc::channel(100);

    // Owned copies for the spawned task
    let state_c = state.clone();
    let tenant_id_c = tenant_id.to_string();
    let model_name_c = model_name.to_string();
    let raw_prompt_c = raw_prompt.to_string();
    let trace_ctx_c = trace_ctx.clone();

    tokio::spawn(async move {
        let mut event_stream = event_stream;

        while let Some(Ok(event)) = event_stream.next().await {
            if event.data == "[DONE]" {
                let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
                break;
            }

            // Pass through verbatim to achieve true zero-copy streaming
            let _ = tx.send(Ok(Event::default().data(event.data))).await;
        }

        // Stream complete — log metadata only (no body buffering)
        let latency_ms = start_time.elapsed().as_millis() as u32;

        fire_async_telemetry(
            &state_c, &tenant_id_c, &model_name_c, &raw_prompt_c, &trace_ctx_c,
            upstream_status, latency_ms, 0, false, None,
        )
        .await;
    });

    Ok(ProxyResult {
        status: upstream_status,
        content_type,
        body: ProxyBody::SseStream(ReceiverStream::new(rx)),
        cache_hit: false,
    })
}

/// Handles buffered (non-streaming) response: reads body, extracts token usage,
/// fires background telemetry.
async fn handle_buffered_response(
    state: &Arc<AppState>,
    upstream_response: reqwest::Response,
    upstream_status: u16,
    content_type: String,
    tenant_id: &str,
    model_name: &str,
    raw_prompt: &str,
    trace_ctx: &TraceContext,
    start_time: std::time::Instant,
) -> Result<ProxyResult, GatewayError> {
    let body_bytes = upstream_response.bytes().await.unwrap_or_default().to_vec();
    let latency_ms = start_time.elapsed().as_millis() as u32;

    let tokens = serde_json::from_slice::<Value>(&body_bytes)
        .ok()
        .and_then(|v| v["usage"]["total_tokens"].as_u64())
        .unwrap_or(0) as u32;

    spawn_telemetry(
        state, tenant_id, model_name, raw_prompt, trace_ctx,
        upstream_status, latency_ms, tokens, false, Some(body_bytes.clone()),
    );

    Ok(ProxyResult {
        status: upstream_status,
        content_type,
        body: ProxyBody::Buffered(body_bytes),
        cache_hit: false,
    })
}

// ── Telemetry helpers ─────────────────────────────────────────────────────────

/// Convenience wrapper: spawns `fire_async_telemetry` in a detached Tokio task.
fn spawn_telemetry(
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
        fire_async_telemetry(&s, &tid, &model, &prompt, &ctx, status_code, latency_ms, tokens, cache_hit, response_bytes).await;
    });
}

/// Formats telemetry properties and dispatches them to the background worker channel.
async fn fire_async_telemetry(
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
    // ── Record session spend for burn-rate tracking ──────────────────────────
    crate::usecases::behavior_guard::record_session_spend(state, &trace_ctx.session_id, tokens).await;

    let response_content = extract_response_content(response_bytes.as_deref());

    // 1. Clickhouse bulk batched payload
    let payload = json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "tenant_id": tenant_id,
        "status": status_code,
        "latency_ms": latency_ms,
        "model": model_name,
        "tokens": tokens
    });

    if let Err(e) = state.telemetry_tx.send(payload).await {
        error!(error = %e, "Failed to send JSON telemetry payload to background worker");
    }

    // 2. Tracer deep payload sent synchronously
    let trace_payload = json!({
        "trace_id":        trace_ctx.trace_id,
        "session_id":      trace_ctx.session_id,
        "parent_trace_id": trace_ctx.parent_trace_id,
        "tenant_id":       tenant_id,
        "model":           model_name,
        "status":          status_code,
        "latency_ms":      latency_ms,
        "total_tokens":    tokens,
        "cache_hit":       cache_hit,
        "prompt_content":  raw_prompt,
        "response_content": response_content
    });

    clickhouse_logger::send_trace_to_tracer(&state.http_client, &state.tracer_url, &trace_payload).await;

    // 3. Cache injection for successful misses
    if !cache_hit && status_code == 200 && !response_content.is_empty() {
        cache_client::store_in_cache(
            &state.http_client, &state.cache_url,
            tenant_id, model_name, raw_prompt, &response_content,
        )
        .await;
    }
}

// ── Small pure helpers ────────────────────────────────────────────────────────

/// Extracts `choices[0].message.content` from a raw OpenAI-format JSON body.
fn extract_response_content(bytes: Option<&[u8]>) -> String {
    bytes
        .and_then(|b| serde_json::from_slice::<Value>(b).ok())
        .and_then(|v| v["choices"][0]["message"]["content"].as_str().map(str::to_string))
        .unwrap_or_default()
}

// ── Compliance helper ─────────────────────────────────────────────────────────

/// POSTs `payload` to the compliance redaction endpoint and returns the
/// sanitised body.  Implements strict **fail-closed** semantics: any network
/// error, non-2xx status, or missing `sanitized_payload` field aborts the
/// request, preventing raw PII from reaching the upstream LLM provider.
async fn call_compliance_redact(
    http_client: &reqwest::Client,
    compliance_url: &str,
    payload: &Value,
) -> Result<Value, GatewayError> {
    let endpoint = format!("{}/api/v1/compliance/redact", compliance_url.trim_end_matches('/'));
    debug!(endpoint = %endpoint, "Calling compliance redaction service");

    let resp = http_client
        .post(&endpoint)
        .json(payload)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Compliance service unreachable — blocking request (fail-closed)");
            GatewayError::ComplianceFailure(format!("Compliance service unreachable: {}", e))
        })?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        error!(status = status, "Compliance service returned non-2xx — blocking request");
        return Err(GatewayError::ComplianceFailure(
            format!("Compliance service returned HTTP {}", status),
        ));
    }

    let compliance_json: Value = resp.json().await.map_err(|e| {
        error!(error = %e, "Failed to parse compliance response body");
        GatewayError::ComplianceFailure(format!("Malformed compliance response: {}", e))
    })?;

    match compliance_json.get("sanitized_payload").cloned() {
        Some(sanitized) => {
            info!("PII redaction complete — forwarding sanitised payload to upstream LLM");
            Ok(sanitized)
        }
        None => {
            error!("Compliance response missing `sanitized_payload` field — blocking request");
            Err(GatewayError::ComplianceFailure(
                "Compliance response did not contain `sanitized_payload`".into(),
            ))
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // super::* use panna, mela irukkura pii_regex() function inga kedaikum
    use super::*;

    #[test]
    fn test_pii_regex_matches_credit_card() {
        let regex = pii_regex();
        let prompt = "My credit card number is 1234-5678-9012-3456, please don't share it.";
        // assert! na "ithu unmai nu prove pannu" nu artham. Match aagalana test fail aagum.
        assert!(regex.is_match(prompt)); 
    }

    #[test]
    fn test_pii_regex_matches_email() {
        let regex = pii_regex();
        let prompt = "Send the AI response to admin@nmmglobal.com";
        assert!(regex.is_match(prompt));
    }

    #[test]
    fn test_pii_regex_ignores_safe_prompt() {
        let regex = pii_regex();
        let prompt = "What is the capital of Tamil Nadu? Explain in 50 words.";
        // assert_eq! false check pannuthu. Safe prompt-a adhu block panna koodathu.
        assert_eq!(regex.is_match(prompt), false); 
    }

    #[tokio::test]
    async fn test_telemetry_mpsc_channel_flow() {
        use tokio::sync::mpsc;

        // 1. Channel create pandrom (Capacity 100 logs)
        let (tx, mut rx) = mpsc::channel::<String>(100);

        // 2. Gateway Producer: User request vantha udane channel-la data poduthu
        tokio::spawn(async move {
            tx.send("Log 1: Tenant A used 400 tokens".to_string()).await.unwrap();
            tx.send("Log 2: Tenant B latency 45ms".to_string()).await.unwrap();
        });

        // 3. Background Worker (Consumer): Channel-la irunthu data-va edukuthu
        let first_log = rx.recv().await.unwrap();
        assert_eq!(first_log, "Log 1: Tenant A used 400 tokens");

        let second_log = rx.recv().await.unwrap();
        assert_eq!(second_log, "Log 2: Tenant B latency 45ms");
        
        // Ippadi thaan channel ulla data pass aaguthu nu compile aagi prove aagidum!
    }
}