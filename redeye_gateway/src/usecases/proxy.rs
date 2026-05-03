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

use regex::Regex;
use serde_json::{json, Value};
use tracing::{debug, error, info, warn};

use crate::domain::models::{AppState, TraceContext};
use crate::error::GatewayError;
use crate::infrastructure::routing_strategy::RoutingStrategy;
use crate::infrastructure::{cache_client, clickhouse_logger, llm_router};

// ── Public types ─────────────────────────────────────────────────────────────

pub enum ProxyBody {
    Buffered(Vec<u8>),
    Stream(axum::body::Body),
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
/// Rough chars-per-token estimate for prompt token counting.
const CHARS_PER_TOKEN: usize = 4;

// ── PII regex (compiled once) ─────────────────────────────────────────────────

fn pii_regex() -> &'static Regex {
    static PII_REGEX: OnceLock<Regex> = OnceLock::new();
    PII_REGEX.get_or_init(|| {
        // Bug 5 Fix: ReDoS-safe regex — replaced backtracking-prone lazy quantifiers
        // like `(?:\d[ -]*?){13,16}` with fixed-width, DFA-compatible alternations.
        // Each alternative uses a concrete digit count and explicit separator positions,
        // preventing polynomial backtracking on adversarial inputs.
        //
        // Patterns covered:
        //   CC:     4×4-digit groups (space/dash/none separator)
        //   Email:  possessive char classes, bounded TLD {2,7}
        //   Phone:  US format with strict separator positions
        //   Aadhaar:4-4-4 digit groups
        //   IFSC:   4 alpha + 0 + 6 alnum
        //   Bank/SSN: 9–12 digit sequences (bounded, word-anchored)
        let pattern = concat!(
            // Credit card: 4×4 groups with space, dash or no separator
            r"\b\d{4}[[:space:]-]?\d{4}[[:space:]-]?\d{4}[[:space:]-]?\d{4}\b",
            // Email: bounded TLD prevents runaway backtracking
            r"|\b[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,7}\b",
            // US Phone: +1 optional, strict separator positions
            r"|\b(?:\+1[[:space:]])?\(?\d{3}\)?[[:space:].-]\d{3}[[:space:].-]\d{4}\b",
            // Aadhaar: 4-4-4 digit groups
            r"|\b\d{4}[[:space:]-]?\d{4}[[:space:]-]?\d{4}\b",
            // IFSC: 4 alpha + literal 0 + 6 alnum
            r"|\b[A-Z]{4}0[A-Z0-9]{6}\b",
            // Bank account / SSN-like: 9–12 consecutive digits, word-anchored
            r"|\b\d{9,12}\b"
        );
        Regex::new(pattern).expect("Invalid PII Regex")
    })
}

// ── Main entry point ──────────────────────────────────────────────────────────

fn get_requested_provider(state: &Arc<AppState>, model_name: &str) -> String {
    state.routing_state.state.load()
        .get(model_name)
        .map(|c| c.schema_format.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Execute the full proxy pipeline: compliance → cache → upstream → telemetry.
pub async fn execute_proxy(
    state: &Arc<AppState>,
    body_bytes: &axum::body::Bytes,
    tenant_id: &str,
    model_name: &str,
    raw_prompt: &str,
    accept_header: &str,
    trace_ctx: &TraceContext,
    strategy: RoutingStrategy,
) -> Result<ProxyResult, GatewayError> {
    let start_time = std::time::Instant::now();

    // ── Stage 0: PII Redaction (fail-closed) ─────────────────────────────────
    let raw_prompt_owned = raw_prompt.to_string();
    let is_pii_match = tokio::task::spawn_blocking(move || {
        pii_regex().is_match(&raw_prompt_owned)
    })
    .await
    .map_err(|e| GatewayError::ResponseBuild(format!("Regex task error: {}", e)))?;

    // Lazily evaluate JSON only if necessary
    let (body_bytes, parsed_value_if_needed): (axum::body::Bytes, Option<Value>) = if is_pii_match {
        let mut parsed_body: Value = serde_json::from_slice(body_bytes)
            .map_err(|e| GatewayError::ResponseBuild(format!("Invalid JSON body: {}", e)))?;
        
        let redacted = call_compliance_redact(&state.http_client, &state.compliance_url, &parsed_body, trace_ctx).await?;
        parsed_body = redacted;
        
        let redacted_bytes = serde_json::to_vec(&parsed_body)
            .map_err(|e| GatewayError::ResponseBuild(format!("Failed to serialize redacted body: {}", e)))?;
        (axum::body::Bytes::from(redacted_bytes), Some(parsed_body))
    } else {
        (body_bytes.clone(), None)
    };

    // ── Stage 1 & 2: Speculative Cache & Router Prep ──────────────────────────
    let cache_future = async {
        if let Some(cached_content) = state.l1_cache.get_exact(raw_prompt).await {
            return Some(cached_content);
        }
        if let Some(cached_content) = state.l1_cache.get_semantic(raw_prompt).await {
            return Some(cached_content);
        }
        cache_client::lookup_cache(
            &state.cache_grpc_client,
            tenant_id,
            model_name,
            raw_prompt,
            trace_ctx,
        )
        .await
    };

    let prep_future = llm_router::prep_upstream_request(
        state,
        tenant_id,
        &body_bytes,
        accept_header,
        strategy,
    );

    let (cache_result, prep_result) = tokio::join!(cache_future, prep_future);

    if let Some(cached_content) = cache_result {
        return handle_cache_hit(
            state,
            cached_content,
            tenant_id,
            model_name,
            raw_prompt,
            trace_ctx,
            start_time,
        );
    }

    let prep = prep_result?;

    // ── Stage 1.5 & 1.6: Pre-flight checks (Token-Bucket & Loop Detection) ─────
    let lazy_parsed = match parsed_value_if_needed {
        Some(v) => v,
        None => serde_json::from_slice(&body_bytes)
            .map_err(|e| GatewayError::ResponseBuild(format!("Invalid JSON body: {}", e)))?,
    };

    let token_future = enforce_token_bucket(state, tenant_id, raw_prompt);
    let loop_future = crate::usecases::behavior_guard::enforce_loop_detection(state, &trace_ctx.session_id, &lazy_parsed);

    if let Err(e) = tokio::try_join!(token_future, loop_future) {
        let latency_ms = start_time.elapsed().as_millis() as u32;

        let model_telemetry = match &e {
            GatewayError::RateLimitExceeded(_) => "__token_blocked",
            GatewayError::LoopDetected(_) => "__loop_blocked",
            _ => "__preflight_blocked",
        };

        // Bug 4 Fix: Fallback JSON now includes all ClickHouse-required schema fields.
        // An empty `json!({})` would be rejected by ClickHouse's strict column schema;
        // we supply sentinel values so the row is valid and auditable.
        let payload = serde_json::to_value(crate::domain::models::LogPayload {
            id: uuid::Uuid::new_v4().to_string(),
            trace_id: trace_ctx.trace_id.clone(),
            session_id: trace_ctx.session_id.clone(),
            parent_trace_id: trace_ctx.parent_trace_id.clone(),
            tenant_id: tenant_id.to_string(),
            model: model_telemetry.to_string(),
            status: 429,
            latency_ms,
            tokens: 0,
            total_tokens: 0,
            cache_hit: false,
            prompt_content: raw_prompt.to_string(),
            response_content: "".to_string(),
            error_message: e.to_string(),
            requested_provider: get_requested_provider(state, model_name),
            executed_provider: "".to_string(),
            is_hot_swapped: 0,
        })
        .unwrap_or_else(|_| {
            json!({
                // Bug 4 Fix: Schema-valid sentinel — never send an empty object to ClickHouse.
                "id": uuid::Uuid::new_v4().to_string(),
                "trace_id": trace_ctx.trace_id,
                "session_id": trace_ctx.session_id,
                "tenant_id": tenant_id,
                "model": model_telemetry,
                "status": 429_u16,
                "latency_ms": latency_ms,
                "tokens": 0_u32,
                "total_tokens": 0_u32,
                "cache_hit": false,
                "error": "serialization_failed"
            })
        });

        let _ = state.telemetry_tx.send(payload.clone()).await;
        crate::infrastructure::clickhouse_logger::send_trace_to_tracer(
            &state.http_client,
            &state.tracer_url,
            &payload,
        )
        .await;

        return Err(e);
    }

    // ── Stage 1.7: Burn-Rate Control ─────────────────────────────────────────
    if let Err(e) =
        crate::usecases::behavior_guard::enforce_burn_rate(state, &trace_ctx.session_id).await
    {
        let latency_ms = start_time.elapsed().as_millis() as u32;
        // Bug 4 Fix: Same schema-valid fallback for burn-rate guard events.
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
            requested_provider: get_requested_provider(state, model_name),
            executed_provider: "".to_string(),
            is_hot_swapped: 0,
        })
        .unwrap_or_else(|_| {
            json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "trace_id": trace_ctx.trace_id,
                "session_id": trace_ctx.session_id,
                "tenant_id": tenant_id,
                "model": "__burn_rate_blocked",
                "status": 429_u16,
                "latency_ms": latency_ms,
                "tokens": 0_u32,
                "total_tokens": 0_u32,
                "cache_hit": false,
                "error": "serialization_failed"
            })
        });

        let _ = state.telemetry_tx.send(payload.clone()).await;
        crate::infrastructure::clickhouse_logger::send_trace_to_tracer(
            &state.http_client,
            &state.tracer_url,
            &payload,
        )
        .await;

        return Err(e);
    }

    // ── Stage 1.8: Circuit-Breaker / Adaptive Fallback ────────────────────────
    let provider = get_requested_provider(state, model_name);

    // ── Stage 2 & 3: Dynamic Provider Key Resolution and Routing with Fallback ─
    let upstream_response = llm_router::execute_upstream_request(
        state,
        prep,
        &body_bytes, // Pass bytes for zero-copy proxy
    )
    .await?;

    let requested_provider = provider.to_string();
    let executed_provider = upstream_response
        .headers()
        .get("x-redeye-executed-provider")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(&provider)
        .to_string();
    let is_hot_swapped: u8 = upstream_response
        .headers()
        .get("x-redeye-hot-swapped")
        .map(|v| if v == "1" { 1 } else { 0 })
        .unwrap_or(0);

    let upstream_status = upstream_response.status().as_u16();
    let content_type = upstream_response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    info!(
        status = upstream_status,
        provider = executed_provider.as_str(),
        "Received response from upstream LLM provider"
    );

    let is_streaming = lazy_parsed
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // ── Stage 4a: SSE Streaming Path ─────────────────────────────────────────
    if is_streaming {
        return handle_streaming_response(
            state,
            upstream_response,
            upstream_status,
            content_type,
            tenant_id,
            model_name,
            raw_prompt,
            trace_ctx,
            start_time,
            requested_provider,
            executed_provider,
            is_hot_swapped,
        );
    }

    // ── Stage 4b: Buffered (non-streaming) Path ───────────────────────────────
    handle_buffered_response(
        state,
        upstream_response,
        upstream_status,
        content_type,
        tenant_id,
        model_name,
        raw_prompt,
        trace_ctx,
        start_time,
        requested_provider,
        executed_provider,
        is_hot_swapped,
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

    let bytes = serde_json::to_vec(&mock_response)
        .map_err(|e| GatewayError::ResponseBuild(format!("Failed to serialize mock response: {}", e)))?;
    let latency_ms = start_time.elapsed().as_millis() as u32;

    let requested_provider = get_requested_provider(state, model_name);

    spawn_telemetry(
        state,
        tenant_id,
        model_name,
        raw_prompt,
        trace_ctx,
        200,
        latency_ms,
        0,
        true,
        None,
        requested_provider.clone(),
        requested_provider,
        0,
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

/// Bug 3 Fix: Reconciles the Redis token bucket after the actual LLM usage is known.
///
/// At request time the bucket was debited by `estimated_tokens`. If the actual
/// usage was lower, we credit the difference back via `INCRBY`; if higher, we
/// debit the extra. This prevents long-term billing drift without requiring a
/// blocking call on the hot path.
async fn sync_token_bucket(
    state: &Arc<AppState>,
    tenant_id: &str,
    estimated_tokens: u32,
    actual_tokens: u32,
) {
    // A positive delta means we over-estimated and should refund the difference.
    // A negative delta means we under-estimated and should deduct the extra.
    let delta = estimated_tokens as i64 - actual_tokens as i64;
    if delta == 0 {
        return;
    }

    let bucket_key = format!("tb:tokens:{}", tenant_id);
    let mut conn = state.redis_conn.clone();

    // INCRBY with a positive delta refunds tokens; with a negative delta it deducts.
    // We use INCRBY (not DECRBY) so a single command covers both directions.
    let result: Result<(), _> = redis::cmd("INCRBY")
        .arg(&bucket_key)
        .arg(delta)
        .query_async(&mut conn)
        .await;

    if let Err(e) = result {
        warn!(
            tenant_id = %tenant_id,
            delta = delta,
            error = %e,
            "Token bucket sync failed — bucket may drift"
        );
    } else {
        debug!(
            tenant_id = %tenant_id,
            estimated = estimated_tokens,
            actual = actual_tokens,
            delta = delta,
            "Token bucket reconciled after LLM response"
        );
    }
}

/// Handles SSE streaming: zero-copy stream proxy, fires telemetry on completion.
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
    requested_provider: String,
    executed_provider: String,
    is_hot_swapped: u8,
) -> Result<ProxyResult, GatewayError> {
    let state_c = state.clone();
    let tenant_id_c = tenant_id.to_string();
    let model_name_c = model_name.to_string();
    let raw_prompt_c = raw_prompt.to_string();
    let trace_ctx_c = trace_ctx.clone();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<bytes::Bytes>();

    use futures::StreamExt;
    let stream = upstream_response.bytes_stream().map(move |chunk_res| {
        if let Ok(chunk) = &chunk_res {
            let _ = tx.send(chunk.clone());
        }
        chunk_res
    });

    let body_stream = axum::body::Body::from_stream(stream);

    // Spawn a lightweight task to parse tokens and record telemetry
    tokio::spawn(async move {
        let mut total_tokens = 0;
        let mut response_bytes = Vec::new();
        while let Some(chunk) = rx.recv().await {
            response_bytes.extend_from_slice(&chunk);
            if let Ok(text) = std::str::from_utf8(&chunk) {
                if text.contains("\"usage\"") {
                    if let Some(idx) = text.find("\"total_tokens\":") {
                        let sub = &text[idx + 15..];
                        let end = sub.find(|c: char| !c.is_ascii_digit()).unwrap_or(sub.len());
                        if let Ok(t) = sub[..end].trim().parse::<u32>() {
                            total_tokens = t;
                        }
                    }
                }
            }
        }

        if total_tokens == 0 {
            let estimated_prompt = (raw_prompt_c.len() / CHARS_PER_TOKEN).max(1) as u32;
            let estimated_completion = (response_bytes.len() / CHARS_PER_TOKEN).max(1) as u32;
            total_tokens = estimated_prompt + estimated_completion;
        }

        let latency_ms = start_time.elapsed().as_millis() as u32;

        sync_token_bucket(&state_c, &tenant_id_c, (raw_prompt_c.len() / CHARS_PER_TOKEN).max(1) as u32, total_tokens).await;

        fire_async_telemetry(
            &state_c,
            &tenant_id_c,
            &model_name_c,
            &raw_prompt_c,
            &trace_ctx_c,
            upstream_status,
            latency_ms,
            total_tokens,
            false,
            Some(response_bytes),
            requested_provider,
            executed_provider,
            is_hot_swapped,
        )
        .await;
    });

    Ok(ProxyResult {
        status: upstream_status,
        content_type,
        body: ProxyBody::Stream(body_stream),
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
    requested_provider: String,
    executed_provider: String,
    is_hot_swapped: u8,
) -> Result<ProxyResult, GatewayError> {
    let mut body_bytes = upstream_response.bytes().await
        .map_err(|e| GatewayError::ResponseBuild(format!("Failed to read upstream body: {}", e)))?.to_vec();
    let latency_ms = start_time.elapsed().as_millis() as u32;

    let config = crate::infrastructure::llm_router::get_provider_config(&executed_provider);
    let translator = crate::infrastructure::llm_router::get_translator(&config.schema_format);
    if let Ok(raw_json) = serde_json::from_slice::<Value>(&body_bytes) {
        match translator.unify_response(raw_json) {
            Ok(unified_json) => {
                if let Ok(unified_bytes) = serde_json::to_vec(&unified_json) {
                    body_bytes = unified_bytes;
                }
            }
            Err(e) => {
                warn!("Response unification error: {}", e);
            }
        }
    }

    let estimated_tokens = (raw_prompt.len() / CHARS_PER_TOKEN).max(1) as u32;

    // ── Tunnel 3 Phase 3: MCP Sandbox Firewall ───────────────────────────────
    // Inspect the (now-unified) response for tool_calls and apply OPA RBAC.
    // Fail-open: if OPA is unreachable or times out, the original bytes are
    // returned unchanged.  Zero overhead on the non-agentic path: the fast
    // simd-json scan returns immediately when no tool_calls key is present.
    let body_bytes = crate::usecases::behavior_guard::enforce_mcp_sandbox(
        state,
        tenant_id,
        body_bytes,
    )
    .await;

    // ── Tunnel 3 Phase 2: Phantom tool interception ───────────────────────────
    // If the LLM called the phantom `get_tool_details` tool, resolve the
    // schema from cache and replace body_bytes — no external call needed.
    let body_bytes = if !state.tool_registry.is_empty() {
        if let Some(phantom_resp) = state.tool_registry.intercept_phantom_call(&body_bytes) {
            tracing::debug!("Tunnel 3 Phase 2 — phantom call resolved from cache");
            phantom_resp
        } else {
            body_bytes
        }
    } else {
        body_bytes
    };

    // ── Tunnel 3 Phase 4: Flow-Based Parallel Fan-Out ────────────────────────
    // If the (sandbox-cleared) response contains tool_calls AND every called
    // tool is registered in the MCP registry, dispatch all calls concurrently.
    // Results are merged and forwarded to the telemetry channel so the next
    // agentic turn can include them as `role: "tool"` context messages.
    //
    // This block is a no-op when:
    //   • `body_bytes` contains no `tool_calls` key (byte-scan early-exit)
    //   • `mcp_registry` is empty (pre-fetching disabled)
    {
        let has_tool_calls_bytes = body_bytes
            .windows(b"tool_calls".len())
            .any(|w| w == b"tool_calls");

        if has_tool_calls_bytes && !state.mcp_registry.is_empty() {
            let calls =
                crate::infrastructure::mcp_client::extract_tool_calls(&body_bytes);

            if !calls.is_empty() {
                let results =
                    crate::infrastructure::mcp_client::fan_out(calls, state).await;

                if !results.is_empty() {
                    let tool_messages =
                        crate::infrastructure::mcp_client::merge_results(results);

                    // Forward merged tool results to telemetry as structured context.
                    let ctx_payload = serde_json::json!({
                        "type": "mcp_tool_results",
                        "tenant_id": tenant_id,
                        "tool_messages": tool_messages,
                    });
                    let _ = state.telemetry_tx.send(ctx_payload).await;
                }
            }
        }
    }

    let actual_tokens = serde_json::from_slice::<Value>(&body_bytes)
        .ok()
        .and_then(|v| v["usage"]["total_tokens"].as_u64())
        .map(|v| v as u32)
        .unwrap_or_else(|| {
            let estimated_completion = (body_bytes.len() / CHARS_PER_TOKEN).max(1) as u32;
            estimated_tokens + estimated_completion
        });

    // Bug 3 Fix: Reconcile the Redis token bucket with the actual usage reported by
    // the LLM provider. The rate-limiter deducted an *estimate* at request time;
    // now we compute the delta and credit any overestimation back, preventing drift
    // that would cause tenants to be incorrectly throttled over time.
    sync_token_bucket(state, tenant_id, estimated_tokens, actual_tokens).await;

    spawn_telemetry(
        state,
        tenant_id,
        model_name,
        raw_prompt,
        trace_ctx,
        upstream_status,
        latency_ms,
        actual_tokens,
        false,
        Some(body_bytes.clone()),
        requested_provider,
        executed_provider,
        is_hot_swapped,
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
    requested_provider: String,
    executed_provider: String,
    is_hot_swapped: u8,
) {
    let s = state.clone();
    let tid = tenant_id.to_string();
    let model = model_name.to_string();
    let prompt = raw_prompt.to_string();
    let ctx = trace_ctx.clone();

    tokio::spawn(async move {
        fire_async_telemetry(
            &s,
            &tid,
            &model,
            &prompt,
            &ctx,
            status_code,
            latency_ms,
            tokens,
            cache_hit,
            response_bytes,
            requested_provider,
            executed_provider,
            is_hot_swapped,
        )
        .await;
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
    requested_provider: String,
    executed_provider: String,
    is_hot_swapped: u8,
) {
    // ── Record session spend for burn-rate tracking ──────────────────────────
    crate::usecases::behavior_guard::record_session_spend(state, &trace_ctx.session_id, tokens)
        .await;

    let response_content = extract_response_content(response_bytes.as_deref());

    // 1. Clickhouse bulk batched payload
    let payload = json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "tenant_id": tenant_id,
        "status": status_code,
        "latency_ms": latency_ms,
        "model": model_name,
        "tokens": tokens,
        "requested_provider": requested_provider,
        "executed_provider": executed_provider,
        "is_hot_swapped": is_hot_swapped
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
        "response_content": response_content,
        "requested_provider": requested_provider,
        "executed_provider": executed_provider,
        "is_hot_swapped": is_hot_swapped
    });

    clickhouse_logger::send_trace_to_tracer(&state.http_client, &state.tracer_url, &trace_payload)
        .await;

    // 3. Cache injection for successful misses
    if !cache_hit && status_code == 200 && !response_content.is_empty() {
        // Smart Token-based Routing & Background Sync
        if tokens < 1000 {
            // Save in BOTH L1 and L2
            if let Err(e) = state.l1_cache.insert(raw_prompt, &response_content).await {
                tracing::warn!("Failed to store in L1 cache: {}", e);
            }
            cache_client::store_in_cache(
                &state.cache_grpc_client,
                tenant_id,
                model_name,
                raw_prompt,
                &response_content,
                trace_ctx,
            )
            .await;
        } else {
            // Tokens >= 1000: Bulk response, L2 only to prevent L1 OOM
            cache_client::store_in_cache(
                &state.cache_grpc_client,
                tenant_id,
                model_name,
                raw_prompt,
                &response_content,
                trace_ctx,
            )
            .await;
        }
    }
}

// ── Small pure helpers ────────────────────────────────────────────────────────

/// Extracts `choices[0].message.content` from a raw OpenAI-format JSON body.
fn extract_response_content(bytes: Option<&[u8]>) -> String {
    bytes
        .and_then(|b| serde_json::from_slice::<Value>(b).ok())
        .and_then(|v| {
            v["choices"][0]["message"]["content"]
                .as_str()
                .map(str::to_string)
        })
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
    trace_ctx: &TraceContext,
) -> Result<Value, GatewayError> {
    let endpoint = format!(
        "{}/api/v1/compliance/redact",
        compliance_url.trim_end_matches('/')
    );
    debug!(endpoint = %endpoint, "Calling compliance redaction service");

    let resp = http_client
        .post(&endpoint)
        .header("x-redeye-trace-id", &trace_ctx.trace_id)
        .header("x-redeye-session-id", &trace_ctx.session_id)
        .json(payload)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Compliance service unreachable — blocking request (fail-closed)");
            GatewayError::ComplianceFailure(format!("Compliance service unreachable: {}", e))
        })?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        error!(
            status = status,
            "Compliance service returned non-2xx — blocking request"
        );
        return Err(GatewayError::ComplianceFailure(format!(
            "Compliance service returned HTTP {}",
            status
        )));
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
            tx.send("Log 1: Tenant A used 400 tokens".to_string())
                .await
                .unwrap();
            tx.send("Log 2: Tenant B latency 45ms".to_string())
                .await
                .unwrap();
        });

        // 3. Background Worker (Consumer): Channel-la irunthu data-va edukuthu
        let first_log = rx.recv().await.unwrap();
        assert_eq!(first_log, "Log 1: Tenant A used 400 tokens");

        let second_log = rx.recv().await.unwrap();
        assert_eq!(second_log, "Log 2: Tenant B latency 45ms");

        // Ippadi thaan channel ulla data pass aaguthu nu compile aagi prove aagidum!
    }

    #[test]
    fn test_pii_rayon_short_circuit() {
        let regex = pii_regex();
        
        // 1. Test Match at the end of a massive payload
        let mut lines: Vec<String> = (0..5000).map(|i| format!("Safe line content #{}", i)).collect();
        lines.push("My credit card is 4111-2222-3333-4444".to_string()); // Match!
        let massive_prompt = lines.join("\n");
        
        use rayon::prelude::*;
        let has_pii = massive_prompt.par_lines().any(|line| regex.is_match(line));
        assert!(has_pii, "Rayon failed to detect PII at the end of a large payload");

        // 2. Test Match at the beginning (Short-circuit verification)
        let mut early_match = vec!["Contact me at boss@nmmglobal.com".to_string()];
        early_match.extend((0..5000).map(|i| format!("Safe line content #{}", i)));
        let early_prompt = early_match.join("\n");
        
        let has_pii_early = early_prompt.par_lines().any(|line| regex.is_match(line));
        assert!(has_pii_early, "Rayon failed to detect PII at the beginning");
    }
}
