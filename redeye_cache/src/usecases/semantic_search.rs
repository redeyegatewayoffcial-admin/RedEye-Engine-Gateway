use std::sync::Arc;

use tracing::{info, instrument};

use crate::domain::models::{CacheLookupRequest, CacheStoreRequest, CachedResponse};
use crate::infrastructure::{local_embedder::LocalEmbedder, postgres_repo::PostgresRepo};

#[derive(Clone)]
pub struct SemanticSearchUseCase {
    pg_repo: Arc<PostgresRepo>,
    embedder: Arc<LocalEmbedder>,
}

impl SemanticSearchUseCase {
    pub fn new(pg_repo: Arc<PostgresRepo>, embedder: Arc<LocalEmbedder>) -> Self {
        Self { pg_repo, embedder }
    }

    #[instrument(skip(self, req))]
    pub async fn check_cache(
        &self,
        req: &CacheLookupRequest,
    ) -> Result<Option<CachedResponse>, String> {
        // Bug 8 Fix: `compute_ast_hash` performs O(N) synchronous character iteration.
        // For a 100k-token payload this blocks the Tokio executor, causing API starvation.
        // `spawn_blocking` offloads it to Tokio's dedicated blocking thread pool.
        let prompt_clone = req.prompt.clone();
        let ast_hash = tokio::task::spawn_blocking(move || Self::compute_ast_hash(&prompt_clone))
            .await
            .map_err(|e| format!("AST hash task join error: {}", e))?;
        info!(ast_hash, "Computed Structural AST Hash");

        info!("Generating embedding for incoming prompt via local ONNX model");
        let embedding = self
            .embedder
            .embed(&req.prompt)
            .await
            .map_err(|e| e.to_string())?;

        info!("Querying Postgres pgvector for semantic similarity with HNSW");
        let result = self
            .pg_repo
            .find_similar(&req.tenant_id, ast_hash, &embedding, 0.95)
            .await
            .map_err(|e| e.to_string())?;

        if result.is_some() {
            info!("Semantic Cache HIT!");
        } else {
            info!("Semantic Cache MISS");
        }

        Ok(result)
    }

    #[instrument(skip(self, req))]
    pub async fn store_response(&self, req: &CacheStoreRequest) -> Result<(), String> {
        // Bug 8 Fix: Same spawn_blocking guard for the store path.
        let prompt_clone = req.prompt.clone();
        let ast_hash = tokio::task::spawn_blocking(move || Self::compute_ast_hash(&prompt_clone))
            .await
            .map_err(|e| format!("AST hash task join error: {}", e))?;

        info!("Generating embedding for new prompt to cache via local ONNX model");
        let embedding = self
            .embedder
            .embed(&req.prompt)
            .await
            .map_err(|e| e.to_string())?;

        info!("Storing payload in Postgres HNSW index");
        self.pg_repo
            .store(
                &req.tenant_id,
                ast_hash,
                &req.prompt,
                &req.response_content,
                &embedding,
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Implement Structural AST Hashing: isolates syntax, strips semantics.
    // O(N) Time where N is prompt length, O(1) Space
    fn compute_ast_hash(prompt: &str) -> i64 {
        // Strip alphanumeric and whitespace characters.
        // What's left is pure structure: punctuation, brackets, XML tags, etc.
        let skeleton: String = prompt
            .chars()
            .filter(|c| !c.is_alphanumeric() && !c.is_whitespace())
            .collect();

        let word_count = prompt.split_whitespace().count();
        let combined = format!("{}_{}", skeleton, word_count);
        xxhash_rust::xxh3::xxh3_64(combined.as_bytes()) as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_cache_key_generation() {
        // Generating reproducible cache keys for identical prompts
        let p1 = "How do I reverse a string in Rust?";
        let p2 = "How do I reverse a string in Rust?";
        assert_eq!(
            SemanticSearchUseCase::compute_ast_hash(p1),
            SemanticSearchUseCase::compute_ast_hash(p2)
        );
    }

    #[test]
    fn test_failure_cache_key_generation_miss() {
        // Different punctuation/structure should yield different keys
        let p1 = "How do I reverse a string in Rust?";
        let p2 = "How do I reverse a string in Python?";
        // Notice the word count is the same, and punctuation is '?', so wait:
        // skeleton of p1 = "?"
        // skeleton of p2 = "?"
        // Is hash identical? Let's check logic:
        // `skeleton: String = prompt.chars().filter(|c| !c.is_alphanumeric() && !c.is_whitespace()).collect();`
        // So `?` is the only non-alphanumeric. word_count is 8.
        // Wait, AST Hash strips ALL semantics! It only hashes structure + word_count.
        // Therefore p1 and p2 WILL have the SAME ast_hash!
        // That's exactly why pgvector is needed in stage 2.

        let hash1 = SemanticSearchUseCase::compute_ast_hash(p1);
        let hash2 = SemanticSearchUseCase::compute_ast_hash(p2);
        assert_eq!(
            hash1, hash2,
            "Structural hashes match for same word count and punctuation"
        );

        // This causes a true MISS structurally:
        let p3 = "How can I reverse string in Rust programming language!!";
        let hash3 = SemanticSearchUseCase::compute_ast_hash(p3);
        assert_ne!(hash1, hash3, "Different skeleton/word count must miss");
    }
}
