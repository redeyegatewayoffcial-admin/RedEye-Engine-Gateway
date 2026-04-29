use serde::{Deserialize, Serialize};

/// Represents an incoming prompt request to the cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheLookupRequest {
    pub tenant_id: String,
    pub model: String,
    pub prompt: String,
}

/// The structure of a cached completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub content: String,
    pub original_prompt: String,
    pub similarity_score: f32,
}

/// To store a new completion in the cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStoreRequest {
    pub tenant_id: String,
    pub model: String,
    pub prompt: String,
    pub response_content: String,
}
