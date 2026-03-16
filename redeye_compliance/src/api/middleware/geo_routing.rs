//! api/middleware/geo_routing.rs — Phase 8 Step 2 Data Residency.
//!
//! Intercepts LLM requests, determines client region via IP address (mocked MaxMind),
//! and injects a `ResidencyRule` stating the appropriate regional LLM endpoint.

use std::sync::Arc;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::info;

use crate::domain::models::ResidencyRule;

/// Shared state dummy for middleware extraction (to be expanded in main.rs)
pub struct SharedConfig {
    pub default_endpoint: String,
    pub eu_endpoint: String,
    pub us_endpoint: String,
}

/// Simulated Geo-IP lookup.
/// In production, this would use the `maxminddb` crate to query a local MMDB file.
fn get_region_for_ip(ip: &str) -> &'static str {
    if ip.starts_with("192.168.1") || ip.starts_with("10.") {
        "EU"
    } else if ip.starts_with("172.") {
        "US"
    } else {
        "GLOBAL"
    }
}

pub async fn geo_routing_middleware(
    State(config): State<Arc<SharedConfig>>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Extract IP address or Region Headers
    // Prioritize explicit tenant routing header over IP
    let region = if let Some(r) = headers.get("x-enforce-region") {
        r.to_str().unwrap_or("GLOBAL").to_uppercase()
    } else {
        let ip = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");
        
        get_region_for_ip(ip).to_string()
    };

    // 2. Select Regional Endpoint using data models
    let rule = match region.as_str() {
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
        _ => ResidencyRule {
            region: "GLOBAL".to_string(),
            regional_endpoint: config.default_endpoint.clone(),
            strict_isolation: false,
        },
    };

    info!(region = %rule.region, endpoint = %rule.regional_endpoint, "Geo-Routing decision established");

    // 3. Inject ResidencyRule into Request Extensions
    // Handlers (or the gateway if implemented remotely) can now read `extensions.get::<ResidencyRule>()`
    // to know exactly where to send the internal HTTP request.
    request.extensions_mut().insert(rule.clone());

    // 4. Also inject headers for downstream/responses
    let mut response = next.run(request).await;
    
    if let Ok(region_val) = axum::http::HeaderValue::from_str(&rule.region) {
        response.headers_mut().insert("x-routed-region", region_val);
    }

    Ok(response)
}
