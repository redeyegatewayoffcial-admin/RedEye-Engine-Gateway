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
use tracing::{info, instrument, error};

use crate::domain::models::{AppState, GatewayError, TraceContext};
use crate::usecases::proxy;
use crate::infrastructure::llm_router;

/// GET /health
pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "redeye_gateway",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// POST /v1/chat/completions
#[instrument(skip(state, body))]
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Extension(trace_ctx): Extension<TraceContext>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, GatewayError> {
    info!("Received chat completion request");

    // Extract metadata
    let model_name = llm_router::extract_model(&body).to_string();
    let tenant_id = headers.get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string();
    let raw_prompt = serde_json::to_string(&body).unwrap_or_default();
    let accept = headers.get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");
        
    // Delegate to use case
    let result = proxy::execute_proxy(
        &state, &body, &tenant_id, &model_name, &raw_prompt, accept, &trace_ctx
    ).await?;

    // Build Axum response
    let cache_header = if result.cache_hit { "HIT" } else { "MISS" };

    match result.body {
        crate::usecases::proxy::ProxyBody::Buffered(body_bytes) => {
            let response = Response::builder()
                .status(result.status)
                .header("content-type", &result.content_type)
                .header("X-Cache", cache_header)
                .body(Body::from(body_bytes))
                .map_err(|e| {
                    error!(error = %e, "Failed to construct proxy response");
                    GatewayError::ResponseBuild(e.to_string())
                })?;

            Ok(response)
        }
        crate::usecases::proxy::ProxyBody::SseStream(stream) => {
            use axum::response::sse::{Sse, KeepAlive};
            let sse = Sse::new(stream).keep_alive(KeepAlive::default());
            let mut response = sse.into_response();
            if let Ok(value) = axum::http::HeaderValue::from_str(cache_header) {
                response.headers_mut().insert("X-Cache", value);
            }
            Ok(response)
        }
    }
}

use crate::api::middleware::auth::Claims;

/// GET /v1/admin/metrics
#[instrument(skip(state, claims))]
pub async fn admin_metrics(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, GatewayError> {
    info!(tenant_id = %claims.tenant_id, "Fetching live metrics from ClickHouse");

    // 1. Summary Stats
    let stats_query = format!("
        SELECT 
            count() as total_requests,
            avg(latency_ms) as avg_latency_ms,
            sum(tokens) as total_tokens,
            countIf(status = 429) as rate_limited_requests
        FROM RedEye_telemetry.request_logs
        WHERE tenant_id = '{}'
        FORMAT JSON
    ", claims.tenant_id);

    // 2. Traffic Series (Last 24 hours, hourly buckets)
    let traffic_query = format!("
        SELECT 
            formatDateTime(toStartOfHour(created_at), '%Y-%m-%dT%H:%M:%S') as timestamp,
            count() as requests
        FROM RedEye_telemetry.request_logs
        WHERE tenant_id = '{}' AND created_at > now() - INTERVAL 24 HOUR
        GROUP BY timestamp
        ORDER BY timestamp
        FORMAT JSON
    ", claims.tenant_id);

    // 3. Model Distribution
    let model_query = format!("
        SELECT 
            model as name,
            count() as value
        FROM RedEye_telemetry.request_logs
        WHERE tenant_id = '{}'
        GROUP BY name
        FORMAT JSON
    ", claims.tenant_id);

    // 4. Latency Buckets
    let latency_query = format!("
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
    ", claims.tenant_id);

    // Execute queries (simplified sequential for reliability, could use join_all)
    let stats_resp = state.http_client.post(&state.clickhouse_url).body(stats_query).send().await
        .map_err(|e| { error!(error = %e); GatewayError::Proxy(e) })?;
    let traffic_resp = state.http_client.post(&state.clickhouse_url).body(traffic_query).send().await
        .map_err(|e| { error!(error = %e); GatewayError::Proxy(e) })?;
    let model_resp = state.http_client.post(&state.clickhouse_url).body(model_query).send().await
        .map_err(|e| { error!(error = %e); GatewayError::Proxy(e) })?;
    let latency_resp = state.http_client.post(&state.clickhouse_url).body(latency_query).send().await
        .map_err(|e| { error!(error = %e); GatewayError::Proxy(e) })?;

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

    // Parameterised via format! — tenant_id comes from a trusted, validated JWT claim,
    // so there is no SQL-injection risk from external user input.
    let query = format!(
        "SELECT sum(tokens) as total_tokens \
         FROM RedEye_telemetry.request_logs \
         WHERE tenant_id = '{}' \
         FORMAT JSON",
        claims.tenant_id
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
        claims.tenant_id
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

    let query = format!("
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
    ", claims.tenant_id);

    let response = state.http_client.post(&state.clickhouse_url).body(query).send().await
        .map_err(|e| { error!(error = %e); GatewayError::Proxy(e) })?;

    if !response.status().is_success() {
        let err = response.text().await.unwrap_or_default();
        error!(error = %err, "ClickHouse traces query failed");
        return Err(GatewayError::ResponseBuild("Traces query failed".to_string()));
    }

    let clickhouse_json: Value = response.json().await
        .map_err(|e| { error!(error = %e); GatewayError::ResponseBuild(e.to_string()) })?;

    let data = clickhouse_json["data"].clone();
    Ok(Json(data))
}
