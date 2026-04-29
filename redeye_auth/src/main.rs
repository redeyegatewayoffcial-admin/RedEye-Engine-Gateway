use redeye_auth::api::router::create_router;
use redeye_auth::infrastructure::db::setup_db_pool;
use redeye_auth::AppState;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "redeye_auth=debug,axum=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting redeye_auth service on PORT 8084");

    // Strict Bootstrapping check: Enforce JWT configuration exists before accepting traffic
    std::env::var("JWT_SECRET")
        .expect("CRITICAL BOOTSTRAP ERROR: JWT_SECRET environment variable must be set");

    // Setup SQLx DB Pool
    let pool = setup_db_pool().await?;

    tracing::info!("Running SQLx database migrations");
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.set_ignore_missing(true);
    migrator.run(&pool).await.map_err(|e| {
        tracing::error!("Migration failed: {}", e);
        e
    })?;

    let state = AppState { db_pool: pool };

    let app = create_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8084));
    tracing::debug!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
