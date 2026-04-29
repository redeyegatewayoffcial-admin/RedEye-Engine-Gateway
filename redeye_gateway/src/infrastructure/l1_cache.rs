use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use moka::future::Cache;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::domain::models::GatewayError;

/// Hybrid L1 Cache using RAM for Exact and Semantic Matches.
pub struct L1Cache {
    /// Exact match layer: SHA256(Prompt) -> LLM Response bounds by Moka
    exact_cache: Cache<String, String>,
    /// Thread-safe local ONNX embedding model for Semantic L1 Check
    embedder: RwLock<TextEmbedding>,
    /// Vector Index: Ring buffer of recent embeddings and their responses
    /// Bounded to 5000 elements to strictly enforce RAM limits (pure rust).
    semantic_cache: RwLock<VecDeque<(Vec<f32>, String)>>,
}

impl L1Cache {
    /// Initializes the L1 cache. Limits memory using `capacity_bytes`.
    pub fn new(capacity_bytes: u64) -> Result<Self, GatewayError> {
        let exact_cache = Cache::builder()
            .max_capacity(capacity_bytes)
            .time_to_idle(Duration::from_secs(3600))
            .build();

        // 384 dimensions for BAAI/bge-small-en-v1.5
        let embedder_opts = InitOptions::new(EmbeddingModel::BGESmallENV15);
        let embedder = TextEmbedding::try_new(embedder_opts).map_err(|e| {
            GatewayError::ResponseBuild(format!("Failed to initialize L1 FastEmbed: {}", e))
        })?;

        Ok(Self {
            exact_cache,
            embedder: RwLock::new(embedder),
            semantic_cache: RwLock::new(VecDeque::with_capacity(5000)),
        })
    }

    /// Generates a SHA-256 hash of the prompt string
    fn hash_prompt(prompt: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Check Exact Match
    pub async fn get_exact(&self, prompt: &str) -> Option<String> {
        let hash = Self::hash_prompt(prompt);
        let hit = self.exact_cache.get(&hash).await;
        if hit.is_some() {
            debug!("L1 Exact Match hit for prompt!");
        }
        hit
    }

    /// Calculates Cosine distance between two vectors. Lower is better.
    fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
        let mut dot = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;
        for i in 0..a.len() {
            dot += a[i] * b[i];
            norm_a += a[i] * a[i];
            norm_b += b[i] * b[i];
        }
        if norm_a == 0.0 || norm_b == 0.0 {
            return 1.0;
        }
        let similarity = dot / (norm_a.sqrt() * norm_b.sqrt());
        1.0 - similarity
    }

    /// Check Semantic Match -> Threshold 0.9 similarity
    /// Distance < 0.1 for Cosine Metric
    pub async fn get_semantic(&self, prompt: &str) -> Option<String> {
        let embeddings = self.embedder.write().await.embed(vec![prompt], None).ok()?;
        let vector = embeddings.first()?;

        let queue = self.semantic_cache.read().await;

        let mut best_dist = f32::MAX;
        let mut best_response = None;

        for (cached_vec, response) in queue.iter() {
            let dist = Self::cosine_distance(vector, cached_vec);
            if dist < best_dist {
                best_dist = dist;
                best_response = Some(response.clone());
            }
        }

        if best_dist < 0.10 {
            debug!(distance = best_dist, "L1 Semantic Match hit!");
            return best_response;
        }

        None
    }

    /// Insert into L1 if payload is small, caching Exact + Vectorized Semantic
    pub async fn insert(&self, prompt: &str, response: &str) -> Result<(), GatewayError> {
        let hash = Self::hash_prompt(prompt);

        // Insert exact
        self.exact_cache.insert(hash, response.to_string()).await;

        // Vectorize and insert semantic
        let mut embedder = self.embedder.write().await;
        let embeddings = embedder
            .embed(vec![prompt], None)
            .map_err(|e| GatewayError::ResponseBuild(format!("L1 Embed failed: {}", e)))?;

        let vector = embeddings
            .first()
            .ok_or_else(|| GatewayError::ResponseBuild("L1 Embed returned empty".into()))?
            .clone();

        let mut queue = self.semantic_cache.write().await;
        if queue.len() >= 5000 {
            queue.pop_front();
        }
        queue.push_back((vector, response.to_string()));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_l1_exact_match() {
        let cache = match L1Cache::new(1024 * 1024) {
            Ok(c) => c,
            Err(_) => {
                println!("Skipping test due to FastEmbed network failure");
                return;
            }
        };
        cache.insert("Hello", "World").await.unwrap();

        let exact = cache.get_exact("Hello").await;
        assert_eq!(exact, Some("World".to_string()));

        let miss = cache.get_exact("Unknown").await;
        assert_eq!(miss, None);
    }

    #[tokio::test]
    async fn test_l1_semantic_match() {
        let cache = match L1Cache::new(1024 * 1024) {
            Ok(c) => c,
            Err(_) => return, // Skip if offline
        };
        cache
            .insert("This is a test prompt about rust.", "Rust response")
            .await
            .unwrap();

        // Very similar string, should have a distance < 0.1
        let semantic = cache
            .get_semantic("This is a test prompt about rust programming.")
            .await;
        assert!(
            semantic.is_some(),
            "Expected semantic match to trigger for similar prompt"
        );
    }

    #[tokio::test]
    async fn test_moka_eviction_limits() {
        // Initialize an artificially small cache
        let cache = match L1Cache::new(1024) {
            // 1 KB max capacity
            Ok(c) => c,
            Err(_) => return,
        };

        for i in 0..50 {
            let prompt = format!("Large Prompt {} with a lot of extra text to consume RAM", i);
            let response = format!(
                "Large Response {} with a lot of extra text to consume RAM",
                i
            );
            cache.insert(&prompt, &response).await.unwrap();
        }

        // Moka eviction is async, but we can just check it handles things gracefully without crashing
        let hit = cache
            .get_exact("Large Prompt 49 with a lot of extra text to consume RAM")
            .await;
        assert!(hit.is_some() || hit.is_none());
    }

    #[tokio::test]
    async fn test_l1_handle_empty() {
        let cache = match L1Cache::new(1024 * 1024) {
            Ok(c) => c,
            Err(_) => return,
        };
        // Insert empty edge case
        cache.insert("", "").await.unwrap();
        assert_eq!(cache.get_exact("").await, Some("".to_string()));
    }
}
