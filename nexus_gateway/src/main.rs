//! main.rs — NexusAI Gateway entry point.
//!
//! Responsibilities:
//!   - Load `.env` configuration
//!   - Initialize the `tracing` subscriber for structured logging
//!   - Build the Axum router and bind it to the configured port
//!   - Construct a shared, connection-pooled `reqwest::Client` (expensive to create)

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use dotenvy::dotenv;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod handlers;
mod middleware;
mod routes;

/// Application-wide shared state passed to every handler via Axum's `State` extractor.
///
/// Wrapping in `Arc` allows cheap `.clone()` across threads without copying data.
#[derive(Clone)]
pub struct AppState {
    /// A single, connection-pooled HTTP client reused for all upstream requests.
    /// `reqwest::Client` is `Send + Sync`, making it safe to share across threads.
    pub http_client: reqwest::Client,

    /// The OpenAI API key loaded from the environment at startup.
    pub openai_api_key: String,

    /// Async connection pool to Redis for rate-limiting.
    pub redis_pool: deadpool_redis::Pool,

    /// Maximum requests allowed per rate limit window.
    pub rate_limit_max: u32,
    /// Rate limit window in seconds.
    pub rate_limit_window: u32,

    /// ClickHouse HTTP endpoint for async telemetry logging
    pub clickhouse_url: String,
}

#[tokio::main]
async fn main() {
    // ── 1. Load `.env` ──────────────────────────────────────────────────────
    // `ok()` intentionally swallows the error: in production (Docker) we rely on
    // real env vars; `.env` is a developer convenience only.
    dotenv().ok();

    // ── 2. Init structured logging ──────────────────────────────────────────
    // Reads the `RUST_LOG` env var (e.g., "info", "nexus_gateway=debug").
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    // ── 3. Read required config ─────────────────────────────────────────────
    let openai_api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set in environment or .env file");

    let port: u16 = std::env::var("GATEWAY_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("GATEWAY_PORT must be a valid port number");

    // ── 4. Build shared state ───────────────────────────────────────────────
    let redis_url = std::env::var("REDIS_URL")
        .expect("REDIS_URL must be set in environment or .env file");
    
    let clickhouse_url = std::env::var("CLICKHOUSE_URL")
        .expect("CLICKHOUSE_URL must be set in environment or .env file");

    let rate_limit_max: u32 = std::env::var("RATE_LIMIT_MAX_REQUESTS")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .expect("RATE_LIMIT_MAX_REQUESTS must be a valid integer");
        
    let rate_limit_window: u32 = std::env::var("RATE_LIMIT_WINDOW_SECS")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .expect("RATE_LIMIT_WINDOW_SECS must be a valid integer");

    // Initialize Redis connection pool
    let cfg = deadpool_redis::Config::from_url(redis_url);
    let redis_pool = cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Failed to create Redis connection pool");

    // `reqwest::Client` manages a connection pool internally. We construct it
    // once here and share it — never construct it per-request.
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120)) // Long timeout for streaming LLM responses
        .build()
        .expect("Failed to construct reqwest HTTP client");

    let state = Arc::new(AppState {
        http_client,
        openai_api_key,
        redis_pool,
        rate_limit_max,
        rate_limit_window,
        clickhouse_url,
    });

    // ── 5. Build router ─────────────────────────────────────────────────────
    let app: Router = routes::create_router(state);

    // ── 6. Bind & serve ─────────────────────────────────────────────────────
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 NexusAI Gateway listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app)
        .await
        .expect("Axum server encountered a fatal error");
}
