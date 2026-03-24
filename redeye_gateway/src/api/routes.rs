//! api/routes.rs — Single source of truth for all route registrations.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

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

    // Admin routes
    let admin_routes = Router::new()
        .route("/metrics", get(handlers::admin_metrics))
        .route("/metrics/usage", get(handlers::get_usage_metrics))
        .route("/billing/breakdown", get(handlers::get_billing_breakdown))
        .route("/traces", get(handlers::get_traces))
        .route("/security/alerts", get(handlers::security_alerts))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::api::middleware::auth::auth_middleware,
        ));

    // CORS for React dashboard
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .nest("/v1", proxy_routes)
        .nest("/v1/admin", admin_routes.clone())
        .route("/v1/admin/analytics", get(handlers::admin_metrics).with_state(state.clone()))
        .route("/health", get(handlers::health_check))
        // Global trace context middleware
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::api::middleware::trace_context::trace_context_middleware,
        ))
        .layer(cors)
        .with_state(state)
}
