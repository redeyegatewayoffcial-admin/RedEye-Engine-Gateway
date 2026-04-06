use axum::{
    body::Body,
    http::{Request, StatusCode, HeaderMap},
    middleware::Next,
    response::Response,
};
use crate::infrastructure::security::verify_jwt;

/// Extract JWT token from Authorization Bearer header or HttpOnly cookie
fn extract_token(headers: &HeaderMap) -> Option<String> {
    // First, try to extract from Authorization: Bearer header
    if let Some(auth_header) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    // If no Bearer header, try to extract from auth_token cookie
    if let Some(cookie_header) = headers.get(axum::http::header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            // Parse cookie string to find auth_token
            for cookie_pair in cookie_str.split(';') {
                let pair = cookie_pair.trim();
                if let Some((name, value)) = pair.split_once('=') {
                    if name.trim() == "auth_token" {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }

    None
}

pub async fn auth_middleware(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.method() == axum::http::Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    // Extract token from Bearer header or auth_token cookie
    let token = extract_token(req.headers()).ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = verify_jwt(&token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    // Inject claims into request extensions for handlers to use
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
