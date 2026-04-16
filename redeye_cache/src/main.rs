//! main.rs — RedEye Cache Microservice entry point.
//!
//! Runs two servers concurrently:
//!   • REST/HTTP on `PORT`      (default 8081) — existing Axum handlers.
//!   • gRPC/H2  on `GRPC_PORT` (default 50051) — new tonic CacheService.

mod api;
mod domain;
mod infrastructure;
mod usecases;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{routing::post, Router};
use dotenvy::dotenv;
use tonic::transport::Server as GrpcServer;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::api::handlers::{lookup_handler, store_handler, ApiState};
use crate::api::grpc_server::CacheServiceImpl;
use crate::api::grpc_server::proto::cache_service_server::CacheServiceServer;
use crate::infrastructure::{openai_client::OpenAiClient, postgres_repo::PostgresRepo};
use crate::usecases::semantic_search::SemanticSearchUseCase;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env FIRST — before any env::var() call in infrastructure.
    dotenv().ok();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    info!("Starting RedEye Semantic Cache Microservice...");

    // ── Infrastructure ────────────────────────────────────────────────────────
    let pg_repo        = Arc::new(PostgresRepo::new().await?);
    let openai_client  = Arc::new(OpenAiClient::new()?);
    let search_use_case = Arc::new(SemanticSearchUseCase::new(pg_repo, openai_client));

    // ── REST server (port 8081) ───────────────────────────────────────────────
    let app_state = ApiState { search_use_case: search_use_case.clone() };
    let rest_app  = Router::new()
        .route("/v1/cache/lookup", post(lookup_handler))
        .route("/v1/cache/store",  post(store_handler))
        .with_state(app_state);

    let rest_port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8081);
    let rest_addr = SocketAddr::from(([0, 0, 0, 0], rest_port));

    let grpc_port: u16 = std::env::var("GRPC_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50051);
    let grpc_addr = SocketAddr::from(([0, 0, 0, 0], grpc_port));

    info!("REST API listening on http://{}", rest_addr);
    info!("gRPC API listening on http://{}", grpc_addr);

    // ── gRPC server (port 50051) ──────────────────────────────────────────────
    let grpc_service = CacheServiceServer::new(CacheServiceImpl::new(search_use_case));

    // Spawn both servers concurrently; either failing is fatal.
    tokio::select! {
        result = async {
            let listener = tokio::net::TcpListener::bind(rest_addr).await?;
            axum::serve(listener, rest_app).await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        } => {
            if let Err(e) = result {
                tracing::error!(error = %e, "REST server failed");
            }
        }
        result = GrpcServer::builder()
            .add_service(grpc_service)
            .serve(grpc_addr)
        => {
            if let Err(e) = result {
                tracing::error!(error = %e, "gRPC server failed");
            }
        }
    }

    Ok(())
}
