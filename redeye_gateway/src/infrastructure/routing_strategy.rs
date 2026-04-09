//! infrastructure/routing_strategy.rs — Advanced LLM routing strategy engine.
//!
//! Provides two intelligent routing strategies beyond the default primary→fallback:
//!
//! - **Least Latency**: Reads EMA-smoothed P95 rankings from Redis (populated by
//!   `redeye_tracer`'s background worker) and routes to the fastest provider.
//! - **Cost Optimized**: Estimates input tokens via tiktoken BPE and routes to the
//!   cheapest provider using an embedded pricing catalog.

use std::collections::HashMap;
use std::sync::OnceLock;

use redis::AsyncCommands;
use serde_json::Value;
use tracing::{debug, warn};

use crate::infrastructure::llm_router::ProviderKey;

// ── Routing Strategy Enum ────────────────────────────────────────────────────

/// Determines how the gateway selects a provider for an incoming request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// Existing primary → fallback behavior (default).
    Default,
    /// Pick the provider with the lowest EMA P95 latency from Redis.
    LeastLatency,
    /// Pick the cheapest provider based on estimated input token cost.
    CostOptimized,
}

impl RoutingStrategy {
    /// Parses the `x-redeye-routing-strategy` header value.
    pub fn from_header(value: Option<&str>) -> Self {
        match value {
            Some("least_latency") => Self::LeastLatency,
            Some("cost_optimized") => Self::CostOptimized,
            _ => Self::Default,
        }
    }
}

// ── Pricing Catalog ──────────────────────────────────────────────────────────

/// Per-model pricing in USD per 1,000 tokens.
#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    pub cost_per_1k_input: f64,
    pub cost_per_1k_output: f64,
}

/// Returns the static pricing catalog, keyed by `"provider/model"` or a
/// provider-level default `"provider/*"`.
///
/// Prices sourced from public provider pricing pages (April 2026).
pub fn get_pricing_catalog() -> &'static HashMap<&'static str, ModelPricing> {
    static CATALOG: OnceLock<HashMap<&'static str, ModelPricing>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut m = HashMap::new();

        // ── OpenAI ───────────────────────────────────────────────────────
        m.insert("openai/gpt-4o", ModelPricing {
            cost_per_1k_input: 0.0025,
            cost_per_1k_output: 0.010,
        });
        m.insert("openai/gpt-4o-mini", ModelPricing {
            cost_per_1k_input: 0.00015,
            cost_per_1k_output: 0.0006,
        });
        m.insert("openai/o1-preview", ModelPricing {
            cost_per_1k_input: 0.015,
            cost_per_1k_output: 0.060,
        });
        m.insert("openai/o3-mini", ModelPricing {
            cost_per_1k_input: 0.0011,
            cost_per_1k_output: 0.0044,
        });
        // Provider-level default
        m.insert("openai/*", ModelPricing {
            cost_per_1k_input: 0.0025,
            cost_per_1k_output: 0.010,
        });

        // ── Anthropic ────────────────────────────────────────────────────
        m.insert("anthropic/claude-3-haiku-20240307", ModelPricing {
            cost_per_1k_input: 0.00025,
            cost_per_1k_output: 0.00125,
        });
        m.insert("anthropic/claude-3-sonnet", ModelPricing {
            cost_per_1k_input: 0.003,
            cost_per_1k_output: 0.015,
        });
        m.insert("anthropic/claude-3-opus", ModelPricing {
            cost_per_1k_input: 0.015,
            cost_per_1k_output: 0.075,
        });
        m.insert("anthropic/*", ModelPricing {
            cost_per_1k_input: 0.003,
            cost_per_1k_output: 0.015,
        });

        // ── Groq ─────────────────────────────────────────────────────────
        m.insert("groq/llama3-8b-8192", ModelPricing {
            cost_per_1k_input: 0.00005,
            cost_per_1k_output: 0.00008,
        });
        m.insert("groq/llama-3.3-70b-versatile", ModelPricing {
            cost_per_1k_input: 0.00059,
            cost_per_1k_output: 0.00079,
        });
        m.insert("groq/*", ModelPricing {
            cost_per_1k_input: 0.00027,
            cost_per_1k_output: 0.00027,
        });

        // ── Gemini ───────────────────────────────────────────────────────
        m.insert("gemini/gemini-1.5-flash", ModelPricing {
            cost_per_1k_input: 0.000075,
            cost_per_1k_output: 0.0003,
        });
        m.insert("gemini/gemini-pro", ModelPricing {
            cost_per_1k_input: 0.00025,
            cost_per_1k_output: 0.0005,
        });
        m.insert("gemini/*", ModelPricing {
            cost_per_1k_input: 0.000075,
            cost_per_1k_output: 0.0003,
        });

        // ── DeepSeek ─────────────────────────────────────────────────────
        m.insert("deepseek/deepseek-coder", ModelPricing {
            cost_per_1k_input: 0.00014,
            cost_per_1k_output: 0.00028,
        });
        m.insert("deepseek/*", ModelPricing {
            cost_per_1k_input: 0.00014,
            cost_per_1k_output: 0.00028,
        });

        // ── OpenRouter ───────────────────────────────────────────────────
        m.insert("openrouter/meta-llama/llama-3.3-70b-instruct", ModelPricing {
            cost_per_1k_input: 0.00039,
            cost_per_1k_output: 0.0004,
        });
        m.insert("openrouter/*", ModelPricing {
            cost_per_1k_input: 0.0005,
            cost_per_1k_output: 0.0005,
        });

        // ── Together ─────────────────────────────────────────────────────
        m.insert("together/*", ModelPricing {
            cost_per_1k_input: 0.0002,
            cost_per_1k_output: 0.0002,
        });

        // ── Mistral ──────────────────────────────────────────────────────
        m.insert("mistral/open-mistral-7b", ModelPricing {
            cost_per_1k_input: 0.00025,
            cost_per_1k_output: 0.00025,
        });
        m.insert("mistral/*", ModelPricing {
            cost_per_1k_input: 0.00025,
            cost_per_1k_output: 0.00025,
        });

        // ── xAI ──────────────────────────────────────────────────────────
        m.insert("xai/*", ModelPricing {
            cost_per_1k_input: 0.002,
            cost_per_1k_output: 0.010,
        });

        // ── Cerebras ─────────────────────────────────────────────────────
        m.insert("cerebras/*", ModelPricing {
            cost_per_1k_input: 0.0001,
            cost_per_1k_output: 0.0001,
        });

        // ── Fireworks ────────────────────────────────────────────────────
        m.insert("fireworks/*", ModelPricing {
            cost_per_1k_input: 0.0002,
            cost_per_1k_output: 0.0002,
        });

        // ── SiliconFlow ──────────────────────────────────────────────────
        m.insert("siliconflow/*", ModelPricing {
            cost_per_1k_input: 0.00015,
            cost_per_1k_output: 0.00015,
        });

        // ── Perplexity ───────────────────────────────────────────────────
        m.insert("perplexity/*", ModelPricing {
            cost_per_1k_input: 0.001,
            cost_per_1k_output: 0.001,
        });

        // ── Cohere ───────────────────────────────────────────────────────
        m.insert("cohere/*", ModelPricing {
            cost_per_1k_input: 0.0003,
            cost_per_1k_output: 0.0003,
        });

        m
    })
}

/// Looks up the pricing for a given `provider_id` and `model` name.
///
/// Resolution order:
/// 1. Exact match on `"provider/model"`
/// 2. Provider wildcard `"provider/*"`
/// 3. `None` if the provider is completely unknown
pub fn lookup_pricing(provider_id: &str, model: &str) -> Option<&'static ModelPricing> {
    let catalog = get_pricing_catalog();

    // 1. Exact match
    let exact_key = format!("{}/{}", provider_id, model);
    if let Some(p) = catalog.get(exact_key.as_str()) {
        return Some(p);
    }

    // 2. Provider wildcard
    let wildcard_key = format!("{}/*", provider_id);
    if let Some(p) = catalog.get(wildcard_key.as_str()) {
        return Some(p);
    }

    None
}

// ── Token Estimation ─────────────────────────────────────────────────────────

/// Characters-per-token fallback when tiktoken encoding is unavailable.
const CHARS_PER_TOKEN_FALLBACK: usize = 4;

/// Estimates the input token count of a chat completion request body.
///
/// Uses tiktoken's `cl100k_base` BPE encoding (covers GPT-4, GPT-4o, and
/// provides a reasonable approximation for Claude/Gemini). Falls back to
/// `len(chars) / 4` if the encoding fails.
pub fn estimate_input_tokens(body: &Value) -> usize {
    let text = extract_prompt_text(body);
    if text.is_empty() {
        return 0;
    }

    // Attempt BPE tokenization (blocking but sub-millisecond for typical prompts).
    match tiktoken_rs::cl100k_base() {
        Ok(bpe) => {
            let tokens = bpe.encode_with_special_tokens(&text);
            debug!(chars = text.len(), tokens = tokens.len(), "tiktoken BPE token estimate");
            tokens.len()
        }
        Err(_) => {
            let estimated = text.len() / CHARS_PER_TOKEN_FALLBACK + 1;
            debug!(chars = text.len(), estimated, "Fallback chars/4 token estimate");
            estimated
        }
    }
}

/// Extracts all prompt text from the request body for token counting.
/// Concatenates system prompt + all message contents.
fn extract_prompt_text(body: &Value) -> String {
    let mut text = String::new();

    // System prompt (if present as a top-level field)
    if let Some(system) = body.get("system").and_then(|v| v.as_str()) {
        text.push_str(system);
        text.push(' ');
    }

    // Messages array
    if let Some(messages) = body.get("messages").and_then(|v| v.as_array()) {
        for msg in messages {
            if let Some(content) = msg.get("content") {
                match content {
                    Value::String(s) => {
                        text.push_str(s);
                        text.push(' ');
                    }
                    Value::Array(parts) => {
                        // Multimodal content blocks
                        for part in parts {
                            if let Some(t) = part.get("text").and_then(|v| v.as_str()) {
                                text.push_str(t);
                                text.push(' ');
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    text
}

// ── Least Latency Resolver ───────────────────────────────────────────────────

/// Redis key for the latency-sorted provider ranking (written by redeye_tracer).
const LATENCY_REDIS_KEY: &str = "redeye:latency:rankings";

/// Reads the sorted provider ranking from Redis and returns the fastest
/// `provider_id` that the tenant also has a key for.
///
/// Returns `None` if Redis is empty, unreachable, or no ranked provider
/// matches the tenant's available keys.
pub async fn resolve_least_latency(
    redis_conn: &mut redis::aio::MultiplexedConnection,
    available_keys: &[ProviderKey],
) -> Option<String> {
    // ZRANGE returns members in ascending score order (lowest latency first).
    let ranked: Vec<String> = redis_conn
        .zrange(LATENCY_REDIS_KEY, 0, -1)
        .await
        .unwrap_or_else(|e| {
            warn!(error = %e, "Failed to read latency rankings from Redis");
            Vec::new()
        });

    if ranked.is_empty() {
        debug!("No latency rankings in Redis — falling back to default strategy");
        return None;
    }

    // Intersect with available keys: pick the first (fastest) that the tenant has.
    for provider_id in &ranked {
        if available_keys.iter().any(|k| k.provider_id == *provider_id) {
            debug!(provider = %provider_id, "Least-latency provider selected");
            return Some(provider_id.clone());
        }
    }

    debug!("No ranked provider matches tenant keys — falling back to default");
    None
}

// ── Cost Optimized Resolver ──────────────────────────────────────────────────

/// Picks the cheapest provider for the given model and estimated input token count.
///
/// For each provider the tenant has a key for, looks up the pricing catalog and
/// computes `estimated_tokens * cost_per_1k_input / 1000`. Returns the provider
/// with the lowest cost, plus the estimated cost in USD.
///
/// Returns `None` if no provider has pricing data (the caller should fall back
/// to the default strategy).
pub fn resolve_cost_optimized(
    model: &str,
    estimated_tokens: usize,
    available_keys: &[ProviderKey],
) -> Option<(String, f64)> {
    struct Candidate {
        provider_id: String,
        cost_usd: f64,
    }

    let mut candidates: Vec<Candidate> = Vec::new();
    let mut fallback_provider: Option<String> = None;

    for key in available_keys {
        if fallback_provider.is_none() {
            fallback_provider = Some(key.provider_id.clone());
        }

        if let Some(pricing) = lookup_pricing(&key.provider_id, model) {
            let cost = (estimated_tokens as f64) * pricing.cost_per_1k_input / 1000.0;
            candidates.push(Candidate {
                provider_id: key.provider_id.clone(),
                cost_usd: cost,
            });
        }
    }

    // Among priced candidates, pick the cheapest.
    if let Some(best) = candidates.iter().min_by(|a, b| {
        a.cost_usd.partial_cmp(&b.cost_usd).unwrap_or(std::cmp::Ordering::Equal)
    }) {
        debug!(
            provider = %best.provider_id,
            estimated_cost_usd = best.cost_usd,
            estimated_tokens,
            "Cost-optimized provider selected"
        );
        return Some((best.provider_id.clone(), best.cost_usd));
    }

    // No pricing found — return None so the caller uses default routing.
    debug!("No pricing data for model '{}' — falling back to default strategy", model);
    None
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── RoutingStrategy parsing ──────────────────────────────────────────

    #[test]
    fn test_routing_strategy_from_header() {
        assert_eq!(RoutingStrategy::from_header(None), RoutingStrategy::Default);
        assert_eq!(RoutingStrategy::from_header(Some("")), RoutingStrategy::Default);
        assert_eq!(RoutingStrategy::from_header(Some("garbage")), RoutingStrategy::Default);
        assert_eq!(RoutingStrategy::from_header(Some("least_latency")), RoutingStrategy::LeastLatency);
        assert_eq!(RoutingStrategy::from_header(Some("cost_optimized")), RoutingStrategy::CostOptimized);
    }

    // ── Token estimation ─────────────────────────────────────────────────

    #[test]
    fn test_estimate_input_tokens_basic() {
        let body = json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "user", "content": "Hello, how are you today?"}
            ]
        });
        let tokens = estimate_input_tokens(&body);
        // "Hello, how are you today?" should be ~ 7 BPE tokens.
        assert!(tokens > 0, "Token count should be positive");
        assert!(tokens < 50, "Token count should be reasonable: got {}", tokens);
    }

    #[test]
    fn test_estimate_input_tokens_empty() {
        let body = json!({"model": "gpt-4o"});
        let tokens = estimate_input_tokens(&body);
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_estimate_input_tokens_multimodal() {
        let body = json!({
            "model": "gpt-4o",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "What is in this image?"},
                    {"type": "image_url", "url": "https://example.com/img.jpg"}
                ]
            }]
        });
        let tokens = estimate_input_tokens(&body);
        assert!(tokens > 0, "Should count text blocks in multimodal content");
    }

    #[test]
    fn test_estimate_input_tokens_system_prompt() {
        let body = json!({
            "model": "gpt-4o",
            "system": "You are a helpful assistant.",
            "messages": [
                {"role": "user", "content": "Hi"}
            ]
        });
        let tokens = estimate_input_tokens(&body);
        assert!(tokens > 3, "Should include system prompt tokens: got {}", tokens);
    }

    // ── Pricing catalog ──────────────────────────────────────────────────

    #[test]
    fn test_lookup_pricing_exact_match() {
        let p = lookup_pricing("openai", "gpt-4o").expect("Should find exact match");
        assert!(p.cost_per_1k_input > 0.0);
        assert!(p.cost_per_1k_output > 0.0);
    }

    #[test]
    fn test_lookup_pricing_wildcard_fallback() {
        let p = lookup_pricing("openai", "some-future-model")
            .expect("Should fall back to openai/*");
        assert!(p.cost_per_1k_input > 0.0);
    }

    #[test]
    fn test_lookup_pricing_unknown_provider() {
        let p = lookup_pricing("totally_unknown_provider", "model-x");
        assert!(p.is_none());
    }

    // ── Cost resolver ────────────────────────────────────────────────────

    #[test]
    fn test_resolve_cost_optimized_selects_cheapest() {
        let keys = vec![
            ProviderKey { provider_id: "openai".to_string(), decrypted_key: "sk-a".to_string() },
            ProviderKey { provider_id: "groq".to_string(), decrypted_key: "sk-b".to_string() },
            ProviderKey { provider_id: "anthropic".to_string(), decrypted_key: "sk-c".to_string() },
        ];

        // For model "gpt-4o" (or wildcard), groq/* is cheapest.
        let (provider, cost) = resolve_cost_optimized("gpt-4o", 1000, &keys)
            .expect("Should select a provider");

        // Groq's wildcard = $0.00027/1K input → 1000 tokens = $0.00027
        assert_eq!(provider, "groq", "Groq should be cheapest");
        assert!(cost < 0.003, "Cost should be low: got {}", cost);
    }

    #[test]
    fn test_resolve_cost_optimized_no_keys() {
        let result = resolve_cost_optimized("gpt-4o", 1000, &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_cost_optimized_single_provider() {
        let keys = vec![
            ProviderKey { provider_id: "openai".to_string(), decrypted_key: "sk-a".to_string() },
        ];

        let (provider, _cost) = resolve_cost_optimized("gpt-4o", 500, &keys)
            .expect("Should select the only provider");
        assert_eq!(provider, "openai");
    }

    // ── Extract prompt text ──────────────────────────────────────────────

    #[test]
    fn test_extract_prompt_text_basic() {
        let body = json!({
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "What is Rust?"}
            ]
        });
        let text = extract_prompt_text(&body);
        assert!(text.contains("You are helpful."));
        assert!(text.contains("What is Rust?"));
    }
}
