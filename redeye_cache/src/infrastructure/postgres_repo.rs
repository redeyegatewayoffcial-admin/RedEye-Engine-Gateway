use serde_json::json;
use sqlx::{PgPool, Row};
use std::env;
use uuid::Uuid;
use pgvector::Vector;

use crate::domain::models::CachedResponse;

#[derive(Clone)]
pub struct PostgresRepo {
    pool: PgPool,
}

impl PostgresRepo {
    pub async fn new() -> Result<Self, sqlx::Error> {
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/nexusai".to_string());
        let pool = PgPool::connect(&database_url).await?;
        Ok(Self { pool })
    }

    /// Searches for the most similar prompt using HNSW Vector Search
    /// Bypasses the cache if `ast_hash` does not strictly match the stored record.
    pub async fn find_similar(
        &self, 
        tenant_id: &str, 
        ast_hash: i64,
        embedding: &[f32], 
        threshold: f32
    ) -> Result<Option<CachedResponse>, sqlx::Error> {
        let tenant_uuid = Uuid::parse_str(tenant_id).unwrap_or_default();
        let query_vector = Vector::from(embedding.to_vec());

        // We use <-> for L2 distance, <=> for Cosine distance, or <#> for Inner Product.
        // pgvector `vector_cosine_ops` uses `<=>`.
        // `1.0 - threshold` is the maximum allowed distance for Cosine.
        let max_distance = 1.0 - threshold;

        let row = sqlx::query(
            "
            SELECT content, original_prompt, (embedding <=> $3) as distance
            FROM semantic_cache
            WHERE tenant_id = $1 
              AND structural_hash = $2
              AND (embedding <=> $3) < $4
            ORDER BY embedding <=> $3
            LIMIT 1
            "
        )
        .bind(tenant_uuid)
        .bind(ast_hash)
        .bind(query_vector)
        .bind(max_distance as f64)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(r) = row {
            let distance: f64 = r.try_get("distance").unwrap_or(0.0);
            let content: String = r.try_get("content").unwrap_or_default();
            let original_prompt: String = r.try_get("original_prompt").unwrap_or_default();

            Ok(Some(CachedResponse {
                content,
                original_prompt,
                similarity_score: (1.0 - distance) as f32,
            }))
        } else {
            Ok(None)
        }
    }

    /// Stores a new prompt+response+embedding alongside its structural AST hash
    pub async fn store(
        &self, 
        tenant_id: &str, 
        ast_hash: i64,
        prompt: &str, 
        response_content: &str, 
        embedding: &[f32]
    ) -> Result<(), sqlx::Error> {
        let tenant_uuid = Uuid::parse_str(tenant_id).unwrap_or_default();
        let vec = Vector::from(embedding.to_vec());

        sqlx::query(
            "
            INSERT INTO semantic_cache (tenant_id, structural_hash, original_prompt, content, embedding)
            VALUES ($1, $2, $3, $4, $5)
            "
        )
        .bind(tenant_uuid)
        .bind(ast_hash)
        .bind(prompt)
        .bind(response_content)
        .bind(vec)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
