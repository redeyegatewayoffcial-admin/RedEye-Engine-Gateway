use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;

/// Sets up a PostgreSQL connection pool using SQLx
pub async fn setup_db_pool() -> Result<PgPool, sqlx::Error> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");
    
    tracing::info!("Connecting to PostgreSQL database...");
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        // .connect_lazy(&database_url) could be used if DB is not ready
        .connect(&database_url)
        .await?;

    Ok(pool)
}
