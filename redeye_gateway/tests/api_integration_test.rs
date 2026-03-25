//! Integration tests for the redeye_gateway Axum router & middleware pipeline.
//!
//! These tests exercise the **real** Router + middleware stack returned by
//! `create_router`, dispatching HTTP requests in-process with `tower::ServiceExt::oneshot`.
//! No TCP port is bound — tests are fast, deterministic, and parallelisable.
//!
//! ## Prerequisites
//! A running Redis and Postgres instance reachable at the URLs configured in
//! the project `.env` file.  Start them with `docker compose up -d redis db`.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt; // Provides `.oneshot()`

use redeye_gateway::api::routes::create_router;
use redeye_gateway::domain::models::AppState;

/// Helper: bootstraps a real `AppState` from the local `.env` config.
///
/// This connects to the same Redis and Postgres instances used in development.
/// If the infra is down, the test will fail with a clear connection error —
/// not a false-positive assertion failure.
async fn build_test_state() -> Arc<AppState> {
    // Load .env from the workspace root (two levels up from tests/).
    dotenvy::from_filename("../../.env").ok();
    dotenvy::dotenv().ok();

    let redis_url = std::env::var("REDIS_URL")
        .expect("REDIS_URL must be set in .env for integration tests");
    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env for integration tests");
    let clickhouse_url = std::env::var("CLICKHOUSE_URL")
        .unwrap_or_else(|_| "http://localhost:8123".to_string());
    let dashboard_url= std::env::var("DASAHBOARD_URL");
        .expect("")

    let redis_conn = redis::Client::open(redis_url)
        .expect("Failed to create Redis client")
        .get_multiplexed_tokio_connection()
        .await
        .expect("Failed to connect to Redis — is it running?");

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("Failed to build reqwest client");

    let db_pool = sqlx::PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to Postgres — is it running?");

    let (telemetry_tx, _telemetry_rx) = tokio::sync::mpsc::channel(100);

    Arc::new(AppState {
        http_client,
        cache_url: "http://localhost:8081".to_string(),
        compliance_url: "http://localhost:8083".to_string(),
        redis_conn,
        db_pool,
        rate_limit_max: 60,
        rate_limit_window: 60,
        clickhouse_url,
        tracer_url: "http://localhost:8082".to_string(),
        telemetry_tx,
    })
}

/// Proves that the auth middleware correctly rejects requests that carry
/// no `Authorization` header.  The router + middleware stack should return
/// **401 Unauthorized** before the handler or any LLM call is reached.
///
/// This validates:
/// 1. `create_router` wires middleware in the correct order.
/// 2. `auth_middleware` runs on `/v1/chat/completions`.
/// 3. Missing tokens produce a 401 (not 500 or 404).
#[tokio::test]
async fn test_unauthorized_request_is_blocked() {
    let state = build_test_state().await;
    let app = create_router(state);

    // Build a POST /v1/chat/completions request with NO Authorization header.
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "model": "llama-3.3-70b-versatile",
                "messages": [{"role": "user", "content": "Hi"}]
            })
            .to_string(),
        ))
        .expect("Failed to build test request");

    // Dispatch in-process — no TCP socket, no port binding.
    let response = app.oneshot(request).await.expect("Router returned an error");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Expected 401 UNAUTHORIZED for a request with no auth token, got {}",
        response.status()
    );
}

/// Health check endpoint must be publicly accessible (no auth required).
#[tokio::test]
async fn test_health_check_is_public() {
    let state = build_test_state().await;
    let app = create_router(state);

    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .expect("Failed to build test request");

    let response = app.oneshot(request).await.expect("Router returned an error");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Expected 200 OK for /health, got {}",
        response.status()
    );
}
