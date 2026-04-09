//! main.rs — RedEye Compliance Microservice entry point (Clean Architecture).
//!
//! Owns all Autonomous Compliance Engine features: PII redaction, Data Residency,
//! OPA policy enforcement, and Immutable Audit Logging.

use std::{net::SocketAddr, sync::Arc, env};
use axum::{routing::get, Router};
use tower_http::cors::{Any, CorsLayer};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod domain;
mod api;
mod usecases;
mod infrastructure;
mod error;

use api::middleware::geo_routing::SharedConfig;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    info!("Starting RedEye Compliance Microservice...");

    // CORS configuration - strict origin validation
    let cors = create_cors_layer();

    // Compliance engine runs on port 8083 to separate from Gateway (8080), Cache (8081), Tracer (8082)
    let addr = SocketAddr::from(([0, 0, 0, 0], 8083));
    info!("🛡️ RedEye Compliance listening on http://{}", addr);

    let config = Arc::new(SharedConfig {
        default_endpoint: "https://api.openai.com/v1".into(),
        eu_endpoint: "https://api.eu.openai.com/v1".into(),
        us_endpoint: "https://api.us.openai.com/v1".into(),
        in_endpoint: "https://api.in.redeye.ai/v1".into(),
    });

    let pii_engine = Arc::new(
        crate::usecases::pii_engine::PiiEngine::new()
            .expect("FATAL: PII engine failed to initialize — cannot start without compliance")
    );
    
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
        .layer(cors)
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024)); // 10MB limit

    let listener = tokio::net::TcpListener::bind(addr).await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app).await
        .expect("Compliance server encountered a fatal error");
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
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE, axum::http::Method::OPTIONS])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
}
