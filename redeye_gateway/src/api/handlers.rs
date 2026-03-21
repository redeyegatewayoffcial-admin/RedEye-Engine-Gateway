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

    let query = format!("
        SELECT 
            count() as total_requests,
            avg(latency_ms) as avg_latency_ms,
            sum(tokens) as total_tokens,
            countIf(status = 429) as rate_limited_requests
        FROM RedEye_telemetry.request_logs
        WHERE tenant_id = '{}'
        FORMAT JSON
    ", claims.tenant_id);

    let response = state.http_client.post(&state.clickhouse_url).body(query).send().await
        .map_err(|e| { error!(error = %e, "ClickHouse metrics failed"); GatewayError::Proxy(e) })?;

    if !response.status().is_success() {
        let err = response.text().await.unwrap_or_default();
        error!(error = %err, "ClickHouse metrics query failed");
        return Err(GatewayError::ResponseBuild("Metrics query failed".to_string()));
    }

    let mut clickhouse_json: Value = response.json().await
        .map_err(|e| { error!(error = %e, "Failed to parse ClickHouse JSON"); GatewayError::ResponseBuild(e.to_string()) })?;

    let row = clickhouse_json.get_mut("data")
        .and_then(|data| data.as_array_mut())
        .and_then(|arr| arr.pop())
        .unwrap_or_else(|| json!({"total_requests": "0", "avg_latency_ms": 0.0, "total_tokens": "0", "rate_limited_requests": "0"}));

    Ok(Json(row))
}
