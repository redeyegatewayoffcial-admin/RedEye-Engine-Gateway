//! src/api/middleware/rate_limit.rs — Redis Fixed Window rate limiter.
//!
//! Applies an atomic Lua script (INCR + conditional EXPIRE) to enforce
//! a per-tenant or per-IP request limit.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use redis::AsyncCommands;
use serde_json::json;
use tracing::{debug, error, warn};

use crate::domain::models::AppState;

const RATE_LIMIT_LUA: &str = r#"
    local current = redis.call('INCR', KEYS[1])
    if current == 1 then
        redis.call('EXPIRE', KEYS[1], ARGV[1])
    end
    return current
"#;

pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let identifier = headers
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()))
        .unwrap_or("anonymous");

    let redis_key = format!("rl:tenant:{}", identifier);

    let mut conn = state.redis_conn.clone();

    let script = redis::Script::new(RATE_LIMIT_LUA);
    let limit = state.rate_limit_max;
    let window_secs = state.rate_limit_window;

    let current_requests: i64 = script
        .key(&redis_key)
        .arg(window_secs)
        .invoke_async(&mut conn)
        .await
        .map_err(|e| {
            error!(error = %e, "Redis rate limit script execution failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    debug!(tenant = identifier, requests = current_requests, limit = limit, "Rate limit check");

    let remaining = i64::max(0, limit as i64 - current_requests);
    let ttl: i64 = match current_requests {
        1 => window_secs as i64,
        _ => conn.ttl::<_, i64>(&redis_key).await.unwrap_or(window_secs as i64),
    };

    if current_requests > (limit as i64) {
        warn!(tenant = identifier, "Rate limit exceeded (429)");
        let mut response = Json(json!({
            "error": {"code": 429, "message": format!("Rate limit exceeded. Maximum {} requests per {} seconds.", limit, window_secs)}
        })).into_response();
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        let h = response.headers_mut();
        h.insert("x-ratelimit-limit", HeaderValue::from(limit));
        h.insert("x-ratelimit-remaining", HeaderValue::from(0));
        h.insert("x-ratelimit-reset", HeaderValue::from(ttl));
        return Ok(response);
    }

    let mut response = next.run(request).await;
    let h = response.headers_mut();
    h.insert("x-ratelimit-limit", HeaderValue::from(limit));
    h.insert("x-ratelimit-remaining", HeaderValue::from(remaining));
    h.insert("x-ratelimit-reset", HeaderValue::from(ttl));

    Ok(response)
}
