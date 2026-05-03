//! api/handlers.rs — Thin Axum handlers that extract, delegate to use cases, and respond.

use serde_json::{json, Value};
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Extension, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use tracing::{error, info, instrument};

use crate::domain::models::{AppState, TraceContext};
use crate::error::GatewayError;
use crate::usecases::proxy;

/// GET /health
pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "redeye_gateway",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(serde::Deserialize)]
struct ExtractModel<'a> {
    #[serde(borrow)]
    model: Option<std::borrow::Cow<'a, str>>,
}

/// POST /v1/chat/completions
#[instrument(skip(state, body_bytes))]
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Extension(trace_ctx): Extension<TraceContext>,
    headers: HeaderMap,
    body_bytes: axum::body::Bytes,
) -> Result<Response, GatewayError> {
    info!("Received chat completion request");

    // Extract metadata using zero-copy extraction
    let extracted: ExtractModel = serde_json::from_slice(&body_bytes).unwrap_or(ExtractModel { model: None });
    let model_name = extracted.model.as_deref().unwrap_or("gpt-4o").to_string();

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let is_agentic = detect_agentic_payload(&body_bytes, content_type);

    let session_id = headers
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let idempotency_key = headers
        .get("x-idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let api_key = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("")
        .to_string();

    let tenant_id = headers
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string();
    let raw_prompt = String::from_utf8_lossy(&body_bytes).to_string();
    let accept = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    let mut loop_count: Option<u64> = None;

    if is_agentic {
        let resolved_session_id = crate::usecases::agentic_tracker::resolve_session_id(
            session_id.as_deref(),
            idempotency_key.as_deref(),
            &tenant_id,
            &api_key,
            &body_bytes,
        );

        let start_detection = tokio::time::Instant::now();
        let budget_res = crate::usecases::behavior_guard::enforce_agentic_loop_budget(&state, &resolved_session_id).await;
        let detection_latency = start_detection.elapsed().as_secs_f64();

        let (passed, hit, count_val) = match &budget_res {
            Ok(c) => (true, 0, *c),
            Err(_) => (false, 1, 0),
        };

        // Offload telemetry non-blocking
        let tx = state.telemetry_tx.clone();
        let tid = tenant_id.clone();
        tokio::spawn(async move {
            let _ = tx.send(serde_json::json!({
                "type": "agentic_loop_detection",
                "session_id": resolved_session_id,
                "tenant_id": tid,
                "agentic_requests_total": 1,
                "loop_limit_hits": hit,
                "detection_latency_seconds": detection_latency,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })).await;
        });

        loop_count = Some(count_val);

        if !passed {
            return Err(budget_res.unwrap_err());
        }
    }

    // Extract routing strategy from header
    let strategy = crate::infrastructure::routing_strategy::RoutingStrategy::from_header(
        headers
            .get("x-redeye-routing-strategy")
            .and_then(|v| v.to_str().ok()),
    );

    // ── Tunnel 3 Phase 1: Speculative MCP Pre-Fetching (Hardened) ────────────
    // FSM-based scan (zero-allocation, chunk-boundary safe).
    // Backpressure: try_acquire_owned() on a 20-slot semaphore.  If all slots
    // are busy the warm-up is silently dropped — this is best-effort advisory.
    // Stateful handoff: warm_connection writes its result into registry.warmed
    // so Phase 4 fan-out can skip redundant TCP handshakes.
    if !state.mcp_registry.is_empty() {
        if let Some(tool_name) = state.mcp_registry.find_tool_hint(&body_bytes) {
            if let Some(sse_url) = state.mcp_registry.get_url(&tool_name) {
                match Arc::clone(&state.mcp_registry.prefetch_sem).try_acquire_owned() {
                    Ok(permit) => {
                        let registry = Arc::clone(&state.mcp_registry);
                        let warm_client = state.http_client.clone();
                        tokio::spawn(async move {
                            crate::infrastructure::mcp_registry::McpConnectionRegistry::warm_connection(
                                registry,
                                warm_client,
                                tool_name,
                                sse_url,
                                permit,
                            )
                            .await;
                        });
                    }
                    Err(_) => {
                        tracing::debug!(
                            "Tunnel 3 Phase 1 — prefetch semaphore full (20 in-flight); \
                             dropping warm-up (best-effort)"
                        );
                    }
                }
            }
        }
    }

    // ── Tunnel 3 Phase 2: Lazy Schema Loading ──────────────────────────────
    // Create a per-request bumpalo arena on the stack.  The arena holds
    // intermediate byte allocations during JSON serialisation; it is freed
    // in O(1) when this block scope ends.
    //
    // `body_bytes` is shadowed: if schemas were stripped the binding now
    // points to the modified bytes; otherwise it points to the original
    // allocation (zero-copy pass-through).
    let body_bytes: axum::body::Bytes = if !state.tool_registry.is_empty() {
        let arena = bumpalo::Bump::new();
        match state.tool_registry.inject_lazy_summaries(&body_bytes, &arena) {
            Some(modified) => {
                tracing::debug!(
                    original_len = body_bytes.len(),
                    modified_len = modified.len(),
                    "Tunnel 3 Phase 2 — using schema-stripped body for upstream LLM call"
                );
                axum::body::Bytes::from(modified)
            }
            // No registered tools found or no schemas present — forward unchanged.
            None => body_bytes,
        }
    } else {
        body_bytes
    };

    // Delegate to use case
    let result = proxy::execute_proxy(
        &state,
        &body_bytes,
        &tenant_id,
        &model_name,
        &raw_prompt,
        accept,
        &trace_ctx,
        strategy,
    )
    .await?;

    // Build Axum response
    let cache_header = if result.cache_hit { "HIT" } else { "MISS" };

    match result.body {
        crate::usecases::proxy::ProxyBody::Buffered(body_bytes) => {
            let mut response = Response::builder()
                .status(result.status)
                .header("content-type", &result.content_type)
                .header("X-Cache", cache_header)
                .body(Body::from(body_bytes))
                .map_err(|e| {
                    error!(error = %e, "Failed to construct proxy response");
                    GatewayError::ResponseBuild(e.to_string())
                })?;

            if let Some(c) = loop_count {
                if let Ok(v) = axum::http::HeaderValue::from_str(&c.to_string()) {
                    response.headers_mut().insert("X-RedEye-Loop-Count", v);
                }
            }

            Ok(response)
        }
        crate::usecases::proxy::ProxyBody::Stream(body) => {
            let mut response = Response::builder()
                .status(result.status)
                .header("content-type", &result.content_type)
                .body(body)
                .map_err(|e| {
                    error!(error = %e, "Failed to construct proxy streaming response");
                    GatewayError::ResponseBuild(e.to_string())
                })?;
                
            if let Ok(value) = axum::http::HeaderValue::from_str(cache_header) {
                response.headers_mut().insert("X-Cache", value);
            }

            if let Some(c) = loop_count {
                if let Ok(v) = axum::http::HeaderValue::from_str(&c.to_string()) {
                    response.headers_mut().insert("X-RedEye-Loop-Count", v);
                }
            }

            Ok(response)
        }
    }
}

#[derive(serde::Serialize)]
pub struct HotSwapMetric {
    pub time: String,
    pub openai_success: u64,
    pub openai_error: u64,
    pub anthropic_fallback: u64,
}

/// GET /v1/admin/metrics/hot-swaps
/// Returns real-time hot-swap counts for the authenticated tenant.
#[instrument(skip(state, claims))]
pub async fn get_hot_swaps(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<HotSwapMetric>>, GatewayError> {
    info!(tenant_id = %claims.tenant_id, "Fetching hot-swaps from ClickHouse");

    let parsed_tenant_id = uuid::Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| GatewayError::ResponseBuild("Invalid tenant ID format".to_string()))?;

    let query = format!(
        "SELECT \
            formatDateTime(toStartOfMinute(created_at), '%H:%M') as time, \
            countIf(requested_provider = 'openai' AND is_hot_swapped = 0 AND status = 200) as openai_success, \
            countIf(requested_provider = 'openai' AND is_hot_swapped = 0 AND status != 200) as openai_error, \
            countIf(is_hot_swapped = 1) as anthropic_fallback \
         FROM RedEye_telemetry.request_logs \
         WHERE tenant_id = '{}' AND created_at >= NOW() - INTERVAL 1 HOUR \
         GROUP BY toStartOfMinute(created_at) \
         ORDER BY toStartOfMinute(created_at) ASC \
         FORMAT JSON",
        parsed_tenant_id
    );

    let resp = state
        .http_client
        .post(&state.clickhouse_url)
        .body(query)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "ClickHouse hot-swaps query failed (network)");
            GatewayError::Proxy(e)
        })?;

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        error!(error = %err_body, "ClickHouse hot-swaps query returned non-2xx");
        return Err(GatewayError::ResponseBuild(
            "ClickHouse hot-swaps query failed".to_string(),
        ));
    }

    let ch_json: Value = resp.json().await.map_err(|e| {
        error!(error = %e, "Failed to deserialise ClickHouse response");
        GatewayError::ResponseBuild(e.to_string())
    })?;

    let mut metrics = Vec::new();
    let rows = ch_json["data"].as_array().cloned().unwrap_or_default();

    for row in rows {
        let time = row["time"].as_str().unwrap_or("00:00").to_string();

        let openai_success: u64 = match &row["openai_success"] {
            Value::String(s) => s.parse().unwrap_or(0),
            Value::Number(n) => n.as_u64().unwrap_or(0),
            _ => 0,
        };
        let openai_error: u64 = match &row["openai_error"] {
            Value::String(s) => s.parse().unwrap_or(0),
            Value::Number(n) => n.as_u64().unwrap_or(0),
            _ => 0,
        };
        let anthropic_fallback: u64 = match &row["anthropic_fallback"] {
            Value::String(s) => s.parse().unwrap_or(0),
            Value::Number(n) => n.as_u64().unwrap_or(0),
            _ => 0,
        };

        metrics.push(HotSwapMetric {
            time,
            openai_success,
            openai_error,
            anthropic_fallback,
        });
    }

    Ok(Json(metrics))
}

use crate::api::middleware::auth::Claims;

/// GET /v1/admin/metrics
#[instrument(skip(state, claims))]
pub async fn admin_metrics(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, GatewayError> {
    info!(tenant_id = %claims.tenant_id, "Fetching live metrics from ClickHouse");

    let parsed_tenant_id = uuid::Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| GatewayError::ResponseBuild("Invalid tenant ID format".to_string()))?;

    // 1. Summary Stats
    let stats_query = format!(
        "
        SELECT 
            count() as total_requests,
            avg(latency_ms) as avg_latency_ms,
            sum(tokens) as total_tokens,
            countIf(status = 429) as rate_limited_requests
        FROM RedEye_telemetry.request_logs
        WHERE tenant_id = '{}'
        FORMAT JSON
    ",
        parsed_tenant_id
    );

    // 2. Traffic Series (Last 24 hours, hourly buckets)
    let traffic_query = format!(
        "
        SELECT 
            formatDateTime(toStartOfHour(created_at), '%Y-%m-%dT%H:%M:%S') as timestamp,
            count() as requests
        FROM RedEye_telemetry.request_logs
        WHERE tenant_id = '{}' AND created_at > now() - INTERVAL 24 HOUR
        GROUP BY timestamp
        ORDER BY timestamp
        FORMAT JSON
    ",
        parsed_tenant_id
    );

    // 3. Model Distribution
    let model_query = format!(
        "
        SELECT 
            model as name,
            count() as value
        FROM RedEye_telemetry.request_logs
        WHERE tenant_id = '{}'
        GROUP BY name
        FORMAT JSON
    ",
        parsed_tenant_id
    );

    // 4. Latency Buckets
    let latency_query = format!(
        "
        SELECT 
            case 
                when latency_ms < 100 then '0-100ms'
                when latency_ms < 500 then '100-500ms'
                when latency_ms < 1000 then '500-1s'
                else '1s+'
            end as bucket,
            count() as count
        FROM RedEye_telemetry.request_logs
        WHERE tenant_id = '{}'
        GROUP BY bucket
        ORDER BY count DESC
        FORMAT JSON
    ",
        parsed_tenant_id
    );

    // Execute queries (simplified sequential for reliability, could use join_all)
    let stats_resp = state
        .http_client
        .post(&state.clickhouse_url)
        .body(stats_query)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e);
            GatewayError::Proxy(e)
        })?;
    let traffic_resp = state
        .http_client
        .post(&state.clickhouse_url)
        .body(traffic_query)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e);
            GatewayError::Proxy(e)
        })?;
    let model_resp = state
        .http_client
        .post(&state.clickhouse_url)
        .body(model_query)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e);
            GatewayError::Proxy(e)
        })?;
    let latency_resp = state
        .http_client
        .post(&state.clickhouse_url)
        .body(latency_query)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e);
            GatewayError::Proxy(e)
        })?;

    let stats_json: Value = stats_resp.json().await.unwrap_or(json!({"data": []}));
    let traffic_json: Value = traffic_resp.json().await.unwrap_or(json!({"data": []}));
    let model_json: Value = model_resp.json().await.unwrap_or(json!({"data": []}));
    let latency_json: Value = latency_resp.json().await.unwrap_or(json!({"data": []}));

    let summary = stats_json["data"].as_array().and_then(|a| a.first()).cloned()
        .unwrap_or_else(|| json!({"total_requests": "0", "avg_latency_ms": 0.0, "total_tokens": "0", "rate_limited_requests": "0"}));

    let mut result = summary;
    result["traffic_series"] = traffic_json["data"].clone();
    result["model_distribution"] = model_json["data"].clone();
    result["latency_buckets"] = latency_json["data"].clone();

    Ok(Json(result))
}

/// GET /v1/admin/metrics/usage
/// Returns total token consumption and estimated cost for the authenticated tenant.
///
/// # Complexity
/// - Time:  O(n) in ClickHouse (full partition scan bounded by tenant_id), O(1) in Rust.
/// - Space: O(1) — single aggregation row is parsed; no allocations proportional to row count.
#[instrument(skip(state, claims))]
pub async fn get_usage_metrics(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, GatewayError> {
    info!(tenant_id = %claims.tenant_id, "Fetching token usage metrics from ClickHouse");

    let parsed_tenant_id = uuid::Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| GatewayError::ResponseBuild("Invalid tenant ID format".to_string()))?;

    // Parameterised via format! — tenant_id comes from a trusted, validated JWT claim,
    // so there is no SQL-injection risk from external user input.
    let query = format!(
        "SELECT sum(tokens) as total_tokens \
         FROM RedEye_telemetry.request_logs \
         WHERE tenant_id = '{}' \
         FORMAT JSON",
        parsed_tenant_id
    );

    let resp = state
        .http_client
        .post(&state.clickhouse_url)
        .body(query)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "ClickHouse usage query failed (network)");
            GatewayError::Proxy(e)
        })?;

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        error!(error = %err_body, "ClickHouse usage query returned non-2xx");
        return Err(GatewayError::ResponseBuild(
            "ClickHouse usage query failed".to_string(),
        ));
    }

    // Parse the ClickHouse JSON envelope: { "data": [{"total_tokens": "N"}], ... }
    // ClickHouse returns numeric aggregates as strings in FORMAT JSON.
    let ch_json: Value = resp.json().await.map_err(|e| {
        error!(error = %e, "Failed to deserialise ClickHouse usage response");
        GatewayError::ResponseBuild(e.to_string())
    })?;

    // Gracefully handle empty tables: fall back to "0".
    let total_tokens: u64 = ch_json["data"]
        .as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row["total_tokens"].as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Flat rate: $0.002 per 1,000 tokens.  Pure arithmetic — no I/O.
    const COST_PER_THOUSAND: f64 = 0.002;
    let estimated_cost = (total_tokens as f64) / 1_000.0 * COST_PER_THOUSAND;

    info!(
        tenant_id = %claims.tenant_id,
        total_tokens,
        estimated_cost,
        "Usage metrics computed"
    );

    Ok(Json(json!({
        "total_tokens": total_tokens,
        "estimated_cost": (estimated_cost * 10_000.0).round() / 10_000.0,
    })))
}

/// GET /v1/admin/billing/breakdown
/// Returns daily cost aggregated by model.
#[instrument(skip(state, claims))]
pub async fn get_billing_breakdown(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, GatewayError> {
    info!(tenant_id = %claims.tenant_id, "Fetching billing breakdown from ClickHouse");

    let parsed_tenant_id = uuid::Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| GatewayError::ResponseBuild("Invalid tenant ID format".to_string()))?;

    // We group by day and model. tenant_id is safe as it's from the JWT claim.
    let query = format!(
        "SELECT \
            formatDateTime(toStartOfDay(created_at), '%Y-%m-%d') as date, \
            model, \
            sum(tokens) as total_tokens \
         FROM RedEye_telemetry.request_logs \
         WHERE tenant_id = '{}' \
         GROUP BY date, model \
         ORDER BY date DESC, model ASC \
         FORMAT JSON",
        parsed_tenant_id
    );

    let resp = state
        .http_client
        .post(&state.clickhouse_url)
        .body(query)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "ClickHouse billing query failed (network)");
            GatewayError::Proxy(e)
        })?;

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        error!(error = %err_body, "ClickHouse billing query returned non-2xx");
        return Err(GatewayError::ResponseBuild(
            "ClickHouse billing query failed".to_string(),
        ));
    }

    let ch_json: Value = resp.json().await.map_err(|e| {
        error!(error = %e, "Failed to deserialise ClickHouse response");
        GatewayError::ResponseBuild(e.to_string())
    })?;

    // Transform ClickHouse rows (where total_tokens is often a string) into typed entries + estimated_cost
    let rows = ch_json["data"].as_array().cloned().unwrap_or_default();

    let mut breakdown = Vec::new();
    const COST_PER_THOUSAND: f64 = 0.002;

    for row in rows {
        let date = row["date"].as_str().unwrap_or("1970-01-01").to_string();
        let model = row["model"].as_str().unwrap_or("unknown").to_string();

        let tokens: f64 = match &row["total_tokens"] {
            Value::String(s) => s.parse().unwrap_or(0.0),
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            _ => 0.0,
        };

        let est_cost = (tokens / 1_000.0) * COST_PER_THOUSAND;
        let rounded_cost = (est_cost * 10_000.0).round() / 10_000.0; // 4 d.p.

        breakdown.push(json!({
            "date": date,
            "model": model,
            "total_tokens": tokens as u64,
            "estimated_cost": rounded_cost
        }));
    }

    Ok(Json(json!(breakdown)))
}

/// GET /v1/admin/traces
#[instrument(skip(state, claims))]
pub async fn get_traces(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, GatewayError> {
    info!(tenant_id = %claims.tenant_id, "Fetching recent traces from ClickHouse");

    let parsed_tenant_id = uuid::Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| GatewayError::ResponseBuild("Invalid tenant ID format".to_string()))?;

    let query = format!(
        "
        SELECT 
            toString(id) as traceId,
            tenant_id as tenantId,
            model,
            tokens,
            concat(toString(latency_ms), ' ms') as latency,
            if(status = 200, 'Allowed', 'Blocked') as policy
        FROM RedEye_telemetry.request_logs
        WHERE tenant_id = '{}'
        ORDER BY created_at DESC
        LIMIT 50
        FORMAT JSON
    ",
        parsed_tenant_id
    );

    let response = state
        .http_client
        .post(&state.clickhouse_url)
        .body(query)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e);
            GatewayError::Proxy(e)
        })?;

    if !response.status().is_success() {
        let err = response.text().await.unwrap_or_default();
        error!(error = %err, "ClickHouse traces query failed");
        return Err(GatewayError::ResponseBuild(
            "Traces query failed".to_string(),
        ));
    }

    let clickhouse_json: Value = response.json().await.map_err(|e| {
        error!(error = %e);
        GatewayError::ResponseBuild(e.to_string())
    })?;

    let data = clickhouse_json["data"].clone();
    Ok(Json(data))
}

/// GET /v1/admin/security/alerts
/// Returns real security alert data from ClickHouse for the Security Command Center.
/// Queries `agent_traces` where `status = 429` and `model IN ('__loop_blocked', '__burn_rate_blocked')`.
#[instrument(skip(state, claims))]
pub async fn security_alerts(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, GatewayError> {
    info!(tenant_id = %claims.tenant_id, "Fetching security alerts from ClickHouse");

    let parsed_tenant_id = uuid::Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| GatewayError::ResponseBuild("Invalid tenant ID format".to_string()))?;
    let tid = &parsed_tenant_id;

    // ── Query 1: Summary Stats ─────────────────────────────────────────────
    let stats_query = format!(
        "SELECT \
            countIf(model = '__loop_blocked') as total_loops, \
            countIf(model = '__burn_rate_blocked') as total_burns \
         FROM RedEye_telemetry.agent_traces \
         WHERE tenant_id = '{}' AND status = 429 \
           AND model IN ('__loop_blocked', '__burn_rate_blocked') \
         FORMAT JSON",
        tid
    );

    // ── Query 2: Daily Blocks (last 7 days) ────────────────────────────────
    let daily_query = format!(
        "SELECT \
            formatDateTime(toStartOfDay(created_at), '%b %d') as date, \
            countIf(model = '__loop_blocked') as loops, \
            countIf(model = '__burn_rate_blocked') as burn_rate \
         FROM RedEye_telemetry.agent_traces \
         WHERE tenant_id = '{}' AND status = 429 \
           AND model IN ('__loop_blocked', '__burn_rate_blocked') \
           AND created_at >= now() - INTERVAL 7 DAY \
         GROUP BY toStartOfDay(created_at) \
         ORDER BY toStartOfDay(created_at) \
         FORMAT JSON",
        tid
    );

    // ── Query 3: Recent Alerts (last 10) ───────────────────────────────────
    let recent_query = format!(
        "SELECT \
            formatDateTime(created_at, '%Y-%m-%dT%H:%i:%sZ') as timestamp, \
            toString(session_id) as session_id, \
            model \
         FROM RedEye_telemetry.agent_traces \
         WHERE tenant_id = '{}' AND status = 429 \
           AND model IN ('__loop_blocked', '__burn_rate_blocked') \
         ORDER BY created_at DESC \
         LIMIT 10 \
         FORMAT JSON",
        tid
    );

    // ── Execute all three queries ──────────────────────────────────────────
    let (stats_res, daily_res, recent_res) = tokio::join!(
        state
            .http_client
            .post(&state.clickhouse_url)
            .body(stats_query)
            .send(),
        state
            .http_client
            .post(&state.clickhouse_url)
            .body(daily_query)
            .send(),
        state
            .http_client
            .post(&state.clickhouse_url)
            .body(recent_query)
            .send(),
    );

    // ── Parse stats ────────────────────────────────────────────────────────
    let stats_json: Value = match stats_res {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or(json!({"data": []})),
        _ => json!({"data": []}),
    };
    let stats_row = stats_json["data"]
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .unwrap_or(json!({"total_loops": "0", "total_burns": "0"}));

    let total_loops: u64 = stats_row["total_loops"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| stats_row["total_loops"].as_u64())
        .unwrap_or(0);
    let total_burns: u64 = stats_row["total_burns"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| stats_row["total_burns"].as_u64())
        .unwrap_or(0);

    // Estimated savings: each blocked request would have consumed ~250 tokens.
    // Cost model: $0.002 per 1K tokens → $0.0005 per request.
    let estimated_savings = (total_loops + total_burns) as f64 * 250.0 / 1000.0 * 0.002;
    let estimated_savings = (estimated_savings * 100.0).round() / 100.0;

    // ── Parse daily blocks ─────────────────────────────────────────────────
    let daily_json: Value = match daily_res {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or(json!({"data": []})),
        _ => json!({"data": []}),
    };
    let daily_blocks = daily_json["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|mut row| {
            // ClickHouse returns counts as strings in FORMAT JSON — normalise them.
            if let Some(s) = row["loops"].as_str() {
                row["loops"] = json!(s.parse::<u64>().unwrap_or(0));
            }
            if let Some(s) = row["burn_rate"].as_str() {
                row["burn_rate"] = json!(s.parse::<u64>().unwrap_or(0));
            }
            row
        })
        .collect::<Vec<_>>();

    // ── Parse recent alerts ────────────────────────────────────────────────
    let recent_json: Value = match recent_res {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or(json!({"data": []})),
        _ => json!({"data": []}),
    };
    let recent_alerts: Vec<Value> = recent_json["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            let model = row["model"].as_str().unwrap_or("");
            let (reason, severity) = match model {
                "__loop_blocked" => ("Loop Detected", "High"),
                "__burn_rate_blocked" => ("Burn Rate Exceeded", "Critical"),
                _ => ("Unknown Block", "Medium"),
            };
            json!({
                "timestamp":  row["timestamp"],
                "session_id": row["session_id"],
                "reason":     reason,
                "severity":   severity,
            })
        })
        .collect();

    Ok(Json(json!({
        "total_blocked_loops":    total_loops,
        "total_burn_rate_blocks": total_burns,
        "estimated_savings_usd":  estimated_savings,
        "daily_blocks":           daily_blocks,
        "recent_alerts":          recent_alerts,
    })))
}

/// GET /v1/admin/metrics/cache
/// Returns hit_ratio, miss_ratio, and total_lookups for the authenticated tenant.
///
/// # Complexity
/// - Time:  O(n) in ClickHouse (full partition scan bounded by tenant_id), O(1) in Rust.
/// - Space: O(1) — single aggregation row is parsed.
#[instrument(skip(state, claims))]
pub async fn get_cache_metrics(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, GatewayError> {
    info!(tenant_id = %claims.tenant_id, "Fetching cache metrics from ClickHouse");

    let parsed_tenant_id = uuid::Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| GatewayError::ResponseBuild("Invalid tenant ID format".to_string()))?;

    let query = format!(
        "SELECT count() as redacted_count \
         FROM RedEye_telemetry.compliance_audit_log \
         WHERE tenant_id = '{}' AND flagged = true AND created_at >= now() - INTERVAL 24 HOUR \
         FORMAT JSON",
        parsed_tenant_id
    );

    let resp = state
        .http_client
        .post(&state.clickhouse_url)
        .body(query)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "ClickHouse cache metrics query failed (network)");
            GatewayError::Proxy(e)
        })?;

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        error!(error = %err_body, "ClickHouse cache metrics query returned non-2xx");
        return Ok(Json(json!({
            "hit_ratio": 0.0,
            "miss_ratio": 0.0,
            "total_lookups": 0,
        })));
    }

    let ch_json: Value = resp.json().await.map_err(|e| {
        error!(error = %e, "Failed to deserialise ClickHouse cache metrics response");
        GatewayError::ResponseBuild(e.to_string())
    })?;

    let row = ch_json["data"]
        .as_array()
        .and_then(|rows| rows.first())
        .cloned()
        .unwrap_or(json!({"total_lookups": "0", "hits": "0", "misses": "0"}));

    let total_lookups: u64 = match &row["total_lookups"] {
        Value::String(s) => s.parse().unwrap_or(0),
        Value::Number(n) => n.as_u64().unwrap_or(0),
        _ => 0,
    };
    let hits: u64 = match &row["hits"] {
        Value::String(s) => s.parse().unwrap_or(0),
        Value::Number(n) => n.as_u64().unwrap_or(0),
        _ => 0,
    };
    let misses: u64 = match &row["misses"] {
        Value::String(s) => s.parse().unwrap_or(0),
        Value::Number(n) => n.as_u64().unwrap_or(0),
        _ => 0,
    };

    let hit_ratio = if total_lookups > 0 {
        (hits as f64) / (total_lookups as f64)
    } else {
        0.0
    };
    let miss_ratio = if total_lookups > 0 {
        (misses as f64) / (total_lookups as f64)
    } else {
        0.0
    };

    Ok(Json(json!({
        "hit_ratio": hit_ratio,
        "miss_ratio": miss_ratio,
        "total_lookups": total_lookups,
    })))
}

#[inline]
pub fn detect_agentic_payload(body_bytes: &[u8], content_type: &str) -> bool {
    // Fast-path size check & Content-Type check (max 5MB for inspection)
    if body_bytes.len() > 5 * 1024 * 1024 || !content_type.starts_with("application/json") {
        return false;
    }
    
    let mut bytes_vec = body_bytes.to_vec();
    let is_agentic = match simd_json::to_borrowed_value(&mut bytes_vec) {
        Ok(simd_json::BorrowedValue::Object(obj)) => {
            obj.contains_key("tools") || obj.contains_key("tool_choice")
        }
        _ => false,
    };
    is_agentic
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agentic_detection_with_tools() {
        let payload = br#"{"model": "gpt-4", "messages": [], "tools": [{"type": "function"}]}"#;
        assert!(detect_agentic_payload(payload, "application/json"));

        let payload2 = br#"{"tool_choice": "auto", "messages": []}"#;
        assert!(detect_agentic_payload(payload2, "application/json"));
    }

    #[test]
    fn test_agentic_detection_without_tools() {
        let payload = br#"{"model": "gpt-4", "messages": [{"role": "user", "content": "hi"}]}"#;
        assert!(!detect_agentic_payload(payload, "application/json"));

        // Large payload check (over 5MB) - mocked by just checking the size condition
        let large_payload = vec![b' '; 5 * 1024 * 1024 + 1];
        assert!(!detect_agentic_payload(&large_payload, "application/json"));
        
        // Wrong content type
        assert!(!detect_agentic_payload(br#"{"tools": []}"#, "text/plain"));
    }

    #[test]
    fn test_agentic_detection_malformed_json() {
        let payload = br#"{"model": "gpt-4", "messages": ["#;
        assert!(!detect_agentic_payload(payload, "application/json"));
    }

    #[tokio::test]
    async fn test_mpsc_telemetry_offloading() {
        use axum::http::{HeaderMap, HeaderValue};
        use axum::extract::{State, Extension};
        use std::sync::Arc;
        use crate::domain::models::{AppState, RoutingState, TraceContext, ModelConfig, KeyConfig};
        use moka::future::Cache;
        use crate::infrastructure::l1_cache::L1Cache;
        use crate::infrastructure::cache_client::CacheGrpcClient;
        use testcontainers::runners::AsyncRunner;
        use testcontainers_modules::redis::Redis;

        // Spin up Redis so enforce_agentic_loop_budget succeeds
        let container = Redis::default().start().await.unwrap();
        let host_port = container.get_host_port_ipv4(6379).await.unwrap();
        let redis_url = format!("redis://127.0.0.1:{}", host_port);
        let redis_client = redis::Client::open(redis_url).unwrap();
        let redis_conn = redis_client.get_multiplexed_tokio_connection().await.unwrap();

        let (telemetry_tx, mut telemetry_rx) = tokio::sync::mpsc::channel(10);
        let circuit_breaker = Cache::builder().build();
        let loop_fallback_cache = Cache::builder().build();
        let routing_state = Arc::new(RoutingState::new());
        
        let mut map = std::collections::HashMap::new();
        map.insert("gpt-4".into(), ModelConfig {
            base_url: "http://localhost:12345".into(),
            schema_format: "openai".into(),
            keys: vec![
                KeyConfig { key_alias: "key-1".into(), api_key: "sk-1".into(), priority: 1, weight: 1 },
            ],
        });
        routing_state.state.store(Arc::new(map));

        let l1_cache = Arc::new(L1Cache::new(1024).unwrap());
        let http_client = reqwest::Client::new();
        let channel = tonic::transport::Channel::from_static("http://127.0.0.1:1").connect_lazy();
        let cache_grpc_client = CacheGrpcClient::new(channel);

        let state = unsafe {
            AppState {
                http_client,
                cache_grpc_client,
                compliance_url: String::new(),
                redis_conn,
                db_pool: std::mem::MaybeUninit::uninit().assume_init(),
                rate_limit_max: 0,
                rate_limit_window: 0,
                clickhouse_url: String::new(),
                tracer_url: String::new(),
                dashboard_url: String::new(),
                llm_api_base_url: None,
                telemetry_tx,
                l1_cache,
                routing_state,
                circuit_breaker,
                loop_fallback_cache,
                mcp_registry: crate::infrastructure::mcp_registry::McpConnectionRegistry::empty(),
                tool_registry: crate::usecases::tool_router::ToolRegistry::empty(),
            }
        };

        let state_arc = Arc::new(state);
        let ext = Extension(TraceContext {
            trace_id: "trace-1".into(),
            session_id: "session-1".into(),
            parent_trace_id: None,
        });

        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("x-session-id", HeaderValue::from_static("test-session"));
        
        // Payload with tools to trigger agentic logic
        let body = axum::body::Bytes::from(r#"{"model": "gpt-4", "tools": [{"type": "function"}]}"#);

        // Call the handler directly
        let _res = super::chat_completions(State(state_arc.clone()), ext, headers, body).await;

        // Verify MPSC received the telemetry
        let msg = telemetry_rx.recv().await.expect("Expected a telemetry message on the MPSC channel");
        
        let type_field = msg.get("type").unwrap().as_str().unwrap();
        assert_eq!(type_field, "agentic_loop_detection");
        
        let req_total = msg.get("agentic_requests_total").unwrap().as_i64().unwrap();
        assert_eq!(req_total, 1);
        
        let hits = msg.get("loop_limit_hits").unwrap().as_i64().unwrap();
        assert_eq!(hits, 0);

        std::mem::forget(state_arc);
    }
}

/// GET /v1/admin/metrics/compliance
/// Returns redacted_count and a list of residency_routes for the authenticated tenant.
///
/// # Complexity
/// - Time:  O(n) in ClickHouse, O(1) in Rust.
/// - Space: O(1) — single aggregation row is parsed.
#[instrument(skip(state, claims))]
pub async fn get_compliance_metrics(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, GatewayError> {
    let tenant_id = uuid::Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| GatewayError::ResponseBuild("Invalid tenant ID format".into()))?;

    // ---- 1️⃣  Use native DateTime comparison (no parse) ----
    let query = format!(
        "SELECT coalesce(sum(redacted_entity_count), 0) AS redacted_count \
         FROM RedEye_telemetry.compliance_engine_audit \
         WHERE tenant_id = '{}' \
           AND timestamp >= now() - INTERVAL 24 HOUR \
         FORMAT JSON",
        tenant_id
    );

    // ---- 2️⃣  Execute the query ----
    let resp = state
        .http_client
        .post(&state.clickhouse_url)
        .body(query)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "ClickHouse compliance query failed (network)");
            GatewayError::Proxy(e)
        })?;

    // ---- 3️⃣  Handle non‑2xx responses ----
    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        error!(error = %err_body, "ClickHouse compliance query returned non-2xx");
        // Return a safe default instead of bubbling up the DB error
        let residency_routes = vec![
            json!({"region": "EU (Frankfurt)", "endpoint": "api.eu.redeye.ai", "isolation": "Strict"}),
            json!({"region": "US East", "endpoint": "api.us.redeye.ai", "isolation": "Relaxed"}),
        ];
        return Ok(Json(json!({
            "redacted_count": 0,
            "residency_routes": residency_routes,
        })));
    }

    // ---- 4️⃣  Parse the JSON envelope ----
    let ch_json: Value = resp.json().await.map_err(|e| {
        error!(error = %e, "Failed to deserialise ClickHouse response");
        GatewayError::ResponseBuild(e.to_string())
    })?;

    let redacted_count = ch_json["data"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|row| row["redacted_count"].as_u64())
        .unwrap_or(0);

    // ---- 5️⃣  Add static residency route data (until dynamical) ----
    let residency_routes = vec![
        json!({"region": "EU (Frankfurt)", "endpoint": "api.eu.redeye.ai", "isolation": "Strict"}),
        json!({"region": "US East", "endpoint": "api.us.redeye.ai", "isolation": "Relaxed"}),
    ];

    Ok(Json(json!({
        "redacted_count": redacted_count,
        "residency_routes": residency_routes,
    })))
}
