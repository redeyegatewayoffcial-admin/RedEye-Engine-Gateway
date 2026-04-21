//! main.rs — RedEye Gateway entry point (Clean Architecture).
//!
//! Bootstrap only: load config, init tracing, build state, start server.

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use dotenvy::dotenv;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// Re-use modules from lib.rs (the library crate) so integration tests
// and the binary see the exact same types.
use redeye_gateway::api;
use redeye_gateway::domain;
#[allow(unused_imports)]
use redeye_gateway::infrastructure;
#[allow(unused_imports)]
use redeye_gateway::usecases;

use domain::models::AppState;
use infrastructure::cache_client::CacheGrpcClient;

// ── Config defaults ───────────────────────────────────────────────────────────

const DEFAULT_PORT: &str = "8080";
const DEFAULT_TRACER_URL: &str = "http://localhost:8082";
const DEFAULT_CACHE_GRPC_ENDPOINT: &str = "http://localhost:50051";
const DEFAULT_COMPLIANCE_URL: &str = "http://localhost:8083";
const DEFAULT_RATE_LIMIT_MAX: &str = "60";
const DEFAULT_RATE_LIMIT_WINDOW: &str = "60";
const HTTP_TIMEOUT_SECS: u64 = 120;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Reads a required environment variable, panicking with a clear message on absence.
fn require_env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{} must be set", key))
}

/// Reads an optional environment variable, returning `default` when absent.
fn optional_env(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Parses a numeric environment variable, panicking with a clear message on failure.
fn parse_env<T: std::str::FromStr>(key: &str, default: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    optional_env(key, default)
        .parse::<T>()
        .unwrap_or_else(|_| panic!("{} must be a valid number", key))
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    // ── Config ────────────────────────────────────────────────────────────────
    let port: u16              = parse_env("GATEWAY_PORT", DEFAULT_PORT);
    let _jwt_secret            = require_env("JWT_SECRET"); // strictly enforces presence at boot
    let redis_url              = require_env("REDIS_URL");
    let clickhouse_url         = require_env("CLICKHOUSE_URL");
    let db_url                 = require_env("DATABASE_URL");
    let tracer_url             = optional_env("TRACER_URL", DEFAULT_TRACER_URL);
    let cache_grpc_endpoint    = optional_env("CACHE_GRPC_ENDPOINT", DEFAULT_CACHE_GRPC_ENDPOINT);
    let compliance_url         = optional_env("COMPLIANCE_URL", DEFAULT_COMPLIANCE_URL);
    let dashboard_url          = optional_env("DASHBOARD_URL", "http://localhost:5173");
    let rate_limit_max: u32    = parse_env("RATE_LIMIT_MAX_REQUESTS", DEFAULT_RATE_LIMIT_MAX);
    let rate_limit_window: u32 = parse_env("RATE_LIMIT_WINDOW_SECS", DEFAULT_RATE_LIMIT_WINDOW);

    // ── Infrastructure clients ────────────────────────────────────────────────
    let redis_conn = redis::Client::open(redis_url)
        .expect("Failed to create Redis client")
        .get_multiplexed_tokio_connection()
        .await
        .expect("Failed to create Redis multiplexed connection");

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .expect("Failed to construct reqwest HTTP client");

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(50)
        .min_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to Postgres DB");

    // ── App state ─────────────────────────────────────────────────────────────
    let (telemetry_tx, mut telemetry_rx) = tokio::sync::mpsc::channel(5000);

    // ── gRPC Channel (L2 Cache) — built once, pooled, fail-open ──────────────
    let cache_grpc_client = match tonic::transport::Channel::from_shared(cache_grpc_endpoint.clone())
        .map(|endpoint| endpoint.connect_lazy())
    {
        Ok(channel) => {
            info!(endpoint = %cache_grpc_endpoint, "gRPC channel to L2 cache established (lazy)");
            CacheGrpcClient::new(channel)
        }
        Err(e) => {
            // Non-fatal: gateway can still serve requests; all cache calls will fail-open.
            tracing::warn!(error = %e, endpoint = %cache_grpc_endpoint, "Failed to build gRPC channel — cache will be bypassed");
            // Fall back to a dummy channel pointing at a no-op loopback.
            let fallback = tonic::transport::Channel::from_static("http://127.0.0.1:0").connect_lazy();
            CacheGrpcClient::new(fallback)
        }
    };

    let l1_cache = Arc::new(infrastructure::l1_cache::L1Cache::new(500 * 1024 * 1024)
        .expect("Failed to initialize Hybrid L1 Cache"));

    let state = Arc::new(AppState {
        http_client: http_client.clone(),
        cache_grpc_client,
        compliance_url,
        redis_conn,
        db_pool,
        rate_limit_max,
        rate_limit_window,
        clickhouse_url: clickhouse_url.clone(),
        tracer_url: tracer_url.clone(),
        dashboard_url,
        llm_api_base_url: None,
        telemetry_tx,
        l1_cache,
    });

    // ── Background Workers ────────────────────────────────────────────────────
    let clickhouse_url_clone = clickhouse_url;
    let http_client_clone = http_client;

    tokio::spawn(async move {
        let mut buffer: Vec<serde_json::Value> = Vec::with_capacity(1000);
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        
        // Ensure the first tick doesn't trigger immediately before buffering anything
        interval.tick().await;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !buffer.is_empty() {
                        let mut body = String::new();
                        for p in &buffer {
                            body.push_str(&p.to_string());
                            body.push('\n');
                        }

                        let _ = http_client_clone
                            .post(format!("{}/?query=INSERT INTO RedEye_telemetry.request_logs FORMAT JSONEachRow", clickhouse_url_clone))
                            .body(body)
                            .send()
                            .await;

                        buffer.clear();
                    }
                }
                msg = telemetry_rx.recv() => {
                    match msg {
                        Some(payload) => {
                            buffer.push(payload);
                            if buffer.len() >= 1000 {
                                let mut body = String::new();
                                for p in &buffer {
                                    body.push_str(&p.to_string());
                                    body.push('\n');
                                }

                                let _ = http_client_clone
                                    .post(format!("{}/?query=INSERT INTO RedEye_telemetry.request_logs FORMAT JSONEachRow", clickhouse_url_clone))
                                    .body(body)
                                    .send()
                                    .await;
                                
                                buffer.clear();
                                // Reset interval so we don't immediately tick and flush an empty buffer
                                interval.reset();
                            }
                        }
                        None => {
                            // Channel closed. Flush and exit.
                            if !buffer.is_empty() {
                                let mut body = String::new();
                                for p in &buffer {
                                    body.push_str(&p.to_string());
                                    body.push('\n');
                                }

                                let _ = http_client_clone
                                    .post(format!("{}/?query=INSERT INTO RedEye_telemetry.request_logs FORMAT JSONEachRow", clickhouse_url_clone))
                                    .body(body)
                                    .send()
                                    .await;
                            }
                            break;
                        }
                    }
                }
            }
        }
    });

    // ── Server ────────────────────────────────────────────────────────────────
    let app: Router = api::routes::create_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!("🚀 RedEye Gateway listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("Axum server encountered a fatal error");
}
