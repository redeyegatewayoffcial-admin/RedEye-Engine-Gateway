//! main.rs — RedEye Tracer Microservice entry point.
//!
//! Owns all observability data: trace ingestion, compliance audit storage, and query APIs.

use std::{net::SocketAddr, sync::Arc, env};
use axum::{routing::{get, post}, Router};
use tower_http::cors::CorsLayer;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod domain;
mod usecases;
mod infrastructure;
mod api;
mod error;

use infrastructure::clickhouse_repo::ClickHouseRepo;
use infrastructure::latency_worker::LatencyWorker;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    // Init tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    
    info!("Starting RedEye Tracer Microservice...");

    // ClickHouse connection
    let clickhouse_url = std::env::var("CLICKHOUSE_URL")
        .unwrap_or_else(|_| {
            let default_url = "http://localhost:8123".to_string();
            tracing::warn!("CLICKHOUSE_URL not set, using default: {}", default_url);
            default_url
        });

    let repo = ClickHouseRepo::new(clickhouse_url);

    // Ensure schema exists on startup
    if let Err(e) = repo.ensure_schema().await {
        tracing::warn!("Schema verification warning (may already exist): {}", e);
    }

    let shared_repo = Arc::new(repo);

    // ── Redis + Latency Worker ───────────────────────────────────────────────
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| {
            let default = "redis://127.0.0.1:6379".to_string();
            tracing::warn!("REDIS_URL not set, using default: {}", default);
            default
        });

    match redis::Client::open(redis_url.as_str()) {
        Ok(client) => match client.get_multiplexed_tokio_connection().await {
            Ok(redis_conn) => {
                let worker = LatencyWorker::new(shared_repo.clone(), redis_conn);
                tokio::spawn(async move { worker.run().await });
                info!("Latency ranking worker spawned");
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to connect to Redis — latency worker disabled");
            }
        },
        Err(e) => {
            tracing::error!(error = %e, "Invalid Redis URL — latency worker disabled");
        }
    }

    // CORS configuration - strict origin validation
    let cors = create_cors_layer();

    let app = Router::new()
        .route("/v1/traces/ingest", post(api::handlers::ingest_handler))
        .route("/v1/traces", get(api::handlers::traces_handler))
        .route("/v1/audit", get(api::handlers::audit_handler))
        .route("/v1/compliance/metrics", get(api::handlers::compliance_metrics_handler))
        .route("/health", get(|| async { axum::Json(serde_json::json!({"status": "ok", "service": "redeye_tracer"})) }))
        .layer(cors)
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB limit
        .with_state(shared_repo);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8082));
    info!("🔍 RedEye Tracer listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app).await
        .expect("Tracer server encountered a fatal error");
}

/// Creates a CORS layer with strict origin validation.
/// In production, only allows the DASHBOARD_URL environment variable.
/// Falls back to restricted local development origins if DASHBOARD_URL is not set.
fn create_cors_layer() -> CorsLayer {
    let dashboard_url = env::var("DASHBOARD_URL").ok();
    
    let origins = match dashboard_url {
        Some(url) => {
            vec![url.parse().expect("Invalid DASHBOARD_URL format")]
        }
        None => {
            // Restricted development origins only
            vec![
                "http://localhost:5173".parse().unwrap(),
                "http://localhost:3000".parse().unwrap(),
            ]
        }
    };
    
    CorsLayer::new()
        .allow_origin(origins)
        .allow_credentials(true)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::OPTIONS])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
}
