//! main.rs — RedEye Compliance Microservice entry point (Clean Architecture).
//!
//! Owns all Autonomous Compliance Engine features: PII redaction, Data Residency,
//! OPA policy enforcement, and Immutable Audit Logging.

use std::{net::SocketAddr, sync::Arc};
use axum::{routing::get, Router};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod domain;
mod api;
mod usecases;
mod infrastructure;

use api::middleware::geo_routing::SharedConfig;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    info!("Starting RedEye Compliance Microservice...");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Compliance engine runs on port 8083 to separate from Gateway (8080), Cache (8081), Tracer (8082)
    let addr = SocketAddr::from(([0, 0, 0, 0], 8083));
    info!("🛡️ RedEye Compliance listening on http://{}", addr);

    let config = Arc::new(SharedConfig {
        default_endpoint: "https://api.openai.com/v1".into(),
        eu_endpoint: "https://api.eu.openai.com/v1".into(),
        us_endpoint: "https://api.us.openai.com/v1".into(),
    });

    let pii_engine = Arc::new(crate::usecases::pii_engine::PiiEngine::new());
    
    let opa = Arc::new(crate::usecases::opa_client::OpaClient::new(
        "http://opa-server:8181".into() // Mock production OPA URL
    ));

    let clickhouse = Arc::new(crate::infrastructure::clickhouse::ClickHouseLogger::new(
        "http://user:password@clickhouse-server:8123".into() // Mock setup
    ));

    let compliance_policy = Arc::new(domain::models::CompliancePolicy {
        active_frameworks: vec!["GDPR".into(), "DPDP".into()],
        enable_pii_redaction: true,
        target_entities: vec!["CREDIT_CARD".into(), "SSN".into(), "EMAIL".into()],
        fail_closed: true, // Strict enterprise security
    });

    let security_state = api::middleware::security::SecurityState {
        opa,
        compliance_policy,
        clickhouse,
    };

    let state = api::routes::AppState { config, pii_engine, security_state };

    let app = Router::new()
        .route("/health", get(|| async { axum::Json(serde_json::json!({"status": "ok", "service": "redeye_compliance"})) }))
        .nest("/api", api::routes::create_router(state))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(addr).await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app).await
        .expect("Compliance server encountered a fatal error");
}
