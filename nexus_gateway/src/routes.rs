//! routes.rs — Axum router definitions for NexusAI Gateway.
//!
//! This module is the single source of truth for all route registrations.
//! Keeping route definitions separate from handler logic makes it easy to:
//!   - Audit the public API surface at a glance
//!   - Add middleware layers (auth, rate-limit) per-route group in later phases

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::{handlers, AppState};

/// Constructs and returns the fully configured Axum `Router`.
///
/// `state` is wrapped in `Arc` so it is cheaply cloned per-request by Axum.
pub fn create_router(state: Arc<AppState>) -> Router {
    // LLM routes requiring rate limiting and API key injection
    let proxy_routes = Router::new()
        .route("/chat/completions", post(handlers::chat_completions))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::rate_limit::rate_limit_middleware,
        ));

    // Admin routes — Phase 5 Dashboard integration
    let admin_routes = Router::new()
        .route("/metrics", get(handlers::admin_metrics));

    // Permissive CORS for the React dashboard (dev mode)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Aggregate router
    Router::new()
        .nest("/v1", proxy_routes)
        .nest("/v1/admin", admin_routes)
        // ── Health Check ──────────────────────────────────────────────────
        // Used by Docker, load balancers, and k8s liveness probes.
        // Intentionally excluded from rate limiting.
        .route(
            "/health",
            get(handlers::health_check),
        )
        // Attach CORS layer so frontend can query it
        .layer(cors)
        // Attach shared state — available in every handler via `State<Arc<AppState>>`
        .with_state(state)
}
