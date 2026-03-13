//! src/middleware/rate_limit.rs — Redis Fixed Window rate limiter.
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

use crate::AppState;

/// The atomic Lua script for a Fixed Window rate limiter.
///
/// Logic:
/// 1. Increment the key (creates it if it doesn't exist).
/// 2. If the value is 1 (meaning it was just created), set the TTL (expiration in seconds).
/// 3. Return the current count.
///
/// This requires only 1 RTT to Redis and avoids race conditions.
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
    // ConnectInfo allows us to extract the underlying TCP IP address as a fallback
    // connect_info: ConnectInfo<std::net::SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // ── 1. Identify Client (Tenant ID or IP) ───────────────────────────────
    // In an enterprise gateway, we expect a Tenant/Project/API Key ID.
    // For now, look for `X-Tenant-ID`, fallback to `X-Forwarded-For`, fallback to "anonymous".
    // (In production, replace "anonymous" with `connect_info.0.ip().to_string()`)
    let identifier = headers
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()))
        .unwrap_or("anonymous");

    let redis_key = format!("rl:tenant:{}", identifier);

    // ── 2. Acquire Redis Connection ─────────────────────────────────────────
    // We grab a connection from the pool. If Redis is down, we fail open or closed?
    // For a security policy gateway, we generally fail CLOSED (or return 500).
    // Here we'll return 500 if Redis is completely unreachable.
    let mut conn = state.redis_pool.get().await.map_err(|e| {
        error!(error = %e, "Failed to get Redis connection from pool");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // ── 3. Execute Atomic Lua Script ────────────────────────────────────────
    let script = redis::Script::new(RATE_LIMIT_LUA);

    // Limit configuration from state
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

    debug!(
        tenant = identifier,
        requests = current_requests,
        limit = limit,
        "Rate limit check"
    );

    // ── 4. Verify Against Limit & Pre-calculate Headers ─────────────────────
    let remaining = i64::max(0, limit as i64 - current_requests);
    
    // We optionally fetch the TTL to power the `X-RateLimit-Reset` header.
    // This adds 1 RTT, but makes the API much friendlier.
    let ttl: i64 = match current_requests {
        1 => window_secs as i64, // we just set it
        _ => conn.ttl(&redis_key).await.unwrap_or(window_secs as i64),
    };

    let is_rate_limited = current_requests > (limit as i64);

    if is_rate_limited {
        warn!(tenant = identifier, "Rate limit exceeded (429)");
        
        let mut response = Json(json!({
            "error": {
                "code": 429,
                "message": format!("Rate limit exceeded. Maximum {} requests per {} seconds.", limit, window_secs),
            }
        })).into_response();
        
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        
        // Append headers
        let h = response.headers_mut();
        h.insert("x-ratelimit-limit", HeaderValue::from(limit));
        h.insert("x-ratelimit-remaining", HeaderValue::from(0));
        h.insert("x-ratelimit-reset", HeaderValue::from(ttl));

        return Ok(response);
    }

    // ── 5. Pass to Next Handler ─────────────────────────────────────────────
    let mut response = next.run(request).await;

    // Append headers to successful response
    let h = response.headers_mut();
    h.insert("x-ratelimit-limit", HeaderValue::from(limit));
    h.insert("x-ratelimit-remaining", HeaderValue::from(remaining));
    h.insert("x-ratelimit-reset", HeaderValue::from(ttl));

    Ok(response)
}
