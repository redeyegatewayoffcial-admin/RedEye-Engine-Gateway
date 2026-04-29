use super::handlers::{
    add_provider_key, get_api_keys, get_provider_keys, github_callback, github_login,
    google_callback, google_login, login, onboard, refresh, request_otp, signup, verify_otp,
};
use crate::AppState;
use axum::http::{HeaderValue, Method};
use axum::{
    routing::{get, post},
    Router,
};
use std::env;
use tower_http::cors::CorsLayer;

pub fn create_router(state: AppState) -> Router {
    let cors = create_cors_layer();

    let protected_routes = Router::new()
        .route("/onboard", post(onboard))
        .route("/api-keys", get(get_api_keys))
        .route("/provider-keys", post(add_provider_key))
        .route("/provider-keys", get(get_provider_keys))
        .route_layer(axum::middleware::from_fn(
            crate::api::middleware::auth::auth_middleware,
        ));

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
        .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024)) // 2MB limit for auth payloads
        .with_state(state)
}

/// Creates a CORS layer with strict origin validation.
/// In production, only allows the DASHBOARD_URL environment variable.
/// Falls back to restricted local development origins if DASHBOARD_URL is not set.
fn create_cors_layer() -> CorsLayer {
    let dashboard_url = env::var("DASHBOARD_URL").ok();

    let origins = match dashboard_url {
        Some(url) => {
            vec![url
                .parse::<HeaderValue>()
                .expect("Invalid DASHBOARD_URL format")]
        }
        None => {
            // Restricted development origins only
            vec![
                "http://localhost:5173".parse::<HeaderValue>().unwrap(),
                "http://localhost:3000".parse::<HeaderValue>().unwrap(),
                "http://127.0.0.1:5173".parse::<HeaderValue>().unwrap(),
                "http://127.0.0.1:3000".parse::<HeaderValue>().unwrap(),
            ]
        }
    };

    CorsLayer::new()
        .allow_origin(origins)
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
            "x-csrf-token".parse().unwrap(),
        ])
}
