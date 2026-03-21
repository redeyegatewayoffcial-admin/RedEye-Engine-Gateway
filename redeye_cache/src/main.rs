mod api;
mod domain;
mod infrastructure;
mod usecases;

use axum::{routing::post, Router};
use std::sync::Arc;
use tracing::info;

use crate::api::handlers::{lookup_handler, store_handler, ApiState};
use crate::infrastructure::{openai_client::OpenAiClient, postgres_repo::PostgresRepo};
use crate::usecases::semantic_search::SemanticSearchUseCase;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    info!("Starting RedEye Semantic Cache Microservice...");

    // 1. Initialize Infrastructure
    let pg_repo = Arc::new(PostgresRepo::new().await?);
    
    let openai_client = Arc::new(OpenAiClient::new()?);

    // 2. Initialize Use Cases
    let search_use_case = Arc::new(SemanticSearchUseCase::new(pg_repo, openai_client));

    // 3. Setup API State
    let app_state = ApiState { search_use_case };

    // 4. Build Router
    let app = Router::new()
        .route("/v1/cache/lookup", post(lookup_handler))
        .route("/v1/cache/store", post(store_handler))
        .with_state(app_state);

    // 5. Start Server (defaulting to 8081 for internal cache)
    let port = std::env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("Cache API listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
