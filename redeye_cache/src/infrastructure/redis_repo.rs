use redis::{AsyncCommands, Client, RedisResult};
use serde_json::json;
use std::env;
use uuid::Uuid;

use crate::domain::models::CachedResponse;

#[derive(Clone)]
pub struct RedisRepo {
    client: Client,
}

impl RedisRepo {
    pub fn new() -> RedisResult<Self> {
        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://:redis_secret@localhost:6379".to_string());
        let client = Client::open(redis_url)?;
        Ok(Self { client })
    }

    /// Creates the RediSearch index if it doesn't exist
    pub async fn ensure_index(&self) -> RedisResult<()> {
        let mut con = self.client.get_multiplexed_async_connection().await?;
        
        let index_name = "idx:prompts";
        
        // We use a basic FT.INFO to check if it exists. If it errors, we create it.
        let info_result: RedisResult<redis::Value> = redis::cmd("FT.INFO").arg(index_name).query_async(&mut con).await;
        
        if info_result.is_err() {
            // Create a JSON index. Path `$.embedding` is a vector, `$.tenant` is a tag.
            // 1536 is the dimension for text-embedding-3-small
            let _ = redis::cmd("FT.CREATE")
                .arg(index_name)
                .arg("ON").arg("JSON")
                .arg("PREFIX").arg("1").arg("cache:")
                .arg("SCHEMA")
                .arg("$.tenant_id").arg("AS").arg("tenant_id").arg("TAG")
                .arg("$.embedding").arg("AS").arg("embedding").arg("VECTOR").arg("FLAT").arg("6")
                .arg("TYPE").arg("FLOAT32")
                .arg("DIM").arg("1536")
                .arg("DISTANCE_METRIC").arg("COSINE")
                .query_async::<()>(&mut con).await?;
        }

        Ok(())
    }

    /// Searches for the most similar prompt using KNN Vector Search
    pub async fn find_similar(&self, tenant_id: &str, embedding: &[f32], threshold: f32) -> RedisResult<Option<CachedResponse>> {
        let mut con = self.client.get_multiplexed_async_connection().await?;
        
        // Convert f32 array to raw bytes for RediSearch
        let embed_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_ne_bytes()).collect();

        // Query: KNN 1 for the specific tenant
        let query = format!("(@tenant_id:{{ {} }})=>[KNN 1 @embedding $BLOB AS score]", tenant_id);
        
        let result: redis::Value = redis::cmd("FT.SEARCH")
            .arg("idx:prompts")
            .arg(&query)
            .arg("PARAMS").arg("2").arg("BLOB").arg(&embed_bytes)
            .arg("RETURN").arg("3").arg("score").arg("$.content").arg("$.original_prompt")
            .arg("DIALECT").arg("2")
            .query_async(&mut con).await?;

        // Parse FT.SEARCH result (very raw in redis-rs)
        if let redis::Value::Array(arr) = result {
            if arr.is_empty() { return Ok(None); }
            
            // arr[0] is the count (integer)
            if let redis::Value::Int(count) = arr[0] {
                if count == 0 || arr.len() < 2 { return Ok(None); }
                
                // arr[1] is the key name, arr[2] is an array of returned fields [score, val, $.content, val, $.original_prompt, val]
                if let redis::Value::Array(fields) = &arr[2] {
                    let mut score = 0.0_f32;
                    let mut content = String::new();
                    let mut original_prompt = String::new();
                    
                    for i in (0..fields.len()).step_by(2) {
                        if let (redis::Value::BulkString(key), redis::Value::BulkString(val)) = (&fields[i], &fields[i+1]) {
                            let k = String::from_utf8_lossy(key);
                            let v = String::from_utf8_lossy(val);
                            match k.as_ref() {
                                "score" => score = v.parse().unwrap_or(1.0),
                                "$.content" => content = v.trim_matches('"').replace("\\n", "\n"),
                                "$.original_prompt" => original_prompt = v.trim_matches('"').to_string(),
                                _ => {}
                            }
                        }
                    }

                    // RediSearch COSINE distance: 0 is exact match, 2 is complete opposite.
                    // Similarity = 1 - (distance / 2), but RediSearch returns Distance.
                    // If distance is <= (1.0 - threshold) * 2 roughly, it's a match.
                    // Let's directly use a simple threshold on the raw score.
                    // If score (distance) is < 0.05, that's >95% similarity.
                    let max_distance = 1.0 - threshold;

                    if score <= max_distance {
                        return Ok(Some(CachedResponse {
                            content,
                            original_prompt,
                            similarity_score: 1.0 - score,
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Stores a new prompt+response+embedding in RedisJSON
    pub async fn store(&self, tenant_id: &str, prompt: &str, response_content: &str, embedding: &[f32]) -> RedisResult<()> {
        let mut con = self.client.get_multiplexed_async_connection().await?;
        
        let id = Uuid::new_v4().to_string();
        let key = format!("cache:{}", id);
        
        let json_data = json!({
            "tenant_id": tenant_id,
            "original_prompt": prompt,
            "content": response_content,
            "embedding": embedding,
            "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        });

        // Use JSON.SET
        let _: () = redis::cmd("JSON.SET")
            .arg(&key)
            .arg("$")
            .arg(json_data.to_string())
            .query_async(&mut con).await?;

        // Optional TTL: expire caches after 7 days
        let _: () = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(604800)
            .query_async(&mut con).await?;

        Ok(())
    }
}
