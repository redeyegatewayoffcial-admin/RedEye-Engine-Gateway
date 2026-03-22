//! usecases/proxy.rs — Core proxy orchestration logic.
//!
//! Pipeline: compliance redaction → cache check → upstream call → async telemetry.
//! It is intentionally free of Axum types so it can be tested or reused independently.

use serde_json::{json, Value};
use std::sync::{Arc, OnceLock};
use regex::Regex;
use tracing::info;
use axum::response::sse::Event;
use futures::stream::StreamExt;
use tokio::sync::mpsc;
use eventsource_stream::Eventsource;
use tokio_stream::wrappers::ReceiverStream;

use crate::domain::models::{AppState, GatewayError, TraceContext};
use sqlx::Row;
use crate::infrastructure::{cache_client, clickhouse_logger, llm_router};

pub enum ProxyBody {
    Buffered(Vec<u8>),
    SseStream(ReceiverStream<Result<Event, axum::Error>>),
}

fn pii_regex() -> &'static Regex {
    static PII_REGEX: OnceLock<Regex> = OnceLock::new();
    PII_REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:\d[ -]*?){13,16}\b|\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b|\b(?:\+\d{1,2}\s)?\(?\d{3}\)?[\s.-]\d{3}[\s.-]\d{4}\b").unwrap()
    })
}

pub struct ProxyResult {
    pub status: u16,
    pub content_type: String,
    pub body: ProxyBody,
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

    // ── 0. PII Redaction via Compliance Service (fail-closed) ───────────────
    let mut sanitized_body_storage: Option<Value> = None;
    let mut actual_body = body;

    if pii_regex().is_match(raw_prompt) {
        let sanitized = call_compliance_redact(
            &state.http_client,
            &state.compliance_url,
            body,
        ).await?;
        sanitized_body_storage = Some(sanitized);
        actual_body = sanitized_body_storage.as_ref().unwrap();
    }

    // Use the sanitized body for all downstream steps.
    // `raw_prompt` (already extracted by caller) continues to reflect the
    // *original* user message for telemetry / cache-key purposes.
    let body = actual_body;

    // ── 1. Semantic Cache Lookup ────────────────────────────────────────────
    if let Some(cached_content) = cache_client::lookup_cache(
        &state.http_client, &state.cache_url, tenant_id, model_name, raw_prompt
    ).await {
        // ... (cache implementation remains same) ...
        let mock_response = json!({
            "id": "chatcmpl-cached",
            "object": "chat.completion",
            "created": 0,
            "model": model_name,
            "choices": [{"index": 0, "message": {"role": "assistant", "content": cached_content}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        });

        let bytes = serde_json::to_vec(&mock_response).unwrap_or_default();

        let s = state.clone();
        let tid = tenant_id.to_string();
        let model = model_name.to_string();
        let prompt = raw_prompt.to_string();
        let ctx = trace_ctx.clone();
        let latency = start_time.elapsed().as_millis() as u32;

        tokio::spawn(async move {
            fire_async_telemetry(
                &s, &tid, &model, &prompt, &ctx,
                200, latency, 0, true, None,
            ).await;
        });

        return Ok(ProxyResult {
            status: 200,
            content_type: "application/json".to_string(),
            body: ProxyBody::Buffered(bytes),
            cache_hit: true,
        });
    }

    // ── 1.5. Token Bucket Circuit Breaker ──────────────────────────────────
    // Estimate tokens (roughly 1 token per 4 chars of prompt).
    let estimated_tokens = (raw_prompt.len() / 4).max(1);
    
    // Lua script: DECR token bucket, return 1 if allowed, 0 if rate limited.
    // keys[1] = bucket key. args[1] = tokens to consume.
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
            -- Initialize bucket (e.g. 100,000 tokens per day). Set TTL to 86400s
            redis.call('SET', KEYS[1], 100000, 'EX', 86400)
            redis.call('DECRBY', KEYS[1], ARGV[1])
            return 1
        end
    "#;

    let bucket_key = format!("tb:tokens:{}", tenant_id);
    {
        let mut conn = state.redis_conn.clone();
        use redis::AsyncCommands;
        let allowed: Option<i32> = redis::Script::new(lua_script)
            .key(&bucket_key)
            .arg(estimated_tokens)
            .invoke_async(&mut conn)
            .await
            .ok();

        if allowed == Some(0) {
            tracing::warn!(tenant_id = %tenant_id, "Rate limit Token Bucket exceeded");
            return Err(GatewayError::RateLimitExceeded("Token limit exceeded for this billing cycle.".into()));
        }
    }

    // ── 1.6. Adaptive Fallback (Circuit Open Check) ─────────────────────────
    let mut initial_model = model_name.to_string();
    let mut provider = llm_router::LlmProvider::detect(&initial_model);
    
    {
        let mut conn = state.redis_conn.clone();
        use redis::AsyncCommands;
        let cb_key = format!("cb:open:{}:{}", tenant_id, provider.as_str());
        let is_open: Option<String> = conn.get(&cb_key).await.ok();
        
        if is_open.is_some() {
            tracing::warn!(provider = provider.as_str(), "Primary circuit is OPEN. Falling back to Groq.");
            initial_model = "llama3-8b-8192".to_string();
            provider = llm_router::LlmProvider::Groq;
        }
    }

    // ── 2. Determine Provider and API Key ──────────────────────────────────
    let provider_str = provider.as_str();
    
    let redis_key = format!("tenant_provider_key:{}:{}", tenant_id, provider_str);
    
    let mut api_key = None;

    // Check Redis first
    {
        let mut conn = state.redis_conn.clone();
        use redis::AsyncCommands;
        if let Ok(cached_key) = conn.get::<_, String>(&redis_key).await {
            api_key = Some(cached_key);
        }
    }

    if api_key.is_none() {
        // Cache Miss, check DB
        let row = sqlx::query("SELECT encrypted_api_key FROM llm_routes WHERE tenant_id = $1 AND provider = $2")
            .bind(uuid::Uuid::parse_str(tenant_id).unwrap_or_default())
            .bind(provider_str)
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("DB error fetching provider key: {}", e);
                GatewayError::ResponseBuild("Internal server error".into())
            })?;
            
        if let Some(r) = row {
            let encrypted_data: Vec<u8> = r.get(0);
            if let Ok(decrypted) = crate::api::middleware::auth::decrypt_api_key(&encrypted_data) {
                // Set in Redis with 300s TTL (5 minutes)
                {
                    let mut conn = state.redis_conn.clone();
                    use redis::AsyncCommands;
                    let _: Result<(), _> = conn.set_ex(&redis_key, &decrypted, 300).await;
                }
                api_key = Some(decrypted);
            }
        }
    }

    let resolved_key = match api_key {
        Some(k) => k,
        None => {
            tracing::warn!(tenant_id = %tenant_id, provider = provider_str, "Provider API key not configured");
            return Err(GatewayError::ResponseBuild("Provider API key not configured".into()));
        }
    };

    // ── 3. Forward to Universal LLM Router (With Fallback) ──────────────────
    let mut upstream_response_res = llm_router::route_chat_completion(
        &state.http_client, &resolved_key, body, accept_header,
    ).await;

    // Evaluate response for Adaptive Circuit Breaker failures (5xx or timeout)
    let mut is_failure = false;
    match &upstream_response_res {
        Ok(resp) if resp.status().is_server_error() => is_failure = true,
        Err(_) => is_failure = true,
        _ => {}
    }

    if is_failure {
        tracing::error!(provider = provider_str, "Primary provider failed. Tracking error for circuit breaker...");
        {
            let mut conn = state.redis_conn.clone();
            use redis::AsyncCommands;
            let err_key = format!("cb:errors:{}:{}", tenant_id, provider_str);
            let count: i32 = conn.incr(&err_key, 1).await.unwrap_or(0);
            let _: Result<(), _> = conn.expire(&err_key, 10).await;

            if count >= 2 {
                tracing::error!("Circuit breaker triggered! Opening circuit for {}", provider_str);
                let open_key = format!("cb:open:{}:{}", tenant_id, provider_str);
                let _: Result<(), _> = conn.set_ex(&open_key, "1", 60).await;
            }
        }
        
        // Attempt immediate seamless retry to Fallback (Groq) if primary failed
        if provider != llm_router::LlmProvider::Groq {
            tracing::info!("Attempting seamless immediate fallback to Groq...");
            initial_model = "llama3-8b-8192".to_string();
            
            // Re-fetch key for Groq
            let mut groq_key = None;
            let row = sqlx::query("SELECT encrypted_api_key FROM llm_routes WHERE tenant_id = $1 AND provider = 'groq'")
                .bind(uuid::Uuid::parse_str(tenant_id).unwrap_or_default())
                .fetch_optional(&state.db_pool)
                .await.ok().flatten();
                
            if let Some(r) = row {
                let encrypted_data: Vec<u8> = r.try_get(0).unwrap_or_default();
                if let Ok(dec) = crate::api::middleware::auth::decrypt_api_key(&encrypted_data) {
                    groq_key = Some(dec);
                }
            }

            if let Some(gk) = groq_key {
                // Must clone body to swap the model field for the fallback request
                let mut fallback_body = body.clone();
                fallback_body["model"] = json!("llama3-8b-8192");

                upstream_response_res = llm_router::route_chat_completion(
                    &state.http_client, &gk, &fallback_body, accept_header,
                ).await;
            } else {
                return Err(GatewayError::ResponseBuild("Fallback provider key not found".into()));
            }
        }
    }

    let upstream_response = upstream_response_res?;

    let upstream_status = upstream_response.status().as_u16();
    let content_type = upstream_response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    info!(status = upstream_status, provider = provider_str, "Received response from upstream LLM provider");

    let is_streaming = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if is_streaming {
        let mut event_stream = upstream_response.bytes_stream().eventsource();
        let compliance_url = state.compliance_url.clone();
        let http_client = state.http_client.clone();
        
        let (tx, rx) = mpsc::channel(100);

        // Clones for the spawned task
        let state_c = state.clone();
        let tenant_id_c = tenant_id.to_string();
        let model_name_c = model_name.to_string();
        let raw_prompt_c = raw_prompt.to_string();
        let trace_ctx_c = trace_ctx.clone();
        let start_time_c = start_time;
        let ups_status = upstream_status;

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut full_response = String::new();
            
            while let Some(Ok(event)) = event_stream.next().await {
                if event.data == "[DONE]" {
                    if !buffer.is_empty() {
                        if let Ok(redacted) = call_compliance_redact_chunk(&http_client, &compliance_url, &buffer).await {
                            let json_str = format!(
                                "{{\"choices\":[{{\"delta\":{{\"content\":{}}}}}]}}",
                                serde_json::to_string(&redacted).unwrap_or_else(|_| "\"\"".to_string())
                            );
                            let _ = tx.send(Ok(Event::default().data(json_str))).await;
                        }
                        buffer.clear();
                    }
                    let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
                    break;
                }

                if let Ok(json) = serde_json::from_str::<Value>(&event.data) {
                    if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                        buffer.push_str(content);
                        full_response.push_str(content);
                        
                        // Heuristic: Flush buffer if it's > 30 chars or ends with punctuation/newline
                        let should_flush = buffer.len() > 30 || content.contains(|c| ['.', '!', '?', '\n'].contains(&c));
                        
                        if should_flush {
                            if let Ok(redacted) = call_compliance_redact_chunk(&http_client, &compliance_url, &buffer).await {
                                let mut new_json = json.clone();
                                new_json["choices"][0]["delta"]["content"] = Value::String(redacted);
                                let _ = tx.send(Ok(Event::default().data(new_json.to_string()))).await;
                                // We don't push redacted to full_response here because we already pushed raw content above.
                                // If the user wants the CACHE to have the redacted version, we should rethink this.
                                // However, follow the user's specific instruction to accumulate from the delta content.
                            } else {
                                // Fallback (e.g. timeout) - send original
                                let mut new_json = json.clone();
                                new_json["choices"][0]["delta"]["content"] = Value::String(buffer.clone());
                                let _ = tx.send(Ok(Event::default().data(new_json.to_string()))).await;
                            }
                            buffer.clear();
                        }
                        continue;
                    }
                }
                
                // Unrecognized data pass-through
                let _ = tx.send(Ok(Event::default().data(event.data))).await;
            }

            // Stream complete - trigger telemetry with the full response content
            let latency_ms = start_time_c.elapsed().as_millis() as u32;
            
            let mock_response = json!({
                "id": "chatcmpl-streamed",
                "object": "chat.completion",
                "created": 0,
                "model": model_name_c,
                "choices": [{"index": 0, "message": {"role": "assistant", "content": full_response}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
            });

            let mock_bytes = serde_json::to_vec(&mock_response).unwrap_or_default();

            fire_async_telemetry(
                &state_c,
                &tenant_id_c,
                &model_name_c,
                &raw_prompt_c,
                &trace_ctx_c,
                ups_status,
                latency_ms,
                0, // Token counting for streams is complex; using 0 as placeholder or implement simple heuristic
                false,
                Some(mock_bytes),
            ).await;
        });

        return Ok(ProxyResult {
            status: upstream_status,
            content_type,
            body: ProxyBody::SseStream(ReceiverStream::new(rx)),
            cache_hit: false,
        });
    }

    // Buffer the response for telemetry + cache storage (non-streaming)
    let body_bytes = upstream_response.bytes().await.unwrap_or_default().to_vec();
    let latency_ms = start_time.elapsed().as_millis() as u32;

    // Extract token usage from OpenAI response
    let tokens = serde_json::from_slice::<Value>(&body_bytes)
        .ok()
        .and_then(|v| v["usage"]["total_tokens"].as_u64())
        .unwrap_or(0) as u32;

    let s = state.clone();
    let tid = tenant_id.to_string();
    let model = model_name.to_string();
    let prompt = raw_prompt.to_string();
    let ctx = trace_ctx.clone();
    let bytes_c = Some(body_bytes.clone());

    tokio::spawn(async move {
        fire_async_telemetry(
            &s, &tid, &model, &prompt, &ctx,
            upstream_status, latency_ms, tokens, false,
            bytes_c,
        ).await;
    });

    Ok(ProxyResult {
        status: upstream_status,
        content_type,
        body: ProxyBody::Buffered(body_bytes),
        cache_hit: false,
    })
}

/// Spawns a detached background task for ClickHouse logging, tracer ingestion, and cache storage.
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
    let s = state.clone();
    let tid = tenant_id.to_string();
    let model = model_name.to_string();
    let prompt = raw_prompt.to_string();
    let ctx = trace_ctx.clone();

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
            cache_client::store_in_cache(&s.http_client, &s.cache_url, &tid, &model, &prompt, &response_content).await;
        }
}

// ── Compliance Helper ────────────────────────────────────────────────────────

/// Sends `payload` to the compliance redaction endpoint and returns the
/// sanitized body. Implements a strict **fail-closed** policy: any network
/// error, non-2xx status, or malformed response causes the proxy to abort
/// with `GatewayError::ComplianceFailure`, preventing raw PII from reaching
/// the upstream LLM provider.
async fn call_compliance_redact(
    http_client: &reqwest::Client,
    compliance_url: &str,
    payload: &Value,
) -> Result<Value, GatewayError> {
    let endpoint = format!("{}/api/v1/compliance/redact", compliance_url.trim_end_matches('/'));

    tracing::debug!(endpoint = %endpoint, "Calling compliance redaction service");

    let resp = http_client
        .post(&endpoint)
        .json(payload)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Compliance service unreachable — blocking request (fail-closed)");
            GatewayError::ComplianceFailure(format!("Compliance service unreachable: {}", e))
        })?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        tracing::error!(status = status, "Compliance service returned non-2xx — blocking request");
        return Err(GatewayError::ComplianceFailure(
            format!("Compliance service returned HTTP {}", status)
        ));
    }

    let compliance_json: Value = resp.json().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to parse compliance response body");
        GatewayError::ComplianceFailure(format!("Malformed compliance response: {}", e))
    })?;

    match compliance_json.get("sanitized_payload").cloned() {
        Some(sanitized) => {
            tracing::info!("PII redaction complete — forwarding sanitized payload to upstream LLM");
            Ok(sanitized)
        }
        None => {
            tracing::error!("Compliance response missing `sanitized_payload` field — blocking request");
            Err(GatewayError::ComplianceFailure(
                "Compliance response did not contain `sanitized_payload`".into()
            ))
        }
    }
}

/// Helper to redact a single string chunk for SSE streaming
async fn call_compliance_redact_chunk(
    http_client: &reqwest::Client,
    compliance_url: &str,
    chunk: &str,
) -> Result<String, GatewayError> {
    // We wrap it in a dummy JSON object, since PiiEngine traverses JSON.
    let payload = json!({ "chunk": chunk });
    let sanitized = call_compliance_redact(http_client, compliance_url, &payload).await?;
    
    if let Some(redacted_str) = sanitized["chunk"].as_str() {
        Ok(redacted_str.to_string())
    } else {
        Ok(chunk.to_string())
    }
}
