use reqwest::Client;
use serde_json::{json, Value};
use tracing::info;

/// Checks the redeye_cache microservice for a semantic cache hit.
pub async fn lookup_cache(
    client: &Client,
    cache_base_url: &str,
    tenant_id: &str,
    model: &str,
    raw_prompt: &str,
) -> Option<String> {
    let base = cache_base_url.trim_end_matches('/');
    let cache_url = format!("{}/v1/cache/lookup", base);
    let payload = json!({
        "tenant_id": tenant_id,
        "model": model,
        "prompt": raw_prompt
    });

    if let Ok(res) = client.post(cache_url).json(&payload).send().await {
        if res.status().is_success() {
            if let Ok(data) = res.json::<Value>().await {
                if data["hit"].as_bool() == Some(true) {
                    info!("Semantic Cache HIT!");
                    return data["data"]["content"].as_str().map(|s| s.to_string());
                }
            }
        }
    }
    info!("Semantic Cache MISS");
    None
}

/// Stores a new prompt→response pair in the semantic cache (async, fire-and-forget).
pub async fn store_in_cache(
    client: &Client,
    cache_base_url: &str,
    tenant_id: &str,
    model: &str,
    raw_prompt: &str,
    response_content: &str,
) {
    let base = cache_base_url.trim_end_matches('/');
    let cache_store_url = format!("{}/v1/cache/store", base);
    let payload = json!({
        "tenant_id": tenant_id,
        "model": model,
        "prompt": raw_prompt,
        "response_content": response_content
    });

    let _ = client.post(cache_store_url).json(&payload).send().await;
    info!("Async task dispatched response to semantic cache tier");
}
