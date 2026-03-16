use std::sync::Arc;
use tracing::{info, instrument};

use crate::domain::models::{CacheLookupRequest, CacheStoreRequest, CachedResponse};
use crate::infrastructure::{openai_client::OpenAiClient, redis_repo::RedisRepo};

#[derive(Clone)]
pub struct SemanticSearchUseCase {
    redis_repo: Arc<RedisRepo>,
    openai_client: Arc<OpenAiClient>,
}

impl SemanticSearchUseCase {
    pub fn new(redis_repo: Arc<RedisRepo>, openai_client: Arc<OpenAiClient>) -> Self {
        Self {
            redis_repo,
            openai_client,
        }
    }

    #[instrument(skip(self, req))]
    pub async fn check_cache(&self, req: &CacheLookupRequest) -> Result<Option<CachedResponse>, String> {
        info!("Generating embedding for incoming prompt");
        let embedding = self.openai_client.get_embeddings(&req.prompt).await.map_err(|e| e.to_string())?;

        info!("Querying vector database for semantic similarity");
        // Threshold: 0.95 (95% similarity)
        let result = self.redis_repo.find_similar(&req.tenant_id, &embedding, 0.95).await.map_err(|e| e.to_string())?;
        
        if result.is_some() {
            info!("Semantic Cache HIT!");
        } else {
            info!("Semantic Cache MISS");
        }

        Ok(result)
    }

    #[instrument(skip(self, req))]
    pub async fn store_response(&self, req: &CacheStoreRequest) -> Result<(), String> {
        info!("Generating embedding for new prompt to cache");
        let embedding = self.openai_client.get_embeddings(&req.prompt).await.map_err(|e| e.to_string())?;

        info!("Storing payload in RedisJSON");
        self.redis_repo.store(&req.tenant_id, &req.prompt, &req.response_content, &embedding).await.map_err(|e| e.to_string())?;

        Ok(())
    }
}
