use std::time::Duration;

/// Validates an LLM provider API key by calling the provider's /models endpoint.
/// Returns Ok(true) if the key is valid (200 OK), Ok(false) otherwise.
/// Returns Err on network/timeout errors.
pub async fn validate_api_key(provider: &str, api_key: &str) -> Result<bool, String> {
    let (url, headers) = match provider {
        "openai" => (
            "https://api.openai.com/v1/models".to_string(),
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "gemini" => (
            "https://generativelanguage.googleapis.com/v1/models".to_string(),
            vec![("x-goog-api-key", api_key.to_string())],
        ),
        "groq" => (
            "https://api.groq.com/openai/v1/models".to_string(),
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "anthropic" => (
            "https://api.anthropic.com/v1/models".to_string(),
            vec![
                ("x-api-key", api_key.to_string()),
                ("anthropic-version", "2023-06-01".to_string()),
            ],
        ),
        "deepseek" => (
            "https://api.deepseek.com/models".to_string(),
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "openrouter" => (
            "https://openrouter.ai/api/v1/models".to_string(),
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "together" => (
            "https://api.together.xyz/v1/models".to_string(),
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        _ => {
            // FALLBACK: For unknown providers, we assume an OpenAI-compatible /v1/models endpoint.
            // If we don't know the URL, we skip strict validation by returning Ok(true) to avoid blocking the user.
            tracing::warn!(
                "Unknown provider '{}'. Skipping strict API key validation.",
                provider
            );
            return Ok(true);
        }
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut request = client.get(url);
    for (key, value) in headers {
        request = request.header(key, value);
    }

    match request.send().await {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(e) if e.is_timeout() => Err("API key validation timed out".to_string()),
        Err(e) => Err(format!("API key validation failed: {}", e)),
    }
}
