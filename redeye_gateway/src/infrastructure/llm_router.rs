//! infrastructure/llm_router.rs — Dynamic Multi-LLM Router with Automated Fallback

use std::sync::Arc;
use serde_json::Value;
use tracing::{error, info, warn};

use crate::domain::models::{AppState, KeyConfig};
use crate::error::GatewayError;
use crate::infrastructure::routing_strategy::RoutingStrategy;
use crate::infrastructure::translators::{
    AnthropicTranslator, BaseTranslator, GeminiTranslator, OpenAiTranslator,
};

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
    pub id: String,
    pub base_url: String, // Kept for backwards compatibility but we use model_config.base_url
    pub auth_scheme: AuthScheme,
    pub schema_format: SchemaFormat,
}

pub fn get_translator(schema: &SchemaFormat) -> Box<dyn BaseTranslator> {
    match schema {
        SchemaFormat::OpenAI => Box::new(OpenAiTranslator),
        SchemaFormat::Anthropic => Box::new(AnthropicTranslator),
        SchemaFormat::Gemini => Box::new(GeminiTranslator),
    }
}

pub fn get_provider_config(provider_name: &str) -> Option<ProviderConfig> {
    let provider = provider_name.to_lowercase();
    let (schema, auth) = if provider.contains("anthropic") {
        (SchemaFormat::Anthropic, AuthScheme::XApiKey)
    } else if provider.contains("gemini") {
        (SchemaFormat::Gemini, AuthScheme::GoogleApiKey)
    } else {
        (SchemaFormat::OpenAI, AuthScheme::Bearer)
    };

    Some(ProviderConfig {
        id: provider.clone(),
        base_url: String::new(), // Dynamic base_url used directly from model config
        auth_scheme: auth,
        schema_format: schema,
    })
}

fn is_retryable_error(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

pub struct PreparedUpstreamRequest {
    pub primary_request: reqwest::RequestBuilder,
    pub primary_key_alias: String,
    pub fallback_keys: Vec<KeyConfig>,
    pub provider_config: ProviderConfig,
    pub base_url: String,
    pub model: String,
    pub requested_provider: String,
    pub executed_provider: String,
    pub is_hot_swapped: u8,
    pub accept_header: String,
}

pub async fn prep_upstream_request(
    state: &Arc<AppState>,
    _tenant_id: &str,
    body_bytes: &axum::body::Bytes,
    accept_header: &str,
    _strategy: RoutingStrategy,
) -> Result<PreparedUpstreamRequest, GatewayError> {
    // 1. Extract the model from the incoming request
    #[derive(serde::Deserialize)]
    struct ExtractModel<'a> {
        #[serde(borrow)]
        model: Option<std::borrow::Cow<'a, str>>,
    }
    let extracted: ExtractModel = serde_json::from_slice(body_bytes).unwrap_or(ExtractModel { model: None });
    let model_owned = extracted.model.as_deref().unwrap_or("gpt-4o").to_string();
    let model = model_owned.as_str();

    // 2. Load the lock-free state
    let routing_map = state.routing_state.state.load();

    // 3. Lookup the model
    let model_config = routing_map.get(model).ok_or(GatewayError::ModelNotConfigured)?;

    if model_config.keys.is_empty() {
        warn!("No keys configured for model {}", model);
        return Err(GatewayError::NoActiveKeys);
    }

    // 4. Sort by priority and filter out circuit-breaker-blacklisted keys.
    let mut sorted_keys = model_config.keys.clone();
    sorted_keys.sort_by_key(|k| k.priority);

    // Drain keys that are currently open-circuited, collecting healthy ones.
    let mut healthy_keys: Vec<KeyConfig> = Vec::with_capacity(sorted_keys.len());
    for key in sorted_keys {
        if state.circuit_breaker.get(&key.key_alias).await.is_some() {
            warn!(
                model = %model,
                key_alias = %key.key_alias,
                "Circuit breaker OPEN for key — skipping"
            );
        } else {
            healthy_keys.push(key);
        }
    }

    if healthy_keys.is_empty() {
        error!(model = %model, "All keys for model are circuit-breaker-blacklisted");
        return Err(GatewayError::NoActiveKeys);
    }

    // 5. Apply the Base URL and API Key dynamically
    let provider_id = model_config.schema_format.as_str();
    let config = get_provider_config(provider_id).unwrap_or_else(|| ProviderConfig {
        id: provider_id.to_string(),
        base_url: model_config.base_url.clone(),
        auth_scheme: AuthScheme::Bearer,
        schema_format: SchemaFormat::OpenAI,
    });

    let requested_provider = config.id.clone();
    let executed_provider = config.id.clone();

    // The first healthy key is our primary key; the rest are ordered fallbacks.
    let mut healthy_keys_iter = healthy_keys.into_iter();
    // Safe: we already checked healthy_keys is non-empty above.
    let primary_key = healthy_keys_iter
        .next()
        .ok_or(GatewayError::NoActiveKeys)?;
    let fallback_keys: Vec<KeyConfig> = healthy_keys_iter.collect();

    let primary_request = build_provider_request(
        &state.http_client,
        &config,
        &model_config.base_url,
        &primary_key,
        model,
        body_bytes,
        accept_header,
    )?;

    Ok(PreparedUpstreamRequest {
        primary_request,
        primary_key_alias: primary_key.key_alias.clone(),
        fallback_keys,
        provider_config: config,
        base_url: model_config.base_url.clone(),
        model: model_owned,
        requested_provider,
        executed_provider,
        is_hot_swapped: 0,
        accept_header: accept_header.to_string(),
    })
}

pub async fn execute_upstream_request(
    state: &Arc<AppState>,
    prep: PreparedUpstreamRequest,
    body_bytes: &axum::body::Bytes,
) -> Result<reqwest::Response, GatewayError> {
    info!(
        model = %prep.model,
        "Attempting primary upstream request"
    );

    match prep.primary_request.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            if !is_retryable_error(status) {
                return Ok(response);
            }
            warn!(
                model = %prep.model,
                key_alias = %prep.primary_key_alias,
                status = status,
                "Primary provider key failed with retryable error, tripping circuit breaker and falling back"
            );
            // Trip the breaker for the primary key too!
            state.circuit_breaker.insert(prep.primary_key_alias.clone(), ()).await;
        }
        Err(e) => {
            warn!(
                model = %prep.model,
                key_alias = %prep.primary_key_alias,
                error = %e,
                "Primary provider key unreachable, tripping circuit breaker and falling back"
            );
            // Trip the breaker for the primary key too!
            state.circuit_breaker.insert(prep.primary_key_alias.clone(), ()).await;
        }
    }

    let mut last_error = GatewayError::ResponseBuild("Primary provider failed".to_string());

    // Fallback loop: circuit-breaker is tripped on each retryable failure.
    for key_config in prep.fallback_keys {
        info!(
            model = %prep.model,
            key_alias = %key_config.key_alias,
            priority = key_config.priority,
            "Attempting fallback upstream request"
        );

        let fallback_req = build_provider_request(
            &state.http_client,
            &prep.provider_config,
            &prep.base_url,
            &key_config,
            &prep.model,
            body_bytes,
            &prep.accept_header,
        )?;

        match fallback_req.send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                if !is_retryable_error(status) {
                    return Ok(response);
                }
                warn!(
                    model = %prep.model,
                    key_alias = %key_config.key_alias,
                    status = status,
                    "Provider key failed with retryable error, tripping circuit breaker and falling back"
                );
                // Trip the breaker — this key is blacklisted for 60 seconds.
                state.circuit_breaker.insert(key_config.key_alias.clone(), ()).await;
                warn!(
                    key_alias = %key_config.key_alias,
                    "Circuit breaker tripped for key {} for 60 seconds",
                    key_config.key_alias
                );
                last_error = GatewayError::ResponseBuild(format!("Provider returned {}", status));
            }
            Err(e) => {
                warn!(
                    model = %prep.model,
                    key_alias = %key_config.key_alias,
                    error = %e,
                    "Provider key unreachable, tripping circuit breaker and falling back"
                );
                // Trip the breaker for network-level failures too.
                state.circuit_breaker.insert(key_config.key_alias.clone(), ()).await;
                warn!(
                    key_alias = %key_config.key_alias,
                    "Circuit breaker tripped for key {} for 60 seconds",
                    key_config.key_alias
                );
                last_error = GatewayError::UpstreamUnreachable(e);
            }
        }
    }

    error!("All configured keys for {} exhausted", prep.model);
    Err(last_error)
}

pub async fn route_chat_completion_with_fallback(
    state: &Arc<AppState>,
    tenant_id: &str,
    body_bytes: &axum::body::Bytes,
    accept_header: &str,
    strategy: RoutingStrategy,
) -> Result<reqwest::Response, GatewayError> {
    let prep = prep_upstream_request(state, tenant_id, body_bytes, accept_header, strategy).await?;
    execute_upstream_request(state, prep, body_bytes).await
}

fn build_provider_request(
    client: &reqwest::Client,
    provider_config: &ProviderConfig,
    base_url: &str,
    key_config: &KeyConfig,
    model: &str,
    body_bytes: &axum::body::Bytes,
    accept_header: &str,
) -> Result<reqwest::RequestBuilder, GatewayError> {
    // 1. Determine Endpoint
    let endpoint = if provider_config.schema_format == SchemaFormat::Anthropic {
        format!("{}/messages", base_url)
    } else if provider_config.schema_format == SchemaFormat::Gemini {
        // Quick extraction to check stream flag for Gemini
        #[derive(serde::Deserialize)]
        struct ExtractStream { stream: Option<bool> }
        let is_stream = serde_json::from_slice::<ExtractStream>(body_bytes).unwrap_or(ExtractStream { stream: None }).stream.unwrap_or(false);
        if is_stream {
            format!("{}/{}:streamGenerateContent", base_url, model)
        } else {
            format!("{}/{}:generateContent", base_url, model)
        }
    } else {
        format!("{}/chat/completions", base_url)
    };

    let mut request = client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Accept", accept_header);

    request = match provider_config.auth_scheme {
        AuthScheme::Bearer => request.header("Authorization", format!("Bearer {}", key_config.api_key)),
        AuthScheme::GoogleApiKey => request.header("x-goog-api-key", &key_config.api_key),
        AuthScheme::XApiKey => {
            if provider_config.schema_format == SchemaFormat::Anthropic {
                request
                    .header("x-api-key", &key_config.api_key)
                    .header("anthropic-version", "2023-06-01")
            } else {
                request.header("x-api-key", &key_config.api_key)
            }
        }
    };

    // 3. Conditional Translation (Zero-Copy proxy for OpenAI schemas)
    let request = if provider_config.schema_format != SchemaFormat::OpenAI {
        use crate::infrastructure::translators::BaseTranslator;
        let source_translator = crate::infrastructure::translators::OpenAiTranslator;
        
        let parsed_value: Value = serde_json::from_slice(body_bytes)
            .map_err(|e| GatewayError::ResponseBuild(format!("Invalid JSON for translation: {}", e)))?;
            
        let mut conv = source_translator.to_universal(parsed_value)
            .map_err(|e| GatewayError::ResponseBuild(format!("Incoming translation failed: {}", e)))?;
        
        conv.model = Some(model.to_string());
        
        let translator = get_translator(&provider_config.schema_format);
        let final_body = translator
            .from_universal(&conv)
            .map_err(|e| GatewayError::ResponseBuild(format!("Translation error: {}", e)))?;
            
        request.json(&final_body)
    } else {
        // Zero-copy: Pass bytes directly
        request.body(body_bytes.clone())
    };

    Ok(request)
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{AppState, RoutingState, ModelConfig, KeyConfig};
    use crate::infrastructure::l1_cache::L1Cache;
    use crate::infrastructure::cache_client::CacheGrpcClient;
    use std::sync::Arc;
    use moka::future::Cache;
    use std::collections::HashMap;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::method;

    /// Helper to create a minimal AppState for unit testing routing logic.
    /// WARNING: This uses dummy values for fields not used in these tests (DB, Redis).
    async fn setup_test_state() -> (Arc<AppState>, MockServer) {
        let mock_server = MockServer::start().await;
        let (telemetry_tx, _) = tokio::sync::mpsc::channel(100);
        
        let circuit_breaker = Cache::builder()
            .max_capacity(100)
            .time_to_live(std::time::Duration::from_secs(60))
            .build();

        let loop_fallback_cache = Cache::builder()
            .max_capacity(100)
            .build();

        let routing_state = Arc::new(RoutingState::new());
        let l1_cache = Arc::new(L1Cache::new(1024).unwrap());
        let http_client = reqwest::Client::new();

        // No-op gRPC client
        let channel = tonic::transport::Channel::from_static("http://127.0.0.1:1").connect_lazy();
        let cache_grpc_client = CacheGrpcClient::new(channel);

        // We use MaybeUninit to bypass initialization of DB/Redis handles that aren't touched by the router.
        // This avoids UB with zero-initialization of types containing non-null pointers (like Arc).
        let state = unsafe {
            AppState {
                http_client,
                cache_grpc_client,
                compliance_url: String::new(),
                redis_conn: std::mem::MaybeUninit::uninit().assume_init(),
                db_pool: std::mem::MaybeUninit::uninit().assume_init(),
                rate_limit_max: 0,
                rate_limit_window: 0,
                clickhouse_url: String::new(),
                tracer_url: String::new(),
                dashboard_url: String::new(),
                llm_api_base_url: Some(mock_server.uri()),
                telemetry_tx,
                l1_cache,
                routing_state,
                circuit_breaker,
                loop_fallback_cache,
            }
        };

        (Arc::new(state), mock_server)
    }

    #[tokio::test]
    async fn test_circuit_breaker_blacklist_enforcement() {
        let (state, _) = setup_test_state().await;
        
        // 1. Setup: 2 keys, key-1 is primary (priority 1)
        let mut map = HashMap::new();
        map.insert("gpt-4".into(), ModelConfig {
            base_url: "http://localhost".into(),
            schema_format: "openai".into(),
            keys: vec![
                KeyConfig { key_alias: "key-1".into(), api_key: "sk-1".into(), priority: 1, weight: 1 },
                KeyConfig { key_alias: "key-2".into(), api_key: "sk-2".into(), priority: 2, weight: 1 },
            ],
        });
        state.routing_state.state.store(Arc::new(map));

        // 2. Blacklist key-1
        state.circuit_breaker.insert("key-1".into(), ()).await;

        // 3. Prep request
        let body = axum::body::Bytes::from(r#"{"model": "gpt-4"}"#);
        let prep = prep_upstream_request(&state, "t1", &body, "*/*", RoutingStrategy::Default).await.unwrap();

        // 4. Verify: key-2 should have been selected as primary (prep.fallback_keys should be empty)
        assert_eq!(prep.fallback_keys.len(), 0, "key-2 should have been promoted to primary, leaving no fallbacks");
        
        // We leak the Arc to prevent it from trying to drop uninitialized handles on DB/Redis
        std::mem::forget(state);
    }

    #[tokio::test]
    async fn test_circuit_breaker_tripping() {
        let (state, mock_server) = setup_test_state().await;

        // 1. Setup: Model with a failing key
        let mut map = HashMap::new();
        map.insert("gpt-4".into(), ModelConfig {
            base_url: mock_server.uri(),
            schema_format: "openai".into(),
            keys: vec![KeyConfig { key_alias: "fail-key".into(), api_key: "sk-1".into(), priority: 1, weight: 1 }],
        });
        state.routing_state.state.store(Arc::new(map));

        // 2. Mock: Provider returns 429
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        // 3. Execute
        let body = axum::body::Bytes::from(r#"{"model": "gpt-4"}"#);
        let prep = prep_upstream_request(&state, "t1", &body, "*/*", RoutingStrategy::Default).await.unwrap();
        let _ = execute_upstream_request(&state, prep, &body).await;

        // 4. Verify: Breaker is now open for "fail-key"
        let is_blacklisted = state.circuit_breaker.get("fail-key").await.is_some();
        assert!(is_blacklisted, "Circuit breaker should have tripped on 429 response");

        std::mem::forget(state);
    }
}
