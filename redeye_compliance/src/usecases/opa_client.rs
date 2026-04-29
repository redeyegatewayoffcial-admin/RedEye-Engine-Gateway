//! usecases/opa_client.rs — Phase 8 Step 4 OPA Policy Engine.
//!
//! Rust client for evaluating requests against Open Policy Agent Rego policies
//! (GDPR, DPDP, HIPAA).

use reqwest::Client;
use std::time::Duration;
use tracing::error;

use crate::domain::models::{OpaRequestPayload, OpaResponsePayload, OpaResult};

pub struct OpaClient {
    http_client: Client,
    opa_url: String,
}

impl OpaClient {
    pub fn new(opa_url: String) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_millis(50)) // Ultra-low timeout requirement (<10ms target)
            .build()
            .expect("Failed to build OPA client");

        Self {
            http_client,
            opa_url,
        }
    }

    /// Evaluates the request against the OPA policy engine.
    /// Returns the OpaResult (allow/block decision).
    pub async fn evaluate_policy(&self, payload: OpaRequestPayload) -> Result<OpaResult, String> {
        let url = format!("{}/v1/data/redeye/compliance/allow", self.opa_url);

        match self.http_client.post(&url).json(&payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<OpaResponsePayload>().await {
                        Ok(opa_resp) => Ok(opa_resp.result),
                        Err(e) => {
                            error!("Failed to parse OPA response: {}", e);
                            Err("Invalid OPA response format".to_string())
                        }
                    }
                } else {
                    error!("OPA returned error status: {}", resp.status());
                    Err("OPA Server Error".to_string())
                }
            }
            Err(e) => {
                error!("OPA request failed (timeout/unreachable): {}", e);
                Err("OPA Engine Unreachable".to_string())
            }
        }
    }
}
