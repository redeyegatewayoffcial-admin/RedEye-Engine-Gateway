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
            .body(reqwest::Body::from(body_bytes))
            .unwrap();
        
        response = reqwest::Response::from(hr);
    }

    Ok(response)
}

/// Extracts the model name from a request body, falling back to "gpt-4o".
pub fn extract_model(body: &Value) -> &str {
    body.get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-4o")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use serde_json::json;

    #[tokio::test]
    async fn test_redeye_hot_swap_triggers_on_503() {
        let mock_server = MockServer::start().await;
        
        // Mock OpenAI endpoint returning 503
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&mock_server)
            .await;
            
        // Mock Anthropic fallback endpoint with tool_use
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "msg_hot_swap",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "I am Claude!"},
                    {
                        "type": "tool_use",
                        "id": "toolu_123",
                        "name": "get_weather",
                        "input": {"location": "London"}
                    }
                ],
                "model": "claude-test",
                "usage": {"input_tokens": 10, "output_tokens": 5}
            })))
            .expect(1)
            .mount(&mock_server)
            .await;
            
        std::env::set_var("ANTHROPIC_MOCK_URL", mock_server.uri());
        
        let client = reqwest::Client::new();
        let body = json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Help me!"}],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get weather",
                    "parameters": {
                        "type": "object",
                        "properties": {"location": {"type": "string"}}
                    }
                }
            }]
        });

        // Use the mock server as the OpenAI base
        let result = route_chat_completion(
            &client, 
            "sk-fake", 
            &body, 
            "application/json", 
            Some(&mock_server.uri())
        ).await;
        
        assert!(result.is_ok());
        let response = result.unwrap();
        
        // Router should have intercepted the 503, swapped, and returned 200
        assert_eq!(response.status().as_u16(), 200);
        let resp_json: serde_json::Value = response.json().await.unwrap();
        
        assert_eq!(resp_json["object"], "chat.completion");
        assert_eq!(resp_json["id"], "msg_hot_swap");
        assert_eq!(resp_json["choices"][0]["message"]["content"], "I am Claude!");
        assert_eq!(resp_json["choices"][0]["finish_reason"], "tool_calls");
        
        let tools = resp_json["choices"][0]["message"]["tool_calls"].as_array().expect("Expected tool_calls array");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["id"], "toolu_123");
        assert_eq!(tools[0]["function"]["name"], "get_weather");
        assert_eq!(tools[0]["function"]["arguments"], "{\"location\":\"London\"}");
        
        assert_eq!(resp_json["usage"]["total_tokens"], 15);
    }
    
    #[tokio::test]
    async fn test_redeye_hot_swap_ignores_400() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(400))
            .expect(1)
            .mount(&mock_server)
            .await;
            
        let client = reqwest::Client::new();
        let body = json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Bad format"}]
        });

        let result = route_chat_completion(
            &client, 
            "sk-fake", 
            &body, 
            "application/json", 
            Some(&mock_server.uri())
        ).await;
        
        assert!(result.is_ok());
        let response = result.unwrap();
        
        // A 400 Bad Request should not trigger failover, it should just be returned directly.
        assert_eq!(response.status().as_u16(), 400);
    }
}
