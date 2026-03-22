use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use crate::infrastructure::security::verify_jwt;

pub async fn auth_middleware(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.method() == axum::http::Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    let auth_header = req.headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|val| val.to_str().ok())
        .and_then(|val| val.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = verify_jwt(auth_header).map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    // Inject claims into request extensions for handlers to use
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
