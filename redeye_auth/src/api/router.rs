use axum::{
    routing::post,
    Router,
};
use super::handlers::{signup, login, onboard};
use crate::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/auth/signup", post(signup))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/onboard", post(onboard))
        // Example: in production /v1/auth/onboard would be protected by JWT middleware
        .with_state(state)
}
