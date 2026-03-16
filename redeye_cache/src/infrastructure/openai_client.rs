use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbeddingsError {
    #[error("API request failed: {0}")]
    RequestFixed(#[from] reqwest::Error),
    #[error("Missing or invalid OpenAI API Key")]
    AuthError,
    #[error("API returned error: {0}")]
    ApiError(String),
}

#[derive(Clone)]
pub struct OpenAiClient {
    client: Client,
    api_key: String,
}

impl OpenAiClient {
    pub fn new() -> Result<Self, EmbeddingsError> {
        let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "placeholder".to_string());
        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    pub async fn get_embeddings(&self, text: &str) -> Result<Vec<f32>, EmbeddingsError> {
        let url = "https://api.openai.com/v1/embeddings";
        
        let payload = json!({
            "input": text,
            "model": "text-embedding-3-small"
        });

        let res = self.client.post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(EmbeddingsError::ApiError(err_text));
        }

        let data: Value = res.json().await?;
        
        // Extract the embedding array from the response
        let embedding = data["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| EmbeddingsError::ApiError("Invalid response format".into()))?
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        Ok(embedding)
    }
}
