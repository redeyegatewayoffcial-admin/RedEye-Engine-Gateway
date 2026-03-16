//! usecases/pii_engine.rs — Phase 8 Step 3 PII Redaction Pipeline.
//!
//! Intercepts LLM prompts, detects sensitive entities (mocked via Regex for this step,
//! but typically uses Microsoft Presidio gRPC), and replaces them with <TOKEN>.
//! Mappings are designed to be stored in Redis (mocked here as an in-memory or returned struct).

use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

pub struct RedactionResult {
    /// The sanitized JSON payload safe to send to the LLM
    pub sanitized_payload: Value,
    
    /// Map of Token -> Original Value (to be stored in Redis)
    pub token_map: HashMap<String, String>,
    
    /// Number of distinct entities redacted
    pub redacted_count: u32,
}

/// A simulated PII detection engine.
/// In production, this would make an RPC call to a Python Presidio sidecar.
pub struct PiiEngine {
    ssn_regex: Regex,
    cc_regex: Regex,
    email_regex: Regex,
}

impl PiiEngine {
    pub fn new() -> Self {
        Self {
            ssn_regex: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
            cc_regex: Regex::new(r"\b(?:\d[ -]*?){13,16}\b").unwrap(),
            email_regex: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,7}\b").unwrap(),
        }
    }

    /// Recursively traverses a JSON payload and redacts string values.
    pub async fn redact_payload(&self, mut payload: Value) -> RedactionResult {
        let mut token_map = HashMap::new();
        let mut redacted_count = 0;

        self.traverse_and_redact(&mut payload, &mut token_map, &mut redacted_count);

        // Here we would traditionally `SETEX auth_token redis_val 3600` via redis-rs async multiplexed client.
        
        RedactionResult {
            sanitized_payload: payload,
            token_map,
            redacted_count,
        }
    }

    fn traverse_and_redact(&self, value: &mut Value, token_map: &mut HashMap<String, String>, count: &mut u32) {
        match value {
            Value::String(text) => {
                let mut new_text = text.clone();
                new_text = self.redact_pattern(&new_text, &self.ssn_regex, "<SSN_REDACTED>", token_map, count);
                new_text = self.redact_pattern(&new_text, &self.cc_regex, "<CREDIT_CARD_REDACTED>", token_map, count);
                new_text = self.redact_pattern(&new_text, &self.email_regex, "<EMAIL_REDACTED>", token_map, count);
                
                *text = new_text;
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    self.traverse_and_redact(item, token_map, count);
                }
            }
            Value::Object(obj) => {
                for (_, val) in obj.iter_mut() {
                    self.traverse_and_redact(val, token_map, count);
                }
            }
            _ => {} // Numbers, booleans, nulls are ignored
        }
    }

    fn redact_pattern(
        &self, 
        text: &str, 
        re: &Regex, 
        token_prefix: &str, 
        token_map: &mut HashMap<String, String>,
        count: &mut u32
    ) -> String {
        re.replace_all(text, |caps: &regex::Captures| {
            let original = caps[0].to_string();
            let secure_token = format!("{}_{}", token_prefix, Uuid::new_v4().as_simple());
            token_map.insert(secure_token.clone(), original);
            *count += 1;
            secure_token
        }).to_string()
    }
}
