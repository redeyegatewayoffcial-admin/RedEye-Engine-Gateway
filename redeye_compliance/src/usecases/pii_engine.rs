//! usecases/pii_engine.rs — Two-Tier PII Detection & Redaction Engine.
//!
//! ## Architecture
//!
//! **Tier 1 (Fast Path):** Aho-Corasick automaton scans the string in O(n) time
//! for PII indicator keywords (e.g., `aadhaar`, `pan`, `ssn`, `@`). If no match
//! is found, the original text is returned immediately — **zero network latency**.
//!
//! **Tier 2 (Deep Path):** Only triggered when Tier 1 detects a potential hit.
//! Calls a `PresidioAnalyzer` backend (gRPC/mock) for entity-level extraction,
//! then redacts each confirmed entity with a unique secure token.
//!
//! ## Safety Policy
//!
//! - **No `.unwrap()` or `.expect()`** — all construction returns `Result`.
//! - **Fail-closed:** if the Presidio backend is unreachable, the engine returns
//!   `ComplianceError::PiiEngineFailure` and the caller must block the request.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use aho_corasick::AhoCorasick;
use regex::Regex;
use serde_json::Value;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::AppError;

// ── Public Result Types ──────────────────────────────────────────────────────

/// Outcome of a full PII scan + redaction pass.
pub struct RedactionResult {
    /// The sanitized JSON payload safe to send to the LLM.
    pub sanitized_payload: Value,

    /// Map of Token → Original Value (to be stored in Redis for de-tokenization).
    pub token_map: HashMap<String, String>,

    /// Number of distinct entities redacted.
    pub redacted_count: u32,

    /// Whether any Indian-specific PII (Aadhaar, PAN) was detected.
    pub indian_pii_detected: bool,
}

/// A single entity detected by the Presidio deep-scan backend.
#[derive(Debug, Clone)]
pub struct PresidioEntity {
    /// Entity type label (e.g. "AADHAAR", "CREDIT_CARD", "SSN", "EMAIL").
    pub entity_type: String,
    /// Start byte offset in the source text.
    pub start: usize,
    /// End byte offset in the source text.
    pub end: usize,
    /// Confidence score in [0.0, 1.0].
    pub score: f64,
}

// ── Presidio Analyzer Trait (mockable) ───────────────────────────────────────

/// Trait abstracting the Presidio gRPC backend so it can be mocked in tests.
///
/// Uses a manually-desugared async return type instead of `async_trait` to avoid
/// an external dependency. Implementors must be `Send + Sync`.
pub trait PresidioAnalyzer: Send + Sync {
    fn analyze<'a>(
        &'a self,
        text: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PresidioEntity>, AppError>> + Send + 'a>>;
}

/// Production Presidio gRPC client stub.
///
/// In a full deployment, this would hold a `tonic::Channel` and call
/// `presidio.analyze(text)`. For now it returns a mock entity list based
/// on regex matching, simulating the network round-trip.
pub struct PresidioGrpcClient {
    ssn_regex: Regex,
    cc_regex: Regex,
    email_regex: Regex,
    aadhaar_regex: Regex,
    pan_regex: Regex,
    ifsc_regex: Regex,
    bank_account_regex: Regex,
}

impl PresidioGrpcClient {
    /// Constructs the client. Returns `Err` if any regex pattern is invalid.
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {
            ssn_regex: compile_regex(r"\b\d{3}-\d{2}-\d{4}\b")?,
            cc_regex: compile_regex(r"\b(?:\d[ -]*?){13,16}\b")?,
            email_regex: compile_regex(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,7}\b")?,
            aadhaar_regex: compile_regex(r"\b\d{4}[ -]?\d{4}[ -]?\d{4}\b")?,
            pan_regex: compile_regex(r"\b[A-Z]{5}\d{4}[A-Z]\b")?,
            ifsc_regex: compile_regex(r"\b[A-Z]{4}0[A-Z0-9]{6}\b")?,
            bank_account_regex: compile_regex(r"\b\d{9,18}\b")?,
        })
    }
}

impl PresidioAnalyzer for PresidioGrpcClient {
    fn analyze<'a>(
        &'a self,
        text: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PresidioEntity>, AppError>> + Send + 'a>> {
        Box::pin(async move {
            let mut entities = Vec::new();

            collect_matches(&self.ssn_regex, text, "SSN", &mut entities);
            collect_matches(&self.cc_regex, text, "CREDIT_CARD", &mut entities);
            collect_matches(&self.email_regex, text, "EMAIL", &mut entities);
            collect_matches(&self.aadhaar_regex, text, "AADHAAR", &mut entities);
            collect_matches(&self.pan_regex, text, "PAN", &mut entities);
            collect_matches(&self.ifsc_regex, text, "IFSC", &mut entities);
            collect_matches(&self.bank_account_regex, text, "BANK_ACCOUNT", &mut entities);

            Ok(entities)
        })
    }
}

// ── PII Category (Tier 1 classification) ─────────────────────────────────────

/// Broad PII category detected by the Tier 1 keyword scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiiCategory {
    /// Indian identifiers (Aadhaar, PAN, IFSC).
    IndianId,
    /// Financial data (credit card, bank account).
    Financial,
    /// US identifiers (SSN).
    UsId,
    /// Contact information (email, phone).
    Contact,
}

// ── Two-Tier PII Engine ──────────────────────────────────────────────────────

/// The main PII detection engine. Constructed once at startup, shared via `Arc`.
pub struct PiiEngine {
    /// Tier 1: Aho-Corasick automaton for fast O(n) keyword detection.
    ac_automaton: AhoCorasick,

    /// Maps each Aho-Corasick pattern index to its PII category.
    pattern_categories: Vec<PiiCategory>,

    /// Tier 2: Presidio deep-scan backend.
    presidio: Arc<dyn PresidioAnalyzer>,
}

impl PiiEngine {
    /// Constructs the engine with the production Presidio gRPC client.
    ///
    /// Returns `Err(AppError::PiiEngineFailure)` if the Aho-Corasick automaton
    /// or internal regexes fail to compile — never panics.
    pub fn new() -> Result<Self, AppError> {
        let presidio = PresidioGrpcClient::new()?;
        Self::with_analyzer(Arc::new(presidio))
    }

    /// Constructs the engine with an injected `PresidioAnalyzer` (for testing).
    pub fn with_analyzer(presidio: Arc<dyn PresidioAnalyzer>) -> Result<Self, AppError> {
        // Keywords ordered to match pattern_categories below.
        let keywords: Vec<&str> = vec![
            // IndianId
            "aadhaar", "aadhar", "pan card", "pan number", "ifsc",
            // Financial
            "credit card", "debit card", "card number", "bank account", "account number",
            // UsId
            "ssn", "social security",
            // Contact
            "@",
        ];

        let categories = vec![
            // IndianId (5 patterns)
            PiiCategory::IndianId,
            PiiCategory::IndianId,
            PiiCategory::IndianId,
            PiiCategory::IndianId,
            PiiCategory::IndianId,
            // Financial (5 patterns)
            PiiCategory::Financial,
            PiiCategory::Financial,
            PiiCategory::Financial,
            PiiCategory::Financial,
            PiiCategory::Financial,
            // UsId (2 patterns)
            PiiCategory::UsId,
            PiiCategory::UsId,
            // Contact (1 pattern)
            PiiCategory::Contact,
        ];

        let ac = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .build(&keywords)
            .map_err(|e| AppError::PiiEngineFailure(format!("Aho-Corasick build failed: {}", e)))?;

        Ok(Self {
            ac_automaton: ac,
            pattern_categories: categories,
            presidio,
        })
    }

    /// Scans a JSON payload for PII using the two-tier architecture.
    ///
    /// 1. Extracts all string content from the JSON tree.
    /// 2. Runs Tier 1 (Aho-Corasick) — if no keywords found, returns immediately.
    /// 3. On Tier 1 hit, invokes Tier 2 (Presidio) for deep entity extraction.
    /// 4. Redacts confirmed entities and returns the sanitized payload.
    pub async fn scan(&self, payload: Value) -> Result<RedactionResult, AppError> {
        // Collect all string content for Tier 1 scanning.
        let full_text = extract_all_text(&payload);

        if full_text.is_empty() {
            return Ok(RedactionResult {
                sanitized_payload: payload,
                token_map: HashMap::new(),
                redacted_count: 0,
                indian_pii_detected: false,
            });
        }

        // ── Tier 1: Aho-Corasick fast keyword scan ──────────────────────────
        let tier1_hits: Vec<PiiCategory> = self
            .ac_automaton
            .find_iter(&full_text)
            .filter_map(|mat| self.pattern_categories.get(mat.pattern().as_usize()).copied())
            .collect();

        // Also check for long digit sequences (potential Aadhaar/CC/bank account).
        let has_long_digits = contains_digit_sequence(&full_text, 9);

        if tier1_hits.is_empty() && !has_long_digits {
            debug!("Tier 1: No PII indicators found — fast bypass");
            return Ok(RedactionResult {
                sanitized_payload: payload,
                token_map: HashMap::new(),
                redacted_count: 0,
                indian_pii_detected: false,
            });
        }

        let indian_pii_detected = tier1_hits.iter().any(|c| *c == PiiCategory::IndianId);

        info!(
            tier1_matches = tier1_hits.len(),
            has_long_digits,
            indian_pii = indian_pii_detected,
            "Tier 1: PII indicators detected — escalating to Tier 2"
        );

        // ── Tier 2: Presidio deep scan ──────────────────────────────────────
        let entities = self.presidio.analyze(&full_text).await?;

        if entities.is_empty() {
            debug!("Tier 2: Presidio found no confirmed entities — returning original");
            return Ok(RedactionResult {
                sanitized_payload: payload,
                token_map: HashMap::new(),
                redacted_count: 0,
                indian_pii_detected,
            });
        }

        info!(entity_count = entities.len(), "Tier 2: Redacting confirmed entities");

        // ── Redaction ───────────────────────────────────────────────────────
        let mut token_map = HashMap::new();
        let mut redacted_count: u32 = 0;
        let mut sanitized = payload;

        // Build a replacement map from the full_text entities.
        let mut replacements: Vec<(String, String)> = Vec::new();
        for entity in &entities {
            if entity.start < full_text.len() && entity.end <= full_text.len() {
                let original = &full_text[entity.start..entity.end];
                let token = format!(
                    "<{}_REDACTED>_{}",
                    entity.entity_type,
                    Uuid::new_v4().as_simple()
                );
                token_map.insert(token.clone(), original.to_string());
                replacements.push((original.to_string(), token));
                redacted_count += 1;
            }
        }

        // Check entity types for Indian PII from Tier 2 as well.
        let indian_from_tier2 = entities
            .iter()
            .any(|e| e.entity_type == "AADHAAR" || e.entity_type == "PAN" || e.entity_type == "IFSC");

        // Apply replacements to every string node in the JSON tree.
        apply_replacements(&mut sanitized, &replacements);

        Ok(RedactionResult {
            sanitized_payload: sanitized,
            token_map,
            redacted_count,
            indian_pii_detected: indian_pii_detected || indian_from_tier2,
        })
    }

    /// Legacy compatibility wrapper — delegates to `scan()`.
    pub async fn redact_payload(&self, payload: Value) -> RedactionResult {
        match self.scan(payload).await {
            Ok(result) => result,
            Err(e) => {
                warn!(error = %e, "PII scan failed — returning empty redaction (fail-closed should be enforced by caller)");
                RedactionResult {
                    sanitized_payload: Value::Null,
                    token_map: HashMap::new(),
                    redacted_count: 0,
                    indian_pii_detected: false,
                }
            }
        }
    }
}

// ── Private Helpers ──────────────────────────────────────────────────────────

/// Compiles a regex pattern, mapping errors to `AppError::PiiEngineFailure`.
fn compile_regex(pattern: &str) -> Result<Regex, AppError> {
    Regex::new(pattern).map_err(|e| {
        AppError::PiiEngineFailure(format!("Invalid PII regex '{}': {}", pattern, e))
    })
}

/// Collects regex matches into the entity list with a high default confidence.
fn collect_matches(re: &Regex, text: &str, entity_type: &str, out: &mut Vec<PresidioEntity>) {
    for mat in re.find_iter(text) {
        out.push(PresidioEntity {
            entity_type: entity_type.to_string(),
            start: mat.start(),
            end: mat.end(),
            score: 0.95,
        });
    }
}

/// Recursively extracts all string values from a JSON tree into a single buffer.
fn extract_all_text(value: &Value) -> String {
    let mut buf = String::new();
    collect_text(value, &mut buf);
    buf
}

fn collect_text(value: &Value, buf: &mut String) {
    match value {
        Value::String(s) => {
            buf.push_str(s);
            buf.push(' ');
        }
        Value::Array(arr) => {
            for item in arr {
                collect_text(item, buf);
            }
        }
        Value::Object(obj) => {
            for (_, val) in obj {
                collect_text(val, buf);
            }
        }
        _ => {}
    }
}

/// Checks whether the text contains a consecutive digit sequence of at least `min_len`.
fn contains_digit_sequence(text: &str, min_len: usize) -> bool {
    let mut count = 0usize;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            count += 1;
            if count >= min_len {
                return true;
            }
        } else if ch != ' ' && ch != '-' {
            count = 0;
        }
        // Spaces and dashes are allowed within digit sequences (e.g., "1234 5678 9012").
    }
    false
}

/// Applies text replacements to every string node in a JSON tree.
fn apply_replacements(value: &mut Value, replacements: &[(String, String)]) {
    match value {
        Value::String(text) => {
            for (original, token) in replacements {
                // Replace all occurrences of the original PII fragment.
                *text = text.replace(original, token);
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                apply_replacements(item, replacements);
            }
        }
        Value::Object(obj) => {
            for (_, val) in obj.iter_mut() {
                apply_replacements(val, replacements);
            }
        }
        _ => {}
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicBool, Ordering};

    // ── Mock Presidio that tracks whether it was called ──────────────────

    struct MockPresidio {
        was_called: Arc<AtomicBool>,
    }

    impl MockPresidio {
        fn new() -> (Self, Arc<AtomicBool>) {
            let flag = Arc::new(AtomicBool::new(false));
            (Self { was_called: flag.clone() }, flag)
        }
    }

    impl PresidioAnalyzer for MockPresidio {
        fn analyze<'a>(
            &'a self,
            text: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<PresidioEntity>, AppError>> + Send + 'a>> {
            self.was_called.store(true, Ordering::SeqCst);
            let text = text.to_string();
            Box::pin(async move {
                let mut entities = Vec::new();
                // Simple mock: detect 12-digit Aadhaar-like patterns.
                if let Some(pos) = text.find("123456789012") {
                    entities.push(PresidioEntity {
                        entity_type: "AADHAAR".to_string(),
                        start: pos,
                        end: pos + 12,
                        score: 0.98,
                    });
                }
                // Detect SSN pattern.
                if let Some(pos) = text.find("123-45-6789") {
                    entities.push(PresidioEntity {
                        entity_type: "SSN".to_string(),
                        start: pos,
                        end: pos + 11,
                        score: 0.99,
                    });
                }
                // Detect email.
                if let Some(pos) = text.find("test@example.com") {
                    entities.push(PresidioEntity {
                        entity_type: "EMAIL".to_string(),
                        start: pos,
                        end: pos + 16,
                        score: 0.97,
                    });
                }
                Ok(entities)
            })
        }
    }

    // ── Mock Presidio that always fails ──────────────────────────────────

    struct FailingPresidio;

    impl PresidioAnalyzer for FailingPresidio {
        fn analyze<'a>(
            &'a self,
            _text: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<PresidioEntity>, AppError>> + Send + 'a>> {
            Box::pin(async {
                Err(AppError::PiiEngineFailure(
                    "Presidio gRPC connection refused (mock)".to_string(),
                ))
            })
        }
    }

    // ── Tests ────────────────────────────────────────────────────────────

    #[test]
    fn test_pii_engine_new_no_unwrap() {
        // Verify that construction never panics and returns Ok.
        let result = PiiEngine::new();
        assert!(result.is_ok(), "PiiEngine::new() should not fail");
    }

    #[tokio::test]
    async fn test_pii_tier1_fast_bypass() {
        // Clean text with zero PII keywords → Presidio should NEVER be called.
        let (mock, was_called) = MockPresidio::new();
        let engine = PiiEngine::with_analyzer(Arc::new(mock))
            .expect("test setup");

        let payload = json!({
            "messages": [
                {"role": "user", "content": "What is the capital of France?"}
            ]
        });

        let result = engine.scan(payload.clone()).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.redacted_count, 0, "No PII should be redacted");
        assert!(!was_called.load(Ordering::SeqCst), "Presidio should NOT have been called for clean text");
        assert_eq!(result.sanitized_payload, payload, "Payload should be unmodified");
        assert!(!result.indian_pii_detected);
    }

    #[tokio::test]
    async fn test_pii_tier2_grpc_trigger() {
        // Text with "Aadhaar" keyword → Tier 1 triggers → Presidio IS called.
        let (mock, was_called) = MockPresidio::new();
        let engine = PiiEngine::with_analyzer(Arc::new(mock))
            .expect("test setup");

        let payload = json!({
            "messages": [
                {"role": "user", "content": "My Aadhaar number is 123456789012"}
            ]
        });

        let result = engine.scan(payload).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(was_called.load(Ordering::SeqCst), "Presidio MUST be called when Tier 1 detects keywords");
        assert!(result.redacted_count > 0, "Entities should be redacted");
        assert!(result.indian_pii_detected, "Should flag Indian PII");
    }

    #[tokio::test]
    async fn test_pii_tier2_deep_redaction() {
        // Full redaction with SSN + email — verify correct token map.
        let (mock, _) = MockPresidio::new();
        let engine = PiiEngine::with_analyzer(Arc::new(mock))
            .expect("test setup");

        let payload = json!({
            "messages": [
                {"role": "user", "content": "SSN is 123-45-6789 and email is test@example.com"}
            ]
        });

        let result = engine.scan(payload).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.redacted_count, 2, "Should redact SSN and EMAIL");
        assert_eq!(result.token_map.len(), 2, "Token map should have 2 entries");

        // Verify originals are stored in the token map.
        let originals: Vec<&String> = result.token_map.values().collect();
        assert!(originals.contains(&&"123-45-6789".to_string()), "SSN original must be in token map");
        assert!(originals.contains(&&"test@example.com".to_string()), "Email original must be in token map");

        // Verify sanitized payload does NOT contain originals.
        let sanitized_str = result.sanitized_payload.to_string();
        assert!(!sanitized_str.contains("123-45-6789"), "SSN must be redacted from output");
        assert!(!sanitized_str.contains("test@example.com"), "Email must be redacted from output");
    }

    #[tokio::test]
    async fn test_no_panic_on_grpc_failure() {
        // Presidio returns an error → engine must return ComplianceError, NOT panic.
        let engine = PiiEngine::with_analyzer(Arc::new(FailingPresidio))
            .expect("test setup");

        let payload = json!({
            "messages": [
                {"role": "user", "content": "My SSN is 123-45-6789, please help"}
            ]
        });

        let result = engine.scan(payload).await;
        assert!(result.is_err(), "Should return Err, not panic");

        match result {
            Err(AppError::PiiEngineFailure(msg)) => {
                assert!(
                    msg.contains("connection refused"),
                    "Error should describe the failure: got '{}'",
                    msg
                );
            }
            Err(other) => panic!("Expected PiiEngineFailure, got {:?}", other),
            Ok(_) => panic!("Should have failed"),
        }
    }

    #[test]
    fn test_contains_digit_sequence() {
        assert!(contains_digit_sequence("abc 123456789 def", 9));
        assert!(contains_digit_sequence("1234 5678 9012", 9)); // spaces within digits
        assert!(!contains_digit_sequence("abc 12345 def", 9));
        assert!(!contains_digit_sequence("no digits here", 9));
        assert!(contains_digit_sequence("my card is 4111-1111-1111-1111", 9));
    }

    #[test]
    fn test_extract_all_text() {
        let v = json!({
            "messages": [
                {"role": "user", "content": "hello world"},
                {"role": "system", "content": "you are helpful"}
            ]
        });
        let text = extract_all_text(&v);
        assert!(text.contains("hello world"));
        assert!(text.contains("you are helpful"));
    }
}
