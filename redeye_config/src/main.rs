//! `redeye_config` service entry-point.
//!
//! Boot sequence:
//!   1. Load environment variables from `.env` (development) / container env (production).
//!   2. Initialise structured, async-aware tracing via `tracing-subscriber`.
//!   3. Open a SQLx Postgres connection pool and run pending migrations.
//!   4. Connect to Redis and construct the [`RedisSyncClient`].
//!   5. Wire dependencies into [`AppState`] and mount the Axum router.
//!   6. Bind the TCP listener and serve.
//!
//! Every fallible step returns a `Box<dyn std::error::Error>` to the Tokio
//! runtime; there are no `unwrap()` or `expect()` calls in this file.

use std::{net::SocketAddr, sync::Arc};

use redis::Client as RedisClient;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use redeye_config::{
    api::router::create_router,
    infrastructure::{
        db::{create_pool, PgConfigRepository},
        redis_sync::RedisSyncClient,
    },
    AppState,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 1. Environment variables ─────────────────────────────────────────────
    dotenvy::dotenv().ok(); // Silently ignores a missing .env in containers.

    // ── 2. Structured tracing ────────────────────────────────────────────────
    // The RUST_LOG environment variable controls verbosity levels per-module.
    // Default: info-level for this crate, warn for noisy dependencies.
    let env_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "redeye_config=info,tower_http=warn,sqlx=warn".into());

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(env_filter))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting redeye_config service…");

    // ── 3. Postgres pool + migrations ────────────────────────────────────────
    let pool = create_pool().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to connect to Postgres");
        e
    })?;

    tracing::info!("Running database migrations…");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database migration failed");
            e
        })?;
    tracing::info!("Migrations applied successfully");

    // ── 4. Redis client ──────────────────────────────────────────────────────
    let redis_url = std::env::var("REDIS_URL").map_err(|_| {
        // Surface a clear startup error rather than a cryptic connection failure.
        "REDIS_URL environment variable must be set (e.g. redis://127.0.0.1:6379)"
    })?;

    let redis_client = RedisClient::open(redis_url.as_str()).map_err(|e| {
        tracing::error!(error = %e, redis_url = %redis_url, "Failed to create Redis client");
        e
    })?;

    // Perform an eager ping to fail fast if Redis is unreachable at startup.
    {
        let mut conn = redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Startup Redis ping failed");
                e
            })?;
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Redis PING response error");
                e
            })?;
        tracing::info!("Redis connection healthy (PING OK)");
    }

    // ── 5. Dependency wiring ─────────────────────────────────────────────────
    let state = AppState {
        repo:  Arc::new(PgConfigRepository::new(pool)),
        redis: Arc::new(RedisSyncClient::new(redis_client)),
    };

    // ── 6. Router + TCP listener ─────────────────────────────────────────────
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8085);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
        tracing::error!(error = %e, %addr, "Failed to bind TCP listener");
        e
    })?;

    let app = create_router(state);

    tracing::info!(address = %addr, "redeye_config is listening");

    axum::serve(listener, app).await.map_err(|e| {
        tracing::error!(error = %e, "axum::serve exited with error");
        e
    })?;

    Ok(())
}
