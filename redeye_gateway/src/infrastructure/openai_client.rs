use reqwest::Client;
use serde_json::Value;
use tracing::{error, info};

use crate::domain::models::GatewayError;

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

/// Forwards a chat completion request to OpenAI.
pub async fn forward_chat_completion(
    client: &Client,
    api_key: &str,
    body: &Value,
    accept_header: &str,
) -> Result<reqwest::Response, GatewayError> {
    info!("Forwarding request to OpenAI");

    let response = client
        .post(OPENAI_CHAT_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("Accept", accept_header)
        .json(body)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to reach OpenAI upstream");
            GatewayError::UpstreamUnreachable(e)
        })?;

    Ok(response)
}
