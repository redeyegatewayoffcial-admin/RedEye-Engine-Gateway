//! main.rs — RedEye Gateway entry point (Clean Architecture).
//!
//! Bootstrap only: load config, init tracing, build state, start server.

use std::{net::SocketAddr, sync::Arc};
use axum::Router;
use dotenvy::dotenv;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod domain;
mod usecases;
mod infrastructure;
mod api;

use domain::models::AppState;

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let openai_api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set");

    let port: u16 = std::env::var("GATEWAY_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("GATEWAY_PORT must be a valid port number");

    let redis_url = std::env::var("REDIS_URL")
        .expect("REDIS_URL must be set");

    let clickhouse_url = std::env::var("CLICKHOUSE_URL")
        .expect("CLICKHOUSE_URL must be set");

    let tracer_url = std::env::var("TRACER_URL")
        .unwrap_or_else(|_| "http://localhost:8082".to_string());

    let cache_url = std::env::var("CACHE_URL")
        .unwrap_or_else(|_| "http://localhost:8081".to_string());

    let rate_limit_max: u32 = std::env::var("RATE_LIMIT_MAX_REQUESTS")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .expect("RATE_LIMIT_MAX_REQUESTS must be a valid integer");

    let rate_limit_window: u32 = std::env::var("RATE_LIMIT_WINDOW_SECS")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .expect("RATE_LIMIT_WINDOW_SECS must be a valid integer");

    let cfg = deadpool_redis::Config::from_url(redis_url);
    let redis_pool = cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Failed to create Redis connection pool");

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("Failed to construct reqwest HTTP client");

    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
        
    let db_pool = sqlx::PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to Postgres DB");

    let state = Arc::new(AppState {
        http_client,
        openai_api_key,
        cache_url,
        redis_pool,
        db_pool,
        rate_limit_max,
        rate_limit_window,
        clickhouse_url,
        tracer_url,
    });

    let app: Router = api::routes::create_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 RedEye Gateway listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app).await
        .expect("Axum server encountered a fatal error");
}
