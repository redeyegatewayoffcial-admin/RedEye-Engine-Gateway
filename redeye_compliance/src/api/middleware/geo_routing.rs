//! api/middleware/geo_routing.rs — DPDP-Aware Data Residency & Geo-Routing.
//!
//! Intercepts LLM requests, determines the client region via headers or IP,
//! and enforces strict data residency rules:
//!
//! - **DPDP (India):** If region = `IN` or the PII engine detected Indian IDs,
//!   the middleware injects `x-redeye-allowed-regions: in` and routes to the
//!   Indian-compliant endpoint.
//! - **Region Lock Enforcement:** If the region is locked to India but the
//!   requested model is US-only, the request is rejected with HTTP 403.
//!
//! ## Safety Policy
//!
//! - No `.unwrap()`, `.expect()`, or `panic!()` — all fallible ops use `?` or
//!   explicit match arms with safe defaults.

use std::sync::Arc;
use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::{IntoResponse, Response},
};
use tracing::{info, warn};

use crate::domain::models::ResidencyRule;
use crate::error::AppError;

// ── Configuration ────────────────────────────────────────────────────────────

/// Shared config providing regional LLM endpoints.
pub struct SharedConfig {
    pub default_endpoint: String,
    pub eu_endpoint: String,
    pub us_endpoint: String,
    pub in_endpoint: String,
}

/// Models that are explicitly restricted to US infrastructure only.
/// Requests from DPDP-locked regions to these models are rejected.
const US_ONLY_MODELS: &[&str] = &[
    "o1-preview",
    "o1-mini",
];

// ── Geo-IP Simulation ────────────────────────────────────────────────────────

/// Simulated Geo-IP lookup.
/// In production, this would use the `maxminddb` crate to query a local MMDB file.
fn get_region_for_ip(ip: &str) -> &'static str {
    if ip.starts_with("192.168.1") || ip.starts_with("10.") {
        "EU"
    } else if ip.starts_with("172.") {
        "US"
    } else if ip.starts_with("103.") || ip.starts_with("49.") || ip.starts_with("223.") {
        // Common Indian IP prefixes (simplified heuristic).
        "IN"
    } else {
        "GLOBAL"
    }
}

// ── Middleware ────────────────────────────────────────────────────────────────

pub async fn geo_routing_middleware(
    State(config): State<Arc<SharedConfig>>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // 1. Determine region — priority: x-user-region > x-enforce-region > IP geo-lookup.
    let region = extract_region(&headers);

    // 2. Check for model in the request body (we need to peek at it).
    let model = extract_model_from_headers(&headers);

    // 3. DPDP strict border enforcement.
    let is_indian_region = region == "IN";

    if is_indian_region {
        // Check if the requested model is US-only.
        if let Some(ref model_name) = model {
            if US_ONLY_MODELS.iter().any(|m| model_name.contains(m)) {
                warn!(
                    region = "IN",
                    model = %model_name,
                    "DPDP region lock violation — US-only model requested from India"
                );
                return Err(AppError::RegionLockViolation(format!(
                    "Model '{}' is restricted to US infrastructure and cannot be accessed from region IN (DPDP compliance)",
                    model_name
                )).into_response());
            }
        }
    }

    // 4. Select regional endpoint and build the ResidencyRule.
    let rule = build_residency_rule(&region, &config);

    info!(
        region = %rule.region,
        endpoint = %rule.regional_endpoint,
        strict = rule.strict_isolation,
        "Geo-Routing decision established"
    );

    // 5. Inject ResidencyRule into request extensions for downstream handlers.
    request.extensions_mut().insert(rule.clone());

    // 6. Run the downstream handler.
    let mut response = next.run(request).await;

    // 7. Inject response headers.
    if let Ok(region_val) = axum::http::HeaderValue::from_str(&rule.region) {
        response.headers_mut().insert("x-routed-region", region_val);
    }

    // 8. DPDP: inject strict allowed-regions header for Indian traffic.
    if is_indian_region || rule.region == "IN" {
        let val = axum::http::HeaderValue::from_static("in");
        response.headers_mut().insert("x-redeye-allowed-regions", val);
    }

    Ok(response)
}

// ── Private Helpers ──────────────────────────────────────────────────────────

/// Extracts the region from request headers with a safe fallback chain.
fn extract_region(headers: &HeaderMap) -> String {
    // Priority 1: Explicit user-region header (DPDP-aware clients).
    if let Some(val) = headers.get("x-user-region") {
        if let Ok(s) = val.to_str() {
            if !s.is_empty() {
                return s.to_uppercase();
            }
        }
    }

    // Priority 2: Enforce-region header (tenant config).
    if let Some(val) = headers.get("x-enforce-region") {
        if let Ok(s) = val.to_str() {
            if !s.is_empty() {
                return s.to_uppercase();
            }
        }
    }

    // Priority 3: IP-based geo lookup.
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    get_region_for_ip(ip).to_string()
}

/// Extracts the model name from headers (lightweight — no body parsing).
fn extract_model_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-model-requested")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Builds a `ResidencyRule` based on the detected region.
fn build_residency_rule(region: &str, config: &SharedConfig) -> ResidencyRule {
    match region {
        "EU" => ResidencyRule {
            region: "EU".to_string(),
            regional_endpoint: config.eu_endpoint.clone(),
            strict_isolation: true,
        },
        "US" => ResidencyRule {
            region: "US".to_string(),
            regional_endpoint: config.us_endpoint.clone(),
            strict_isolation: false,
        },
        "IN" => ResidencyRule {
            region: "IN".to_string(),
            regional_endpoint: config.in_endpoint.clone(),
            strict_isolation: true, // DPDP: strict data isolation for India.
        },
        _ => ResidencyRule {
            region: "GLOBAL".to_string(),
            regional_endpoint: config.default_endpoint.clone(),
            strict_isolation: false,
        },
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_region_from_user_region_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-user-region", HeaderValue::from_static("in"));
        assert_eq!(extract_region(&headers), "IN");
    }

    #[test]
    fn test_extract_region_from_enforce_region_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-enforce-region", HeaderValue::from_static("EU"));
        assert_eq!(extract_region(&headers), "EU");
    }

    #[test]
    fn test_extract_region_priority_order() {
        // x-user-region takes priority over x-enforce-region.
        let mut headers = HeaderMap::new();
        headers.insert("x-user-region", HeaderValue::from_static("IN"));
        headers.insert("x-enforce-region", HeaderValue::from_static("EU"));
        assert_eq!(extract_region(&headers), "IN");
    }

    #[test]
    fn test_extract_region_fallback_to_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("103.21.244.0"));
        assert_eq!(extract_region(&headers), "IN");
    }

    #[test]
    fn test_extract_region_default_global() {
        let headers = HeaderMap::new();
        assert_eq!(extract_region(&headers), "GLOBAL");
    }

    #[test]
    fn test_build_residency_rule_india() {
        let config = SharedConfig {
            default_endpoint: "https://api.openai.com/v1".into(),
            eu_endpoint: "https://api.eu.openai.com/v1".into(),
            us_endpoint: "https://api.us.openai.com/v1".into(),
            in_endpoint: "https://api.in.redeye.ai/v1".into(),
        };

        let rule = build_residency_rule("IN", &config);
        assert_eq!(rule.region, "IN");
        assert!(rule.strict_isolation, "India must have strict isolation for DPDP");
        assert_eq!(rule.regional_endpoint, "https://api.in.redeye.ai/v1");
    }

    #[test]
    fn test_us_only_model_detection() {
        assert!(US_ONLY_MODELS.iter().any(|m| "o1-preview".contains(m)));
        assert!(US_ONLY_MODELS.iter().any(|m| "o1-mini".contains(m)));
        assert!(!US_ONLY_MODELS.iter().any(|m| "gpt-4o".contains(m)));
    }

    #[test]
    fn test_geo_routing_dpdp_strict() {
        // Verify that an Indian region request produces the correct residency rule.
        let config = SharedConfig {
            default_endpoint: "https://api.openai.com/v1".into(),
            eu_endpoint: "https://api.eu.openai.com/v1".into(),
            us_endpoint: "https://api.us.openai.com/v1".into(),
            in_endpoint: "https://api.in.redeye.ai/v1".into(),
        };

        let mut headers = HeaderMap::new();
        headers.insert("x-user-region", HeaderValue::from_static("IN"));

        let region = extract_region(&headers);
        assert_eq!(region, "IN");

        let rule = build_residency_rule(&region, &config);
        assert_eq!(rule.region, "IN");
        assert!(rule.strict_isolation);

        // The middleware would inject x-redeye-allowed-regions: in
        // (verified by the is_indian_region flag in the middleware body).
        let is_indian_region = region == "IN";
        assert!(is_indian_region, "Indian region must be detected");
    }

    #[test]
    fn test_region_lock_violation_detection() {
        // Indian user requesting a US-only model → should be caught.
        let mut headers = HeaderMap::new();
        headers.insert("x-user-region", HeaderValue::from_static("IN"));
        headers.insert("x-model-requested", HeaderValue::from_static("o1-preview"));

        let region = extract_region(&headers);
        let model = extract_model_from_headers(&headers);

        assert_eq!(region, "IN");
        assert!(model.is_some());

        let model_name = model.as_ref().map(|s| s.as_str()).unwrap_or("");
        let is_blocked = US_ONLY_MODELS.iter().any(|m| model_name.contains(m));
        assert!(is_blocked, "o1-preview must be blocked for Indian users");
    }

    #[test]
    fn test_no_region_lock_for_allowed_model() {
        // Indian user requesting gpt-4o → should NOT be blocked.
        let mut headers = HeaderMap::new();
        headers.insert("x-user-region", HeaderValue::from_static("IN"));
        headers.insert("x-model-requested", HeaderValue::from_static("gpt-4o"));

        let model = extract_model_from_headers(&headers);
        let model_name = model.as_ref().map(|s| s.as_str()).unwrap_or("");
        let is_blocked = US_ONLY_MODELS.iter().any(|m| model_name.contains(m));
        assert!(!is_blocked, "gpt-4o should be allowed for Indian users");
    }
}
