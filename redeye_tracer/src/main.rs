//! main.rs — RedEye Tracer Microservice entry point.
//!
//! Owns all observability data: trace ingestion, compliance audit storage, and query APIs.

use std::{net::SocketAddr, sync::Arc};
use axum::{routing::{get, post}, Router};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod domain;
mod usecases;
mod infrastructure;
mod api;

use infrastructure::clickhouse_repo::ClickHouseRepo;

#[tokio::main]
async fn main() {
    // Init tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    info!("Starting RedEye Tracer Microservice...");

    // ClickHouse connection
    let clickhouse_url = std::env::var("CLICKHOUSE_URL")
        .unwrap_or_else(|_| "http://localhost:8123/?user=RedEye&password=clickhouse_secret&database=RedEye_telemetry".to_string());

    let repo = ClickHouseRepo::new(clickhouse_url);

    // Ensure schema exists on startup
    if let Err(e) = repo.ensure_schema().await {
        tracing::warn!("Schema verification warning (may already exist): {}", e);
    }

    let shared_repo = Arc::new(repo);

    // CORS for dashboard
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/v1/traces/ingest", post(api::handlers::ingest_handler))
        .route("/v1/traces", get(api::handlers::traces_handler))
        .route("/v1/audit", get(api::handlers::audit_handler))
        .route("/health", get(|| async { axum::Json(serde_json::json!({"status": "ok", "service": "redeye_tracer"})) }))
        .layer(cors)
        .with_state(shared_repo);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8082));
    info!("🔍 RedEye Tracer listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app).await
        .expect("Tracer server encountered a fatal error");
}
