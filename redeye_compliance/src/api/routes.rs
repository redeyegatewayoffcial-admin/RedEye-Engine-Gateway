use std::sync::Arc;
use axum::{
    routing::post,
    Router,
};

use crate::api::middleware::geo_routing::{geo_routing_middleware, SharedConfig};
use crate::api::middleware::security::{security_guard_middleware, SecurityState};
use crate::api::handlers::{check_routing, redact_prompt};
use crate::usecases::pii_engine::PiiEngine;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<SharedConfig>,
    pub pii_engine: Arc<PiiEngine>,
    pub security_state: SecurityState,
}

impl axum::extract::FromRef<AppState> for Arc<PiiEngine> {
    fn from_ref(state: &AppState) -> Self {
        state.pii_engine.clone()
    }
}

impl axum::extract::FromRef<AppState> for SecurityState {
    fn from_ref(state: &AppState) -> Self {
        state.security_state.clone()
    }
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Step 2 & 4 routes (LLM proxy endpoints)
        .route("/v1/llm/completion", post(check_routing))
        .layer(axum::middleware::from_fn_with_state(
            state.config.clone(),
            geo_routing_middleware,
        ))
        // Run security checks *before* geo-routing (so bad requests die early)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            security_guard_middleware,
        ))
        // Step 3 route (internal sync)
        .route("/v1/compliance/redact", post(redact_prompt))
        .with_state(state) // Provide struct that contains all state
}
