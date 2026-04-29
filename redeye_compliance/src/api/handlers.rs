use crate::domain::models::ResidencyRule;
use crate::usecases::pii_engine::PiiEngine;
use axum::{extract::State, Json};
use std::sync::Arc;

/// Route LLM completion requests (Dummy for Step 2 Verification)
pub async fn check_routing(
    axum::extract::Extension(rule): axum::extract::Extension<ResidencyRule>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "routed",
        "region": rule.region,
        "endpoint": rule.regional_endpoint,
        "isolation": rule.strict_isolation
    }))
}

/// Applies PII redaction rules to a given JSON prompt payload.
pub async fn redact_prompt(
    State(engine): State<Arc<PiiEngine>>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let result = engine.redact_payload(payload).await;

    Json(serde_json::json!({
        "sanitized_payload": result.sanitized_payload,
        "mapping_stored": true, // Simulated mapping save
        "redacted_count": result.redacted_count,
        "token_map": result.token_map // Returned here for testing only
    }))
}
