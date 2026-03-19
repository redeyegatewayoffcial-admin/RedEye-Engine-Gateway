use axum::{
    routing::{post},
    Router,
};
use axum::http::Method;
use tower_http::cors::CorsLayer;
use super::handlers::{signup, login, onboard, refresh};
use crate::AppState;

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<axum::http::HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(true);

    Router::new()
        .route("/v1/auth/signup", post(signup))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/onboard", post(onboard))
        .route("/v1/auth/refresh", post(refresh))
        // Example: in production /v1/auth/onboard would be protected by JWT middleware
        .layer(cors)
        .with_state(state)
}
