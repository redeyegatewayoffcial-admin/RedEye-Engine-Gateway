//! infrastructure/llm_router.rs — Dynamic Multi-LLM Router with Automated Fallback
//!
//! Supports three routing strategies:
//! - **Default**: Primary provider detection → fallback chain
//! - **Least Latency**: Redis-backed EMA P95 ranking → fallback chain
//! - **Cost Optimized**: tiktoken token estimation → cheapest provider → fallback chain

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use serde_json::Value;
use sqlx::Row;
use tracing::{info, error, warn};
use uuid::Uuid;

use crate::domain::models::{AppState, GatewayError};
use crate::api::middleware::auth::decrypt_api_key;
use crate::infrastructure::routing_strategy::{self, RoutingStrategy};

// ── Dynamic Config Structs ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AuthScheme {
    Bearer,
    XApiKey,
    GoogleApiKey,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SchemaFormat {
    OpenAI,
    Anthropic,
    Gemini,
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub id: &'static str,
    pub base_url: &'static str,
    pub auth_scheme: AuthScheme,
    pub schema_format: SchemaFormat,
}

// ── Global Provider Registry ──────────────────────────────────────────────────

pub fn get_provider_registry() -> &'static HashMap<&'static str, ProviderConfig> {
    static PROVIDER_REGISTRY: OnceLock<HashMap<&'static str, ProviderConfig>> = OnceLock::new();
    PROVIDER_REGISTRY.get_or_init(|| {
        let mut m = HashMap::new();
        let configs = vec![
            ("openai", "https://api.openai.com/v1", AuthScheme::Bearer, SchemaFormat::OpenAI),
            ("anthropic", "https://api.anthropic.com/v1", AuthScheme::XApiKey, SchemaFormat::Anthropic),
            ("gemini", "https://generativelanguage.googleapis.com/v1beta/models", AuthScheme::GoogleApiKey, SchemaFormat::Gemini),
            ("groq", "https://api.groq.com/openai/v1", AuthScheme::Bearer, SchemaFormat::OpenAI),
            ("openrouter", "https://openrouter.ai/api/v1", AuthScheme::Bearer, SchemaFormat::OpenAI),
            ("deepseek", "https://api.deepseek.com/v1", AuthScheme::Bearer, SchemaFormat::OpenAI),
            ("together", "https://api.together.xyz/v1", AuthScheme::Bearer, SchemaFormat::OpenAI),
            ("mistral", "https://api.mistral.ai/v1", AuthScheme::Bearer, SchemaFormat::OpenAI),
            ("xai", "https://api.x.ai/v1", AuthScheme::Bearer, SchemaFormat::OpenAI),
            ("cerebras", "https://api.cerebras.ai/v1", AuthScheme::Bearer, SchemaFormat::OpenAI),
            ("fireworks", "https://api.fireworks.ai/inference/v1", AuthScheme::Bearer, SchemaFormat::OpenAI),
            ("siliconflow", "https://api.siliconflow.cn/v1", AuthScheme::Bearer, SchemaFormat::OpenAI),
            ("perplexity", "https://api.perplexity.ai", AuthScheme::Bearer, SchemaFormat::OpenAI),
            ("cohere", "https://api.cohere.ai/v1", AuthScheme::Bearer, SchemaFormat::OpenAI),
        ];

        for (id, url, auth, schema) in configs {
            m.insert(id, ProviderConfig {
                id,
                base_url: url,
                auth_scheme: auth,
                schema_format: schema,
            });
        }
        m
    })
}

pub fn get_provider_config(provider_name: &str) -> Option<ProviderConfig> {
    get_provider_registry().get(provider_name).cloned()
}

/// Provider key entry from database.
#[derive(Debug, Clone)]
pub struct ProviderKey {
    pub provider_id: String,
    pub decrypted_key: String,
}

/// Fetches all provider keys for a tenant from the provider_keys table.
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

    let registry = get_provider_registry();

    for row in rows {
        let provider_name: String = row.try_get("provider_name")
            .map_err(|_| GatewayError::ResponseBuild("Failed to fetch provider_name".into()))?;
        let encrypted_key: Vec<u8> = row.try_get("encrypted_key")
            .map_err(|_| GatewayError::ResponseBuild("Failed to fetch encrypted_key".into()))?;

        let decrypted_key = decrypt_api_key(&encrypted_key)
            .map_err(|_| GatewayError::ResponseBuild(
                format!("Failed to decrypt {} API key", provider_name)
            ))?;

        if registry.contains_key(provider_name.as_str()) {
            keys.push(ProviderKey { provider_id: provider_name, decrypted_key });
        } else {
            warn!("Unknown provider in database: {}", provider_name);
        }
    }

    Ok(keys)
}

/// Detect provider ID from model name string (helper for generic routing).
pub fn detect_primary_provider(model: &str) -> &'static str {
    let m = model.to_lowercase();
    if m.starts_with("gemini-") { "gemini" }
    else if m.starts_with("claude-") { "anthropic" }
    else if m.starts_with("gpt-") || m.starts_with("o1-") || m.starts_with("o3-") { "openai" }
    else if m.starts_with("mistral") { "mistral" }
    else if m.starts_with("deepseek") { "deepseek" }
    else if m.starts_with("cohere") { "cohere" }
    else if m.contains("llama") || m.contains("mixtral") || m.contains("qwen") { "openrouter" } // Default preferred open-source router
    else { "openai" } // Safe default
}

/// Helper method used by proxy.rs
pub fn detect_provider(model: &str) -> &'static str {
    detect_primary_provider(model)
}

/// Gets the appropriate provider key for a given model.
pub fn select_provider_for_model(
    model: &str,
    available_keys: &[ProviderKey],
) -> Option<(String, String)> {
    let target_provider = detect_primary_provider(model);

    // First, try exact map
    for key in available_keys {
        if key.provider_id == target_provider {
            return Some((key.provider_id.clone(), key.decrypted_key.clone()));
        }
    }

    // Smart fallback logic matching user instruction priority
    let fallback_order = vec![
        "openrouter",
        "together",
        "groq",
        "anthropic",
        "openai",
        "gemini",
        "mistral",
        "deepseek",
        "xai",
    ];

    for fallback in &fallback_order {
        for key in available_keys {
            if key.provider_id == *fallback {
                return Some((key.provider_id.clone(), key.decrypted_key.clone()));
            }
        }
    }
    
    // Pick first available
    if let Some(key) = available_keys.first() {
        return Some((key.provider_id.clone(), key.decrypted_key.clone()));
    }

    None
}

fn is_retryable_error(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

fn prepare_fallback_body(
    original_body: &Value,
    target_provider_id: &str,
    original_model: &str,
) -> Value {
    let mut body = original_body.clone();

    // Mapping fallback models for target providers
    let fallback_model = match target_provider_id {
        "openai" => "gpt-4o-mini",
        "anthropic" => "claude-3-haiku-20240307",
        "groq" => "llama3-8b-8192",
        "gemini" => "gemini-1.5-flash",
        "mistral" => "open-mistral-7b",
        "deepseek" => "deepseek-coder",
        _ => "meta-llama/Llama-3-8b-chat-hf", // GenAI open source fallback
    };

    let target_provider_from_model = detect_primary_provider(original_model);
    if target_provider_from_model == target_provider_id {
        // Keep original
    } else {
        body["model"] = Value::String(fallback_model.to_string());
    }

    body
}

pub async fn route_chat_completion_with_fallback(
    state: &Arc<AppState>,
    tenant_id: &str,
    body: &Value,
    accept_header: &str,
    strategy: RoutingStrategy,
) -> Result<reqwest::Response, GatewayError> {
    let model = extract_model(body);
    let available_keys = fetch_tenant_provider_keys(state, tenant_id).await?;

    // In tests, llm_api_base_url is set to the wiremock server URI to intercept all calls.
    let base_url_override: Option<&str> = state.llm_api_base_url.as_deref();

    if available_keys.is_empty() {
        return Err(GatewayError::ResponseBuild(
            "No provider keys configured for this tenant".into()
        ));
    }

    // ── Strategy-based provider selection ─────────────────────────────────
    let strategy_provider = match strategy {
        RoutingStrategy::LeastLatency => {
            let mut redis_conn = state.redis_conn.clone();
            match routing_strategy::resolve_least_latency(&mut redis_conn, &available_keys).await {
                Some(provider_id) => {
                    info!(
                        provider = %provider_id,
                        strategy = "least_latency",
                        "Strategy selected provider"
                    );
                    available_keys.iter()
                        .find(|k| k.provider_id == provider_id)
                        .map(|k| (k.provider_id.clone(), k.decrypted_key.clone()))
                }
                None => None,
            }
        }
        RoutingStrategy::CostOptimized => {
            let estimated_tokens = routing_strategy::estimate_input_tokens(body);
            match routing_strategy::resolve_cost_optimized(model, estimated_tokens, &available_keys) {
                Some((provider_id, est_cost)) => {
                    info!(
                        provider = %provider_id,
                        strategy = "cost_optimized",
                        estimated_tokens,
                        estimated_cost_usd = est_cost,
                        "Strategy selected cheapest provider"
                    );
                    available_keys.iter()
                        .find(|k| k.provider_id == provider_id)
                        .map(|k| (k.provider_id.clone(), k.decrypted_key.clone()))
                }
                None => None,
            }
        }
        RoutingStrategy::Default => None,
    };

    // Use strategy result, or fall back to default model-based selection.
    let (primary_provider, primary_key) = strategy_provider
        .or_else(|| select_provider_for_model(model, &available_keys))
        .ok_or_else(|| GatewayError::ResponseBuild(
            "No compatible provider key available".into()
        ))?;

    info!(
        provider = %primary_provider,
        model = %model,
        strategy = ?strategy,
        "Routing request to primary LLM provider"
    );

    let primary_result = execute_provider_request(
        &state.http_client,
        &primary_provider,
        &primary_key,
        body,
        accept_header,
        base_url_override,
    ).await;

    let primary_error = match primary_result {
        Ok(response) => {
            let status = response.status().as_u16();
            if !is_retryable_error(status) {
                return Ok(response);
            }
            warn!(
                provider = %primary_provider,
                status = status,
                "Primary provider failed with retryable error, attempting fallback"
            );
            GatewayError::ResponseBuild(format!("Primary provider returned {}", status))
        }
        Err(e) => {
            warn!(
                provider = %primary_provider,
                error = %e,
                "Primary provider unreachable, attempting fallback"
            );
            e
        }
    };

    try_fallback_providers(
        state,
        &available_keys,
        &primary_provider,
        body,
        accept_header,
        model,
        base_url_override,
        primary_error,
    ).await
}

pub fn translate_model_name(original_model: &str, target_provider_id: &str) -> String {
    let m = original_model.to_lowercase();
    
    match target_provider_id {
        "openai" | "anthropic" => {
            if m.contains("llama") || m.contains("mixtral") || m.contains("qwen") {
                if target_provider_id == "openai" {
                    return "gpt-4o-mini".to_string();
                } else {
                    return "claude-3-haiku-20240307".to_string();
                }
            }
        }
        "openrouter" => {
            if m == "llama-3.3-70b-versatile" {
                return "meta-llama/llama-3.3-70b-instruct".to_string();
            } else if m == "llama3-8b" || m == "llama-3-8b" {
                return "meta-llama/llama-3-8b-instruct".to_string();
            }
        }
        "groq" => {
            if m == "meta-llama/llama-3.3-70b-instruct" {
                return "llama-3.3-70b-versatile".to_string();
            } else if m.contains("llama-3") || m.contains("llama3") {
                if m.contains("70b") {
                    return "llama-3.3-70b-versatile".to_string();
                } else if m.contains("8b") {
                    return "llama3-8b-8192".to_string();
                }
            }
        }
        _ => {}
    }
    
    original_model.to_string()
}

async fn execute_provider_request(
    client: &reqwest::Client,
    provider_id: &str,
    api_key: &str,
    body: &Value,
    accept_header: &str,
    base_url_override: Option<&str>,
) -> Result<reqwest::Response, GatewayError> {
    let config = get_provider_config(provider_id)
        .ok_or_else(|| GatewayError::ResponseBuild(format!("Provider config missing for {}", provider_id)))?;

    let base_url = base_url_override.unwrap_or(config.base_url);

    let endpoint = if config.schema_format == SchemaFormat::Anthropic {
        format!("{}/messages", base_url)
    } else {
        format!("{}/chat/completions", base_url)
    };

    let mut request = client.post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Accept", accept_header);

    request = match config.auth_scheme {
        AuthScheme::Bearer => request.header("Authorization", format!("Bearer {}", api_key)),
        AuthScheme::GoogleApiKey => request.header("x-goog-api-key", api_key),
        AuthScheme::XApiKey => {
            if config.schema_format == SchemaFormat::Anthropic {
                request.header("x-api-key", api_key).header("anthropic-version", "2023-06-01")
            } else {
                request.header("x-api-key", api_key)
            }
        }
    };

    let mut final_body = body.clone();
    
    let original_model = extract_model(body);
    let translated_model = translate_model_name(original_model, config.id);
    final_body["model"] = Value::String(translated_model);
    
    if config.schema_format == SchemaFormat::Anthropic {
        use crate::infrastructure::translators::{OpenAIChatRequest, AnthropicRequest};
        use crate::domain::models::RedEyeConversation;

        let openai_req: OpenAIChatRequest = serde_json::from_value(final_body.clone())
            .map_err(|e| GatewayError::ResponseBuild(format!("OpenAI parse error for Anthropic format: {}", e)))?;
        
        let conv: RedEyeConversation = openai_req.try_into()
            .map_err(|e| GatewayError::ResponseBuild(format!("To UMS error: {}", e)))?;
            
        let anthropic_req: AnthropicRequest = conv.try_into()
            .map_err(|e| GatewayError::ResponseBuild(format!("To Anthropic error: {}", e)))?;
            
        final_body = serde_json::to_value(anthropic_req)
            .map_err(|e| GatewayError::ResponseBuild(format!("Json Serialization Error: {}", e)))?;
    }

    let response = request
        .json(&final_body)
        .send()
        .await
        .map_err(|e| {
            error!(
                provider = %config.id,
                error = %e,
                "Failed to reach upstream LLM provider"
            );
            GatewayError::UpstreamUnreachable(e)
        })?;

    Ok(response)
}

async fn try_fallback_providers(
    state: &Arc<AppState>,
    available_keys: &[ProviderKey],
    failed_provider: &str,
    body: &Value,
    accept_header: &str,
    original_model: &str,
    base_url_override: Option<&str>,
    mut final_error: GatewayError,
) -> Result<reqwest::Response, GatewayError> {
    let fallback_candidates = vec![
        "openrouter", "together", "groq", "anthropic", "openai", "gemini", "mistral", "deepseek"
    ];

    for fallback_provider in fallback_candidates {
        if fallback_provider == failed_provider {
            continue;
        }

        let Some(key_entry) = available_keys.iter().find(|k| k.provider_id == fallback_provider) else {
            continue;
        };

        info!(
            from_provider = failed_provider,
            to_provider = fallback_provider,
            "Attempting automated fallback"
        );

        let fallback_body = prepare_fallback_body(body, fallback_provider, original_model);

        match execute_provider_request(
            &state.http_client,
            fallback_provider,
            &key_entry.decrypted_key,
            &fallback_body,
            accept_header,
            base_url_override,
        ).await {
            Ok(response) => {
                let status = response.status().as_u16();
                if !is_retryable_error(status) {
                    info!(
                        provider = fallback_provider,
                        "Fallback successful - request completed"
                    );
                    return Ok(response);
                }
                warn!(
                    provider = fallback_provider,
                    status = status,
                    "Fallback provider also failed"
                );
                final_error = GatewayError::ResponseBuild(format!("Fallback provider returned {}", status));
            }
            Err(e) => {
                warn!(
                    provider = fallback_provider,
                    error = %e,
                    "Fallback provider unreachable"
                );
                final_error = e;
            }
        }
    }

    Err(final_error)
}

pub fn extract_model(body: &Value) -> &str {
    body.get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-4o")
}

pub async fn route_chat_completion(
    client: &reqwest::Client,
    api_key: &str,
    body: &Value,
    accept_header: &str,
    base_url_override: Option<&str>,
) -> Result<reqwest::Response, GatewayError> {
    let model = extract_model(body);
    let provider_id = detect_primary_provider(model);
    
    let mut response = execute_provider_request(
        client,
        provider_id,
        api_key,
        body,
        accept_header,
        base_url_override,
    ).await?;

    // Agentic Hot-Swap Feature Support for basic internal retry logic
    if provider_id == "openai" && (response.status().as_u16() == 503 || response.status().as_u16() == 429) {
        tracing::warn!("Primary provider failed. Triggering RedEye Hot-Swap to Anthropic Mock...");

        let anthropic_key = std::env::var("ANTHROPIC_BACKUP_KEY").unwrap_or_else(|_| "mock_backup_key".to_string());
        
        // Convert Request
        use crate::infrastructure::translators::{OpenAIChatRequest, AnthropicRequest};
        use crate::domain::models::RedEyeConversation;

        let openai_req: OpenAIChatRequest = serde_json::from_value(body.clone())
            .map_err(|_| GatewayError::ResponseBuild("Hot-Swap parse error for OpenAI request".to_string()))?;
        
        let conv: RedEyeConversation = openai_req.try_into()
            .map_err(|e| GatewayError::ResponseBuild(format!("Hot-Swap to internal struct failed: {}", e)))?;
        
        let anthropic_req: AnthropicRequest = conv.try_into()
            .map_err(|e| GatewayError::ResponseBuild(format!("Hot-Swap to Anthropic struct failed: {}", e)))?;

        let anthropic_base = std::env::var("ANTHROPIC_MOCK_URL").unwrap_or_else(|_| "https://api.anthropic.com/v1".to_string());
        let anthropic_endpoint = format!("{}/messages", anthropic_base.trim_end_matches('/'));
        
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

        let body_bytes = serde_json::to_vec(&mock_openai_resp)
            .map_err(|_| GatewayError::ResponseBuild("Failed to serialize fallback JSON".into()))?;
        let hr = axum::http::Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .header("x-redeye-hot-swapped", "1")
            .header("x-redeye-executed-provider", "anthropic")
            .body(reqwest::Body::from(body_bytes))
            .map_err(|_| GatewayError::ResponseBuild("Failed to build HTTP response".into()))?;
        
        response = reqwest::Response::from(hr);
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_detection() {
        assert_eq!(detect_primary_provider("gpt-4"), "openai");
        assert_eq!(detect_primary_provider("gpt-4o"), "openai");
        assert_eq!(detect_primary_provider("o1-preview"), "openai");
        assert_eq!(detect_primary_provider("claude-3-opus"), "anthropic");
        assert_eq!(detect_primary_provider("claude-3-sonnet"), "anthropic");
        assert_eq!(detect_primary_provider("gemini-pro"), "gemini");
        assert_eq!(detect_primary_provider("meta-llama/Llama-3-70b"), "openrouter");
        assert_eq!(detect_primary_provider("mixtral-8x7b"), "openrouter");
    }

    #[test]
    fn test_is_retryable_error() {
        assert!(is_retryable_error(429));
        assert!(is_retryable_error(500));
        assert!(is_retryable_error(502));
        assert!(is_retryable_error(503));
        assert!(is_retryable_error(504));
        assert!(!is_retryable_error(400));
        assert!(!is_retryable_error(401));
        assert!(!is_retryable_error(200));
    }

    #[test]
    fn test_select_provider_for_model() {
        let keys = vec![
            ProviderKey {
                provider_id: "openai".to_string(),
                decrypted_key: "sk-openai".to_string(),
            },
            ProviderKey {
                provider_id: "anthropic".to_string(),
                decrypted_key: "sk-anthropic".to_string(),
            },
        ];

        // Exact match
        let (provider, key) = select_provider_for_model("gpt-4", &keys).unwrap();
        assert_eq!(provider, "openai");
        assert_eq!(key, "sk-openai");

        // Another exact match
        let (provider, key) = select_provider_for_model("claude-3", &keys).unwrap();
        assert_eq!(provider, "anthropic");
        assert_eq!(key, "sk-anthropic");
    }

    #[test]
    fn test_select_provider_fallback_order() {
        // Only Anthropic available, but requesting GPT
        let keys = vec![
            ProviderKey {
                provider_id: "anthropic".to_string(),
                decrypted_key: "sk-anthropic".to_string(),
            },
        ];

        // Should fallback to Anthropic since no OpenAI key
        let (provider, key) = select_provider_for_model("gpt-4", &keys).unwrap();
        assert_eq!(provider, "anthropic");
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
        let fallback = prepare_fallback_body(&body, "anthropic", "gpt-4");
        assert_eq!(fallback["model"], "claude-3-haiku-20240307");

        // Keep original if compatible
        let fallback = prepare_fallback_body(&body, "openai", "gpt-4");
        assert_eq!(fallback["model"], "gpt-4");
    }
}
