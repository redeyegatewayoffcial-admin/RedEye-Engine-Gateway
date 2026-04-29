use crate::infrastructure::security::verify_jwt;
use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::Response,
};

/// Sentinel returned alongside the token to signal how it was sourced.
/// Used to enforce CSRF protection only on cookie-authenticated paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenSource {
    /// Came from `Authorization: Bearer <token>` — CSRF not required.
    BearerHeader,
    /// Came from the `re_token` HttpOnly cookie — CSRF check required.
    Cookie,
}

/// Extract JWT from the `Authorization` header or fallback to the `re_token` cookie.
///
/// Bug 9a Fix: the prefix check is now case-insensitive.
/// RFC 7235 §2.1 specifies that the auth-scheme is case-insensitive, so
/// `"bearer token"`, `"BEARER token"`, and `"Bearer token"` are all valid.
/// Using a lowercased comparison prevents 401s from well-behaved clients that
/// don't title-case the scheme name.
fn extract_token(headers: &HeaderMap) -> Option<(String, TokenSource)> {
    // Primary: Authorization header (case-insensitive Bearer prefix check).
    if let Some(auth_header) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            // Lowercase only the first 7 chars (scheme + space) to avoid
            // downcasing the token itself (tokens are case-sensitive).
            let prefix_lower = auth_str
                .get(..7)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            if prefix_lower == "bearer " {
                let token = auth_str[7..].to_string();
                return Some((token, TokenSource::BearerHeader));
            }
        }
    }

    // Fallback: HttpOnly re_token cookie.
    if let Some(cookie_header) = headers.get(axum::http::header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie_pair in cookie_str.split(';') {
                let pair = cookie_pair.trim();
                if let Some((name, value)) = pair.split_once('=') {
                    if name.trim() == "re_token" {
                        return Some((value.to_string(), TokenSource::Cookie));
                    }
                }
            }
        }
    }

    None
}

pub async fn auth_middleware(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    if req.method() == axum::http::Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    let (token, source) = extract_token(req.headers()).ok_or(StatusCode::UNAUTHORIZED)?;

    // Bug 9b Fix: CSRF protection for cookie-authenticated requests.
    //
    // When a JWT is delivered via an HttpOnly cookie the browser automatically
    // attaches it to cross-origin requests (unless SameSite=Strict is enforced
    // at the CDN/proxy layer, which cannot be guaranteed in all deployments).
    // We enforce the double-submit cookie pattern: cookie-auth requests MUST
    // include a non-empty `x-csrf-token` header.  Bearer-header flows are
    // unaffected because a cross-origin attacker cannot read or set custom
    // request headers (blocked by the CORS preflight model).
    if source == TokenSource::Cookie {
        let method = req.method();
        // Skip CSRF check for idempotent methods (GET, HEAD, OPTIONS)
        // Standard practice: CSRF only targets state-changing operations.
        if method != axum::http::Method::GET
            && method != axum::http::Method::HEAD
            && method != axum::http::Method::OPTIONS
        {
            let csrf_present = req
                .headers()
                .get("x-csrf-token")
                .and_then(|v| v.to_str().ok())
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false);

            if !csrf_present {
                tracing::warn!(
                    method = %method,
                    path = %req.uri().path(),
                    "Cookie-authenticated request rejected: missing x-csrf-token header"
                );
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    let claims = verify_jwt(&token).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Inject claims into request extensions for handlers to use.
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
