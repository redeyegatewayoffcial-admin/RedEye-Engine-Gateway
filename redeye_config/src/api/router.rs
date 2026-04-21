//! Axum router factory for the `redeye_config` service.

use axum::{
    routing::{delete, get, put},
    Router,
};
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;

use crate::{
    api::handlers::{get_config, list_api_keys, revoke_api_key, upsert_config},
    AppState,
};

/// Constructs the top-level Axum [`Router`] with all routes, CORS, and shared
/// application state.
///
/// # Route table
///
/// | Method | Path                                      | Handler           |
/// |--------|-------------------------------------------|-------------------|
/// | GET    | `/v1/config/:tenant_id`                   | `get_config`      |
/// | PUT    | `/v1/config/:tenant_id`                   | `upsert_config`   |
/// | GET    | `/v1/config/:tenant_id/api-keys`          | `list_api_keys`   |
/// | DELETE | `/v1/config/:tenant_id/api-keys/:key_id`  | `revoke_api_key`  |
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // ── Config management ───────────────────────────────────────────────
        .route("/v1/config/:tenant_id",                          get(get_config))
        .route("/v1/config/:tenant_id",                          put(upsert_config))
        // ── API Key lifecycle ───────────────────────────────────────────────
        .route("/v1/config/:tenant_id/api-keys",                 get(list_api_keys))
        .route("/v1/config/:tenant_id/api-keys/:key_id",         delete(revoke_api_key))
        // ── Middleware ──────────────────────────────────────────────────────
        .layer(build_cors())
        .layer(axum::extract::DefaultBodyLimit::max(256 * 1024)) // 256 KB
        .with_state(state)
}

/// Builds a strict CORS policy.
///
/// In production, reads `DASHBOARD_URL` from the environment.
/// Falls back to local-development origins if the variable is unset.
fn build_cors() -> CorsLayer {
    let allowed_origin: HeaderValue = std::env::var("DASHBOARD_URL")
        .unwrap_or_else(|_| "http://localhost:5173".into())
        .parse()
        .unwrap_or_else(|_| {
            // If the env var contains an invalid header value, fall back safely.
            "http://localhost:5173"
                .parse()
                .expect("localhost origin is always a valid header value")
        });

    CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
        ])
}
