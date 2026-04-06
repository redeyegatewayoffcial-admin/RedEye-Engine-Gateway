//! infrastructure/llm_router.rs — Dynamic Multi-LLM Router with Automated Fallback
//!
//! Inspects the `model` field in the request body and routes to the
//! correct upstream base URL using tenant-specific provider keys.
//!
//! Provider detection by model name prefix:
//!   • "gpt-*" or "o1-*" or "o3-*"  → OpenAI
//!   • "gemini-*"                    → Google Gemini (OpenAI-compat endpoint)
//!   • "llama*" / "mixtral*" / "whisper-*" → Groq
//!   • "claude-*"                    → Anthropic (OpenAI-compat endpoint)
//!   • anything else                 → OpenAI (safe default)

use std::sync::Arc;
use serde_json::Value;
use sqlx::Row;
use tracing::{info, error, warn};
use uuid::Uuid;

use crate::domain::models::{AppState, GatewayError};
use crate::api::middleware::auth::decrypt_api_key;

// ── Provider base URLs ─────────────────────────────────────────────────────────
const OPENAI_BASE:    &str = "https://api.openai.com/v1";
const GEMINI_BASE:    &str = "https://generativelanguage.googleapis.com/v1beta/openai";
const GROQ_BASE:      &str = "https://api.groq.com/openai/v1";
const ANTHROPIC_BASE: &str = "https://api.anthropic.com/v1";  // OpenAI-compat (Anthropic Messages API mirror)

/// Detected LLM provider.
#[derive(Debug, Clone, Copy, PartialEq)]
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
        if m.starts_with("gemini-") {
            LlmProvider::Gemini
        } else if m.starts_with("llama")
            || m.starts_with("mixtral")
        {
            LlmProvider::Groq
        } else if m.starts_with("claude-") {
            LlmProvider::Anthropic
        } else if m.starts_with("gpt-") || m.starts_with("o1-") {
            LlmProvider::OpenAI
        } else {
            // Default: OpenAI
            LlmProvider::OpenAI
        }
    }

    /// Returns the lowercase string identifier for the provider (matches DB).
    pub fn as_str(&self) -> &'static str {
        match self {
            LlmProvider::OpenAI => "openai",
            LlmProvider::Gemini => "gemini",
            LlmProvider::Groq => "groq",
            LlmProvider::Anthropic => "anthropic",
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

/// Provider key entry from database.
#[derive(Debug, Clone)]
pub struct ProviderKey {
    pub provider: LlmProvider,
    pub decrypted_key: String,
}

/// Fetches all provider keys for a tenant from the provider_keys table.
/// Decrypts each key using the AES_MASTER_KEY.
pub async fn fetch_tenant_provider_keys(
    state: &Arc<AppState>,
    tenant_id: &str,
) -> Result<Vec<ProviderKey>, GatewayError> {
    let tenant_uuid = Uuid::parse_str(tenant_id).map_err(|_| {
        GatewayError::ResponseBuild("Invalid tenant ID format".into())
    })?;

    let rows = sqlx::query(
        "SELECT provider_name, encrypted_key FROM provider_keys WHERE tenant_id = $1"
    )
    .bind(tenant_uuid)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Database error fetching provider keys");
        GatewayError::ResponseBuild("Failed to fetch provider keys".into())
    })?;

    let mut keys = Vec::with_capacity(rows.len());

    for row in rows {
        let provider_name: String = row.try_get("provider_name")
            .map_err(|_| GatewayError::ResponseBuild("Failed to fetch provider_name".into()))?;
        let encrypted_key: Vec<u8> = row.try_get("encrypted_key")
            .map_err(|_| GatewayError::ResponseBuild("Failed to fetch encrypted_key".into()))?;

        // Decrypt the key
        let decrypted_key = decrypt_api_key(&encrypted_key)
            .map_err(|_| GatewayError::ResponseBuild(
                format!("Failed to decrypt {} API key", provider_name)
            ))?;

        let provider = match provider_name.as_str() {
            "openai" => LlmProvider::OpenAI,
            "gemini" => LlmProvider::Gemini,
            "groq" => LlmProvider::Groq,
            "anthropic" => LlmProvider::Anthropic,
            _ => {
                warn!("Unknown provider in database: {}", provider_name);
                continue;
            }
        };

        keys.push(ProviderKey { provider, decrypted_key });
    }

    Ok(keys)
}

/// Gets the appropriate provider key for a given model.
/// Returns the key and the provider to use.
pub fn select_provider_for_model(
    model: &str,
    available_keys: &[ProviderKey],
) -> Option<(LlmProvider, String)> {
    let target_provider = LlmProvider::detect(model);

    // First, try to find the exact provider match
    for key in available_keys {
        if key.provider == target_provider {
            return Some((target_provider, key.decrypted_key.clone()));
        }
    }

    // If no exact match, try to find any available key as fallback
    // Priority: OpenAI -> Anthropic -> Groq -> Gemini
    let fallback_order = [
        LlmProvider::OpenAI,
        LlmProvider::Anthropic,
        LlmProvider::Groq,
        LlmProvider::Gemini,
    ];

    for fallback in &fallback_order {
        for key in available_keys {
            if key.provider == *fallback {
                return Some((*fallback, key.decrypted_key.clone()));
            }
        }
    }

    None
}

/// Checks if an HTTP status code indicates a retryable error.
fn is_retryable_error(status: u16) -> bool {
    matches!(status, 500 | 502 | 503 | 504)
}

/// Prepares request body for fallback provider.
/// Translates model name if necessary.
fn prepare_fallback_body(
    original_body: &Value,
    target_provider: LlmProvider,
    original_model: &str,
) -> Value {
    let mut body = original_body.clone();

    // Map model to appropriate fallback model
    let fallback_model = match target_provider {
        LlmProvider::OpenAI => "gpt-4o-mini",
        LlmProvider::Anthropic => "claude-3-haiku-20240307",
        LlmProvider::Groq => "llama3-8b-8192",
        LlmProvider::Gemini => "gemini-1.5-flash",
    };

    // If the original model is already compatible, keep it
    let target_provider_from_model = LlmProvider::detect(original_model);
    if target_provider_from_model == target_provider {
        // Keep original model as it's compatible
    } else {
        // Replace with fallback model
        body["model"] = Value::String(fallback_model.to_string());
    }

    body
}

/// Routes and forwards a chat completion request to the correct LLM provider.
/// Uses tenant-specific provider keys fetched from the database.
/// Implements automated fallback on upstream failures.
pub async fn route_chat_completion_with_fallback(
    state: &Arc<AppState>,
    tenant_id: &str,
    body: &Value,
    accept_header: &str,
) -> Result<reqwest::Response, GatewayError> {
    // Extract model from request body
    let model = extract_model(body);

    // Fetch all available provider keys for the tenant
    let available_keys = fetch_tenant_provider_keys(state, tenant_id).await?;

    if available_keys.is_empty() {
        return Err(GatewayError::ResponseBuild(
            "No provider keys configured for this tenant".into()
        ));
    }

    // Select the primary provider based on the requested model
    let (primary_provider, primary_key) = select_provider_for_model(model, &available_keys)
        .ok_or_else(|| GatewayError::ResponseBuild(
            "No compatible provider key available".into()
        ))?;

    info!(
        provider = primary_provider.name(),
        model = model,
        "Routing request to primary LLM provider"
    );

    // Attempt primary request
    let primary_result = execute_provider_request(
        &state.http_client,
        primary_provider,
        &primary_key,
        body,
        accept_header,
    ).await;

    // Check if primary succeeded
    match primary_result {
        Ok(response) => {
            let status = response.status().as_u16();
            if !is_retryable_error(status) {
                return Ok(response);
            }
            // Retryable error - continue to fallback
            warn!(
                provider = primary_provider.name(),
                status = status,
                "Primary provider failed with retryable error, attempting fallback"
            );
        }
        Err(e) => {
            warn!(
                provider = primary_provider.name(),
                error = %e,
                "Primary provider unreachable, attempting fallback"
            );
        }
    }

    // Try fallback providers
    try_fallback_providers(
        state,
        &available_keys,
        primary_provider,
        body,
        accept_header,
        model,
    ).await
}

/// Executes a request to a specific provider.
async fn execute_provider_request(
    client: &reqwest::Client,
    provider: LlmProvider,
    api_key: &str,
    body: &Value,
    accept_header: &str,
) -> Result<reqwest::Response, GatewayError> {
    let base = provider.base_url();
    let endpoint = format!("{}/chat/completions", base);

    let mut request = client.post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Accept", accept_header);

    // Inject Auth based on provider
    request = match provider {
        LlmProvider::OpenAI | LlmProvider::Groq => {
            request.header("Authorization", format!("Bearer {}", api_key))
        }
        LlmProvider::Gemini => {
            request.header("x-goog-api-key", api_key)
        }
        LlmProvider::Anthropic => {
            request
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
        }
    };

    let response = request
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

/// Attempts fallback to other available providers.
async fn try_fallback_providers(
    state: &Arc<AppState>,
    available_keys: &[ProviderKey],
    failed_provider: LlmProvider,
    body: &Value,
    accept_header: &str,
    original_model: &str,
) -> Result<reqwest::Response, GatewayError> {
    // Define fallback priority based on the failed provider
    let fallback_candidates: Vec<LlmProvider> = match failed_provider {
        LlmProvider::OpenAI => vec![LlmProvider::Anthropic, LlmProvider::Groq, LlmProvider::Gemini],
        LlmProvider::Anthropic => vec![LlmProvider::OpenAI, LlmProvider::Groq, LlmProvider::Gemini],
        LlmProvider::Groq => vec![LlmProvider::OpenAI, LlmProvider::Anthropic, LlmProvider::Gemini],
        LlmProvider::Gemini => vec![LlmProvider::OpenAI, LlmProvider::Anthropic, LlmProvider::Groq],
    };

    for fallback_provider in fallback_candidates {
        // Skip if same as failed provider
        if fallback_provider == failed_provider {
            continue;
        }

        // Find key for this fallback provider
        let Some(key_entry) = available_keys.iter().find(|k| k.provider == fallback_provider) else {
            continue;
        };

        info!(
            from_provider = failed_provider.name(),
            to_provider = fallback_provider.name(),
            "Attempting automated fallback"
        );

        // Prepare fallback body (may need model translation)
        let fallback_body = prepare_fallback_body(body, fallback_provider, original_model);

        match execute_provider_request(
            &state.http_client,
            fallback_provider,
            &key_entry.decrypted_key,
            &fallback_body,
            accept_header,
        ).await {
            Ok(response) => {
                let status = response.status().as_u16();
                if !is_retryable_error(status) {
                    info!(
                        provider = fallback_provider.name(),
                        "Fallback successful - request completed"
                    );
                    return Ok(response);
                }
                warn!(
                    provider = fallback_provider.name(),
                    status = status,
                    "Fallback provider also failed"
                );
            }
            Err(e) => {
                warn!(
                    provider = fallback_provider.name(),
                    error = %e,
                    "Fallback provider unreachable"
                );
            }
        }
    }

    // All fallbacks exhausted
    Err(GatewayError::ResponseBuild("All LLM providers failed".into()))
}

/// Extracts the model name from a request body, falling back to "gpt-4o".
pub fn extract_model(body: &Value) -> &str {
    body.get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-4o")
}

/// Routes and forwards a chat completion request to the correct LLM provider.
/// The `api_key` comes from the decrypted tenant key stored in `llm_routes`.
pub async fn route_chat_completion(
    client: &reqwest::Client,
    api_key: &str,
    body: &Value,
    accept_header: &str,
    base_url_override: Option<&str>,
) -> Result<reqwest::Response, GatewayError> {
    // Extract model from request body
    let model = extract_model(body);

    let provider = LlmProvider::detect(model);
    let base = base_url_override.unwrap_or_else(|| provider.base_url());
    let endpoint = format!("{}/chat/completions", base);

    info!(
        provider = provider.name(),
        model = model,
        endpoint = %endpoint,
        "Routing request to LLM provider"
    );

    let mut request = client.post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Accept", accept_header);

    // Inject Auth based on provider
    request = match provider {
        LlmProvider::OpenAI | LlmProvider::Groq => {
            request.header("Authorization", format!("Bearer {}", api_key))
        }
        LlmProvider::Gemini => {
            request.header("x-goog-api-key", api_key)
        }
        LlmProvider::Anthropic => {
            request
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
        }
    };

    let mut response = request
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

    // Agentic Hot-Swap Feature
    if provider == LlmProvider::OpenAI && (response.status().as_u16() == 503 || response.status().as_u16() == 429) {
        tracing::warn!("Primary provider failed. Triggering RedEye Hot-Swap...");

        let anthropic_key = std::env::var("ANTHROPIC_BACKUP_KEY").unwrap_or_else(|_| "mock_backup_key".to_string());
        
        // Convert Request
        use crate::infrastructure::translators::{OpenAIChatRequest, AnthropicRequest};
        use crate::domain::models::RedEyeConversation;

        let openai_req: OpenAIChatRequest = serde_json::from_value(body.clone())
            .map_err(|e| GatewayError::ResponseBuild(format!("Hot-Swap parse error: {}", e)))?;
        
        let conv: RedEyeConversation = openai_req.try_into()
            .map_err(|e| GatewayError::ResponseBuild(format!("Hot-Swap to internal struct failed: {}", e)))?;
        
        let anthropic_req: AnthropicRequest = conv.try_into()
            .map_err(|e| GatewayError::ResponseBuild(format!("Hot-Swap to Anthropic struct failed: {}", e)))?;

        let anthropic_base = std::env::var("ANTHROPIC_MOCK_URL").unwrap_or_else(|_| "https://api.anthropic.com".to_string());
        let anthropic_endpoint = format!("{}/v1/messages", anthropic_base);
        
        // Fire request to Anthropic
        let anthropic_resp = client.post(&anthropic_endpoint)
            .header("x-api-key", anthropic_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&anthropic_req)
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, "Anthropic fallback unreachable");
                GatewayError::UpstreamUnreachable(e)
            })?;
        
        // Translate Response
        let anth_json: Value = anthropic_resp.json().await.map_err(|_| {
            GatewayError::ResponseBuild("Failed to read Anthropic response JSON".into())
        })?;
        
        let mapped_tool_calls = anth_json["content"]
            .as_array()
            .cloned()
            .and_then(|arr| crate::infrastructure::schema_mapper::map_anthropic_tool_use_to_openai_calls(arr));

        let text_content = anth_json["content"]
            .as_array()
            .and_then(|arr| {
                arr.iter().find(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
            })
            .and_then(|c| c["text"].as_str())
            .unwrap_or("")
            .to_string();
            
        let input_tokens = anth_json["usage"]["input_tokens"].as_u64().unwrap_or(0);
        let output_tokens = anth_json["usage"]["output_tokens"].as_u64().unwrap_or(0);
        
        let finish_reason = if mapped_tool_calls.is_some() { "tool_calls" } else { "stop" };
        
        let mut message_obj = serde_json::json!({
            "role": "assistant",
            "content": text_content
        });
        
        if let Some(tool_calls) = mapped_tool_calls {
            message_obj["tool_calls"] = serde_json::json!(tool_calls);
        }
            
        let mock_openai_resp = serde_json::json!({
            "id": anth_json["id"].as_str().unwrap_or("chatcmpl-fallback"),
            "object": "chat.completion",
            "created": 0,
            "model": "claude-fallback",
            "choices": [{
                "index": 0,
                "message": message_obj,
                "finish_reason": finish_reason
            }],
            "usage": {
                "prompt_tokens": input_tokens,
                "completion_tokens": output_tokens,
                "total_tokens": input_tokens + output_tokens
            }
        });

        let body_bytes = serde_json::to_vec(&mock_openai_resp).unwrap();
        let hr = axum::http::Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .header("x-redeye-hot-swapped", "1")
            .header("x-redeye-executed-provider", "anthropic")
            .body(reqwest::Body::from(body_bytes))
            .unwrap();
        
        response = reqwest::Response::from(hr);
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_detection() {
        assert_eq!(LlmProvider::detect("gpt-4"), LlmProvider::OpenAI);
        assert_eq!(LlmProvider::detect("gpt-4o"), LlmProvider::OpenAI);
        assert_eq!(LlmProvider::detect("o1-preview"), LlmProvider::OpenAI);
        assert_eq!(LlmProvider::detect("claude-3-opus"), LlmProvider::Anthropic);
        assert_eq!(LlmProvider::detect("claude-3-sonnet"), LlmProvider::Anthropic);
        assert_eq!(LlmProvider::detect("gemini-pro"), LlmProvider::Gemini);
        assert_eq!(LlmProvider::detect("llama3-70b"), LlmProvider::Groq);
        assert_eq!(LlmProvider::detect("mixtral-8x7b"), LlmProvider::Groq);
    }

    #[test]
    fn test_is_retryable_error() {
        assert!(is_retryable_error(500));
        assert!(is_retryable_error(502));
        assert!(is_retryable_error(503));
        assert!(is_retryable_error(504));
        assert!(!is_retryable_error(400));
        assert!(!is_retryable_error(401));
        assert!(!is_retryable_error(429));
        assert!(!is_retryable_error(200));
    }

    #[test]
    fn test_select_provider_for_model() {
        let keys = vec![
            ProviderKey {
                provider: LlmProvider::OpenAI,
                decrypted_key: "sk-openai".to_string(),
            },
            ProviderKey {
                provider: LlmProvider::Anthropic,
                decrypted_key: "sk-anthropic".to_string(),
            },
        ];

        // Exact match
        let (provider, key) = select_provider_for_model("gpt-4", &keys).unwrap();
        assert_eq!(provider, LlmProvider::OpenAI);
        assert_eq!(key, "sk-openai");

        // Another exact match
        let (provider, key) = select_provider_for_model("claude-3", &keys).unwrap();
        assert_eq!(provider, LlmProvider::Anthropic);
        assert_eq!(key, "sk-anthropic");
    }

    #[test]
    fn test_select_provider_fallback_order() {
        // Only Anthropic available, but requesting GPT
        let keys = vec![
            ProviderKey {
                provider: LlmProvider::Anthropic,
                decrypted_key: "sk-anthropic".to_string(),
            },
        ];

        // Should fallback to Anthropic since no OpenAI key
        let (provider, key) = select_provider_for_model("gpt-4", &keys).unwrap();
        assert_eq!(provider, LlmProvider::Anthropic);
        assert_eq!(key, "sk-anthropic");
    }

    #[test]
    fn test_extract_model() {
        let body = serde_json::json!({"model": "gpt-4"});
        assert_eq!(extract_model(&body), "gpt-4");

        let body = serde_json::json!({});
        assert_eq!(extract_model(&body), "gpt-4o"); // default
    }

    #[test]
    fn test_prepare_fallback_body() {
        let body = serde_json::json!({"model": "gpt-4", "messages": []});
        
        // Fallback to Anthropic
        let fallback = prepare_fallback_body(&body, LlmProvider::Anthropic, "gpt-4");
        assert_eq!(fallback["model"], "claude-3-haiku-20240307");

        // Keep original if compatible
        let fallback = prepare_fallback_body(&body, LlmProvider::OpenAI, "gpt-4");
        assert_eq!(fallback["model"], "gpt-4");
    }
}
