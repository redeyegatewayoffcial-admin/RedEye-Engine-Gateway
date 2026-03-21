use std::sync::Arc;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use tracing::{info, instrument};

use crate::domain::models::{CacheLookupRequest, CacheStoreRequest, CachedResponse};
use crate::infrastructure::{openai_client::OpenAiClient, postgres_repo::PostgresRepo};

#[derive(Clone)]
pub struct SemanticSearchUseCase {
    pg_repo: Arc<PostgresRepo>,
    openai_client: Arc<OpenAiClient>,
}

impl SemanticSearchUseCase {
    pub fn new(pg_repo: Arc<PostgresRepo>, openai_client: Arc<OpenAiClient>) -> Self {
        Self {
            pg_repo,
            openai_client,
        }
    }

    #[instrument(skip(self, req))]
    pub async fn check_cache(&self, req: &CacheLookupRequest) -> Result<Option<CachedResponse>, String> {
        let ast_hash = Self::compute_ast_hash(&req.prompt);
        info!(ast_hash, "Computed Structural AST Hash");

        info!("Generating embedding for incoming prompt");
        let embedding = self.openai_client.get_embeddings(&req.prompt).await.map_err(|e| e.to_string())?;

        info!("Querying Postgres pgvector for semantic similarity with HNSW");
        let result = self.pg_repo.find_similar(&req.tenant_id, ast_hash, &embedding, 0.95).await.map_err(|e| e.to_string())?;
        
        if result.is_some() {
            info!("Semantic Cache HIT!");
        } else {
            info!("Semantic Cache MISS");
        }

        Ok(result)
    }

    #[instrument(skip(self, req))]
    pub async fn store_response(&self, req: &CacheStoreRequest) -> Result<(), String> {
        let ast_hash = Self::compute_ast_hash(&req.prompt);

        info!("Generating embedding for new prompt to cache");
        let embedding = self.openai_client.get_embeddings(&req.prompt).await.map_err(|e| e.to_string())?;

        info!("Storing payload in Postgres HNSW index");
        self.pg_repo.store(&req.tenant_id, ast_hash, &req.prompt, &req.response_content, &embedding).await.map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Implement Structural AST Hashing: isolates syntax, strips semantics.
    fn compute_ast_hash(prompt: &str) -> i64 {
        // Strip alphanumeric and whitespace characters.
        // What's left is pure structure: punctuation, brackets, XML tags, etc.
        let skeleton: String = prompt
            .chars()
            .filter(|c| !c.is_alphanumeric() && !c.is_whitespace())
            .collect();
            
        let mut hasher = DefaultHasher::new();
        skeleton.hash(&mut hasher);
        // Postgres BIGINT is i64
        hasher.finish() as i64
    }
}
