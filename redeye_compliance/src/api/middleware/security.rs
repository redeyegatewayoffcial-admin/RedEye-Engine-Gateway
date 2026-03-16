//! api/middleware/security.rs — Phase 8 Step 4 Prompt Injection & Policy Execution
//!
//! Evaluates incoming prompts for malicious patterns and checks OPA rules.
//! Drops the request immediately with HTTP 403 on violation.

use std::sync::Arc;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use tracing::warn;
use serde_json::Value;

use crate::{
    domain::models::{ComplianceAuditRecord, CompliancePolicy, OpaInput, OpaRequestPayload, ResidencyRule},
    infrastructure::clickhouse::ClickHouseLogger,
    usecases::opa_client::OpaClient,
};

#[derive(Clone)]
pub struct SecurityState {
    pub opa: Arc<OpaClient>,
    pub compliance_policy: Arc<CompliancePolicy>,
    pub clickhouse: Arc<ClickHouseLogger>,
}

/// Helper to simulate Prompt Injection detection locally
fn contains_prompt_injection(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("ignore previous instructions") ||
    lower.contains("you are now acting as") ||
    lower.contains("jailbreak") ||
    lower.contains("system prompt bypass")
}

pub async fn security_guard_middleware(
    State(state): State<SecurityState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    
    // 1. Read request body to extract prompt and metadata
    let (parts, body) = request.into_parts();
    let bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let trace_id = parts.headers.get("x-trace-id").and_then(|h| h.to_str().ok()).unwrap_or("unknown").to_string();
    let tenant_id = parts.headers.get("x-tenant-id").and_then(|h| h.to_str().ok()).unwrap_or("unknown").to_string();
    let region = parts.headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()).unwrap_or("GLOBAL").to_string();
    
    let mut policy_result = true;
    let mut block_reason = None;

    if let Ok(payload) = serde_json::from_slice::<Value>(&bytes) {
        // A) Prompt Injection Detection
        if let Some(prompt) = payload.get("prompt").and_then(|v| v.as_str()) {
            if contains_prompt_injection(prompt) {
                warn!("🚨 PROMPT INJECTION BLOCKED. Pattern matched.");
                policy_result = false;
                block_reason = Some("Security Violation: Malicious prompt pattern detected.".to_string());
            }
        }

        // B) Extract metadata for OPA
        let model = payload.get("model").and_then(|v| v.as_str()).unwrap_or("unknown");

        // C) OPA Evaluation (if not already blocked)
        if policy_result {
            let opa_request = OpaRequestPayload {
                input: OpaInput {
                    trace_id: trace_id.clone(),
                    tenant_id: tenant_id.clone(),
                    user_region: region.clone(),
                    model_requested: model.to_string(),
                    active_frameworks: state.compliance_policy.active_frameworks.clone(),
                }
            };

            match state.opa.evaluate_policy(opa_request).await {
                Ok(result) => {
                    if !result.allow {
                        warn!("🚨 OPA POLICY VIOLATION BLOCKED: {:?}", result.block_reason);
                        policy_result = false;
                        block_reason = Some(result.block_reason.unwrap_or_else(|| "Blocked by organizational policy".to_string()));
                    }
                }
                Err(_e) => {
                    if state.compliance_policy.fail_closed {
                        warn!("🔒 OPA Unreachable and Fail-Closed is ACTIVE. Blocking.");
                        policy_result = false;
                        block_reason = Some("Security System Unavailable (Fail-Closed active)".to_string());
                    }
                }
            }
        }
    }

    // Capture the destination endpoint region (if it was set by earlier geo-routing or fallback)
    let dest_region = parts.extensions.get::<ResidencyRule>()
         .map(|r| r.region.clone())
         .unwrap_or_else(|| region.clone());

    // 4. FIRE AND FORGET AUDIT LOG
    // We send this to ClickHouse instantly without waiting using the ClickHouseLogger
    let timestamp = format!("{:?}", std::time::SystemTime::now()); // basic timestamp
    state.clickhouse.log_audit_event(ComplianceAuditRecord {
        trace_id: trace_id.clone(),
        tenant_id: tenant_id.clone(),
        timestamp,
        policy_result,
        redacted_entity_count: 0, // In full flow, this comes from the PII redaction pipeline downstream
        destination_region: dest_region,
        block_reason: block_reason.clone(),
    }).await;

    // 5. Terminate request if blocked
    if !policy_result {
        return Ok((StatusCode::FORBIDDEN, Json(serde_json::json!({
            "error": "Compliance or Security Violation",
            "reason": block_reason
        }))).into_response());
    }

    // Reconstruct the request to pass to the next handler
    let new_request = Request::from_parts(parts, axum::body::Body::from(bytes));
    Ok(next.run(new_request).await)
}
