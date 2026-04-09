//! api/routes.rs — Single source of truth for all route registrations.

use std::sync::Arc;
use std::env;

use axum::{
    body::Body,
    extract::{Request, ConnectInfo},
    http::header::{AUTHORIZATION, CONTENT_TYPE},
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::api::handlers;
use crate::domain::models::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    // LLM routes with rate limiting and authentication
    let proxy_routes = Router::new()
        .route("/chat/completions", post(handlers::chat_completions))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::api::middleware::auth::auth_middleware,
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::api::middleware::rate_limit::rate_limit_middleware,
        ));

    // Admin routes with authentication middleware
    let admin_routes = Router::new()
        .route("/metrics", get(handlers::admin_metrics))
        .route("/metrics/usage", get(handlers::get_usage_metrics))
        .route("/metrics/cache", get(handlers::get_cache_metrics))
        .route("/metrics/compliance", get(handlers::get_compliance_metrics))
        .route("/metrics/hot-swaps", get(handlers::get_hot_swaps))
        .route("/billing/breakdown", get(handlers::get_billing_breakdown))
        .route("/traces", get(handlers::get_traces))
        .route("/security/alerts", get(handlers::security_alerts))
        .route("/analytics", get(handlers::admin_metrics)) // Now protected by auth middleware
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::api::middleware::auth::auth_middleware,
        ));

    // CORS with strict origin validation
    let cors = create_cors_layer();

    Router::new()
        .nest("/v1", proxy_routes)
        .nest("/v1/admin", admin_routes)
        // Removed: .route("/v1/admin/analytics", get(handlers::admin_metrics).with_state(state.clone()))
        // Now protected under /v1/admin/analytics above
        .route("/health", get(handlers::health_check))
        // Global trace context middleware
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::api::middleware::trace_context::trace_context_middleware,
        ))
        .layer(cors)
        .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB limit for LLM payloads
        .with_state(state)
}

/// Creates a CORS layer with strict origin validation.
/// In production, only allows the DASHBOARD_URL environment variable.
/// Falls back to restricted local development origins if DASHBOARD_URL is not set.
fn create_cors_layer() -> CorsLayer {
    let dashboard_url = env::var("DASHBOARD_URL").ok();
    
    let origins = match dashboard_url {
        Some(url) => {
            vec![url.parse().expect("Invalid DASHBOARD_URL format")]
        }
        None => {
            // Restricted development origins only
            vec![
                "http://localhost:5173".parse().unwrap(),
                "http://localhost:3000".parse().unwrap(),
            ]
        }
    };
    
    CorsLayer::new()
        .allow_origin(origins)
        .allow_credentials(true)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE, axum::http::Method::OPTIONS])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
}
