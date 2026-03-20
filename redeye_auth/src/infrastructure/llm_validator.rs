use std::time::Duration;

/// Validates an LLM provider API key by calling the provider's /models endpoint.
/// Returns Ok(true) if the key is valid (200 OK), Ok(false) otherwise.
/// Returns Err on network/timeout errors.
pub async fn validate_api_key(provider: &str, api_key: &str) -> Result<bool, String> {
    let (url, headers) = match provider {
        "openai" => (
            "https://api.openai.com/v1/models",
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "gemini" => (
            "https://generativelanguage.googleapis.com/v1/models",
            vec![("x-goog-api-key", api_key.to_string())],
        ),
        "groq" => (
            "https://api.groq.com/openai/v1/models",
            vec![("Authorization", format!("Bearer {}", api_key))],
        ),
        "anthropic" => (
            "https://api.anthropic.com/v1/models",
            vec![
                ("x-api-key", api_key.to_string()),
                ("anthropic-version", "2023-06-01".to_string()),
            ],
        ),
        _ => return Err(format!("Unsupported provider: {}", provider)),
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
