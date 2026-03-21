use axum::{
    routing::{post, get},
    Router,
};
use axum::http::Method;
use tower_http::cors::CorsLayer;
use super::handlers::{signup, login, onboard, refresh, get_api_keys};
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

    let protected_routes = Router::new()
        .route("/onboard", post(onboard))
        .route("/api-keys", get(get_api_keys))
        .route_layer(axum::middleware::from_fn(crate::api::middleware::auth::auth_middleware));

    Router::new()
        .route("/v1/auth/signup", post(signup))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/refresh", post(refresh))
        .nest("/v1/auth", protected_routes)
        .layer(cors)
        .with_state(state)
}
