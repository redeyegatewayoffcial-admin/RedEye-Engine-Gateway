use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use redeye_compliance::api::middleware::geo_routing::SharedConfig;
use redeye_compliance::api::middleware::security::SecurityState;
use redeye_compliance::api::routes::{create_router, AppState};
use redeye_compliance::domain::models::CompliancePolicy;
use redeye_compliance::infrastructure::clickhouse::ClickHouseLogger;
use redeye_compliance::usecases::opa_client::OpaClient;
use redeye_compliance::usecases::pii_engine::PiiEngine;

/// Helper: Sets up the test environment by mocking external downstream services
/// required by the compliance engine (OPA for rules, Clickhouse for logs),
/// compiling the internal AppState router representation.
async fn setup_test_environment() -> (axum::Router, MockServer) {
    let mock_server = MockServer::start().await;

    // MOCK: OPA Server returning Authorized (True)
    Mock::given(method("POST"))
        .and(path("/v1/data/redeye/authz/allow"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "result": true })))
        .mount(&mock_server)
        .await;

    // MOCK: Clickhouse Telemetry
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Build the application shared state mapped against the Wiremock routing servers
    let config = Arc::new(SharedConfig {
        default_endpoint: "https://api.openai.com/v1".into(),
        eu_endpoint: "https://api.eu.openai.com/v1".into(),
        us_endpoint: "https://api.us.openai.com/v1".into(),
        in_endpoint: "https://api.in.redeye.ai/v1".into(),
    });

    let pii_engine = Arc::new(PiiEngine::new().unwrap());

    let opa = Arc::new(OpaClient::new(mock_server.uri()));
    let clickhouse = Arc::new(ClickHouseLogger::new(mock_server.uri()));

    let compliance_policy = Arc::new(CompliancePolicy {
        active_frameworks: vec!["GDPR".into()],
        enable_pii_redaction: true,
        target_entities: vec!["CREDIT_CARD".into(), "EMAIL".into()],
        fail_closed: true, // Forces fail closed for tests ensuring accurate Opa enforcement execution rules
    });

    let security_state = SecurityState {
        opa,
        compliance_policy,
        clickhouse,
    };

    let state = AppState {
        config,
        pii_engine,
        security_state,
    };

    (create_router(state), mock_server)
}

/// Black-Box E2E Test: PII Validation & Redaction Engine
/// This securely injects a JSON string acting as an external API packet, runs it against the
/// deep router extraction paths, and returns analyzing the payload to guarantee redactions
/// mapped back securely as masked tokens.
#[tokio::test]
async fn test_redact_deeply_nested_pii() {
    let (app, _mock) = setup_test_environment().await;

    let nested_payload = json!({
        "metadata": {
            "user": {
                "profile": {
                    "email": "secret.agent@mi6.gov.uk",
                    "payment": {
                        "card": "4111-2222-3333-4444"
                    }
                }
            }
        },
        "messages": [
            {
                "role": "user",
                "content": "Please charge my card 4111-2222-3333-4444 and email receipt to secret.agent@mi6.gov.uk"
            }
        ]
    });

    let request = Request::builder()
        .method("POST")
        .uri("/v1/compliance/redact")
        .header("content-type", "application/json")
        .body(Body::from(nested_payload.to_string()))
        .unwrap();

    // Tower ServiceExt::oneshot spins up the exact internal pipeline cleanly simulating integration dispatch events
    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute internal AXUM testing route.");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Redact router encountered network error during pipeline translation"
    );

    // Extract underlying packet details
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let sanitized_payload = resp_json["sanitized_payload"].to_string();

    // Validate Data is Gone from sanitized boundaries
    assert!(
        !sanitized_payload.contains("secret.agent@mi6.gov.uk"),
        "Security Escalation: Raw Email found dynamically executing through pipeline"
    );
    assert!(
        !sanitized_payload.contains("4111-2222-3333-4444"),
        "Security Escalation: Raw Credit card found dynamically executing through pipeline"
    );

    // Validate Mask Replacements
    assert!(
        sanitized_payload.contains("EMAIL_REDACTED"),
        "Failed mapping string values accurately missing EMAIL block tags"
    );
    assert!(
        sanitized_payload.contains("CREDIT_CARD_REDACTED"),
        "Failed mapping string values accurately missing CREDIT CARD block tags"
    );
}
