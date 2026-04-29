//! src/api/middleware/rate_limit.rs — Redis Fixed Window rate limiter.
//!
//! Applies an atomic Lua script (INCR + conditional EXPIRE) to enforce
//! a per-tenant or per-IP request limit.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{ConnectInfo, Extension, Request, State},
    http::{header::SET_COOKIE, request::Parts, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use redis::AsyncCommands;
use serde_json::json;
use std::net::SocketAddr;
use tracing::{debug, error, warn};

use crate::domain::models::AppState;

const RATE_LIMIT_LUA: &str = r#"
    local current = redis.call('INCR', KEYS[1])
    if current == 1 then
        redis.call('EXPIRE', KEYS[1], ARGV[1])
    end
    return current
"#;

pub fn extract_identifiers(headers: &HeaderMap, addr: &SocketAddr) -> (String, String) {
    let tenant_id = headers
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    let user_id = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "anon_user".to_string());

    (tenant_id, user_id)
}

pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // ConnectInfo is only available with a real TcpListener (not in oneshot tests).
    // Gracefully fall back to a loopback addr so in-process integration tests work.
    let fallback_addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 0);
    let addr = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0)
        .unwrap_or(fallback_addr);

    let headers = request.headers().clone();
    let (tenant_id, user_id) = extract_identifiers(&headers, &addr);

    let redis_key = format!("rl:{}:u:{}", tenant_id, user_id);

    let mut conn = state.redis_conn.clone();

    let script = redis::Script::new(RATE_LIMIT_LUA);
    let limit = state.rate_limit_max;
    let window_secs = state.rate_limit_window;

    let current_requests: i64 = match script
        .key(&redis_key)
        .arg(window_secs)
        .invoke_async(&mut conn)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "Redis rate limit failed, bypassing cache (fail-open)");
            0
        }
    };

    debug!(tenant_id = %tenant_id, user_id = %user_id, requests = current_requests, limit = limit, "Rate limit check");

    let remaining = i64::max(0, limit as i64 - current_requests);
    let ttl: i64 = match current_requests {
        1 => window_secs as i64,
        _ => conn
            .ttl::<_, i64>(&redis_key)
            .await
            .unwrap_or(window_secs as i64),
    };

    if current_requests > (limit as i64) {
        warn!(tenant_id = %tenant_id, user_id = %user_id, "Rate limit exceeded (429)");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_success_extract_identifiers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-tenant-id", HeaderValue::from_static("tenant_123"));
        headers.insert("x-user-id", HeaderValue::from_static("user_456"));

        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let (tenant, user) = extract_identifiers(&headers, &addr);

        assert_eq!(tenant, "tenant_123");
        assert_eq!(user, "user_456");
    }

    #[test]
    fn test_failure_handling_missing_headers() {
        let headers = HeaderMap::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);

        // Tenant ID missing -> fallback to IP address
        // User ID missing -> fallback to "anon_user"
        let (tenant, user) = extract_identifiers(&headers, &addr);

        assert_eq!(tenant, "192.168.1.100");
        assert_eq!(user, "anon_user");
    }
}
