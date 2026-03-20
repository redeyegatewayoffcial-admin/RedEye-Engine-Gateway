//! infrastructure/llm_router.rs — Universal LLM Provider Router
//!
//! Inspects the `model` field in the request body and routes to the
//! correct upstream base URL. All providers expose an OpenAI-compatible
//! `/chat/completions` endpoint, so only the base URL changes.
//!
//! Provider detection by model name prefix:
//!   • "gpt-*" or "o1-*" or "o3-*"  → OpenAI
//!   • "gemini-*"                    → Google Gemini (OpenAI-compat endpoint)
//!   • "llama*" / "mixtral*" / "whisper-*" → Groq
//!   • "claude-*"                    → Anthropic (OpenAI-compat endpoint)
//!   • anything else                 → OpenAI (safe default)

use reqwest::Client;
use serde_json::Value;
use tracing::{info, error};

use crate::domain::models::GatewayError;

// ── Provider base URLs ─────────────────────────────────────────────────────────
const OPENAI_BASE:    &str = "https://api.openai.com/v1";
const GEMINI_BASE:    &str = "https://generativelanguage.googleapis.com/v1beta/openai";
const GROQ_BASE:      &str = "https://api.groq.com/openai/v1";
const ANTHROPIC_BASE: &str = "https://api.anthropic.com/v1";  // OpenAI-compat (Anthropic Messages API mirror)

/// Detected LLM provider.
#[derive(Debug, Clone, PartialEq)]
pub enum LlmProvider {
    OpenAI,
    Gemini,
    Groq,
    Anthropic,
}

impl LlmProvider {
    /// Detect provider from model name string.
    pub fn detect(model: &str) -> Self {
        let m = model.to_lowercase();
        if m.starts_with("gemini") {
            LlmProvider::Gemini
        } else if m.starts_with("llama")
            || m.starts_with("mixtral")
            || m.starts_with("whisper")
            || m.starts_with("deepseek")
            || m.starts_with("qwen")
        {
            LlmProvider::Groq
        } else if m.starts_with("claude") {
            LlmProvider::Anthropic
        } else {
            // Default: OpenAI (covers gpt-*, o1-*, o3-*, etc.)
            LlmProvider::OpenAI
        }
    }

    /// Returns the base URL for this provider's chat completions endpoint.
    pub fn base_url(&self) -> &'static str {
        match self {
            LlmProvider::OpenAI    => OPENAI_BASE,
            LlmProvider::Gemini    => GEMINI_BASE,
            LlmProvider::Groq      => GROQ_BASE,
            LlmProvider::Anthropic => ANTHROPIC_BASE,
        }
    }

    /// Human-readable name for logging.
    pub fn name(&self) -> &'static str {
        match self {
            LlmProvider::OpenAI    => "OpenAI",
            LlmProvider::Gemini    => "Google Gemini",
            LlmProvider::Groq      => "Groq",
            LlmProvider::Anthropic => "Anthropic",
        }
    }
}

/// Routes and forwards a chat completion request to the correct LLM provider.
/// The `api_key` comes from the decrypted tenant key stored in `llm_routes`.
pub async fn route_chat_completion(
    client: &Client,
    api_key: &str,
    body: &Value,
    accept_header: &str,
) -> Result<reqwest::Response, GatewayError> {
    // Extract model from request body
    let model = body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-4o");

    let provider = LlmProvider::detect(model);
    let endpoint = format!("{}/chat/completions", provider.base_url());

    info!(
        provider = provider.name(),
        model = model,
        endpoint = %endpoint,
        "Routing request to LLM provider"
    );

    let response = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("Accept", accept_header)
        .json(body)
        .send()
        .await
        .map_err(|e| {
            error!(
                provider = provider.name(),
                error = %e,
                "Failed to reach upstream LLM provider"
            );
            GatewayError::UpstreamUnreachable(e)
        })?;

    Ok(response)
}

/// Extracts the model name from a request body, falling back to "gpt-4o".
pub fn extract_model(body: &Value) -> &str {
    body.get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-4o")
}
