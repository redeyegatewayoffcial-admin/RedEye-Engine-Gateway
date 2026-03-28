use axum::{
    routing::{post, get},
    Router,
};
use axum::http::Method;
use tower_http::cors::CorsLayer;
use super::handlers::{signup, login, onboard, refresh, get_api_keys, request_otp, verify_otp, google_login, google_callback, github_login, github_callback};
use crate::AppState;

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
        ]);

    let protected_routes = Router::new()
        .route("/onboard", post(onboard))
        .route("/api-keys", get(get_api_keys))
        .route_layer(axum::middleware::from_fn(crate::api::middleware::auth::auth_middleware));

    Router::new()
        .route("/v1/auth/signup", post(signup))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/refresh", post(refresh))
        .route("/v1/auth/otp/request", post(request_otp))
        .route("/v1/auth/otp/verify", post(verify_otp))
        .route("/v1/auth/google/login", get(google_login))
        .route("/v1/auth/google/callback", get(google_callback))
        .route("/v1/auth/github/login", get(github_login))
        .route("/v1/auth/github/callback", get(github_callback))
        .nest("/v1/auth", protected_routes)
        .layer(cors)
        .with_state(state)
}
