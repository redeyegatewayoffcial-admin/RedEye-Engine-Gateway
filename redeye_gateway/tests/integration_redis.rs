//! Integration tests for Redis-based dynamic configuration synchronization.
//!
//! Verifies that the `redeye_config` service's published updates are correctly
//! ingested by the Gateway and atomically swapped into the routing mesh.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use redis::AsyncCommands;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::redis::Redis;

use redeye_gateway::domain::models::{ModelConfig, KeyConfig, RoutingState};
use redeye_gateway::infrastructure::config_subscriber::spawn_config_subscriber;

/// Test Scenario: Successful propagation of a valid routing map.
/// 1. Start Redis.
/// 2. Spawn Gateway Subscriber.
/// 3. Publish valid `HashMap<String, ModelConfig>`.
/// 4. Assert Gateway state matches.
#[tokio::test]
async fn test_redis_pubsub_config_update_success() {
    // 1. Setup Redis Container
    let redis_node = Redis::default().start().await.expect("Failed to start Redis");
    let redis_port = redis_node.get_host_port_ipv4(6379).await.expect("Failed to get Redis port");
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    let redis_client = redis::Client::open(redis_url.clone()).expect("Failed to create Redis client");

    // 2. Initialize Gateway State (Atomic Lock-Free)
    let routing_state = Arc::new(RoutingState::new());

    // 3. Spawn Subscriber Loop (Background Task)
    spawn_config_subscriber(redis_client.clone(), routing_state.clone());
    
    // Give subscriber a moment to establish the TCP connection to Redis
    sleep(Duration::from_millis(300)).await;

    // 4. Publish a valid update payload
    // This payload represents the "Source of Truth" from redeye_config/Postgres
    let mut config_map = HashMap::new();
    config_map.insert("gpt-4o-test".to_string(), ModelConfig {
        base_url: "https://api.openai.com/v1".to_string(),
        schema_format: "openai".to_string(),
        keys: vec![
            KeyConfig {
                key_alias: "primary-key".to_string(),
                api_key: "sk-test-123".to_string(),
                priority: 1,
                weight: 1,
            }
        ],
    });

    let payload = serde_json::to_string(&config_map).unwrap();
    let mut conn = redis_client.get_multiplexed_tokio_connection().await.expect("Failed to get publisher connection");
    let subscriber_count: i64 = conn.publish("redeye:routing_updates", payload).await.expect("Failed to publish to Redis");
    
    assert!(subscriber_count >= 1, "Gateway subscriber failed to receive the message");

    // 5. Verify the Gateway updated its lock-free state
    // We poll briefly to account for async propagation latency
    let mut success = false;
    for _ in 0..20 {
        let current_state = routing_state.state.load();
        if current_state.contains_key("gpt-4o-test") {
            let cfg = &current_state["gpt-4o-test"];
            assert_eq!(cfg.base_url, "https://api.openai.com/v1");
            assert_eq!(cfg.keys[0].key_alias, "primary-key");
            success = true;
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }
    
    assert!(success, "Gateway failed to ingest Redis routing update within timeout");
}

/// Test Scenario: Resilience against malformed/garbage payloads.
/// 1. Gateway should log error but NOT panic.
/// 2. Gateway should preserve last-known-good state.
/// 3. Gateway should recover and process a subsequent valid update.
#[tokio::test]
async fn test_redis_pubsub_invalid_json_resilience() {
    let redis_node = Redis::default().start().await.expect("Failed to start Redis");
    let redis_port = redis_node.get_host_port_ipv4(6379).await.expect("Failed to get Redis port");
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    let redis_client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    let routing_state = Arc::new(RoutingState::new());
    
    // Seed initial state
    let mut initial_map = HashMap::new();
    initial_map.insert("stable-model".to_string(), ModelConfig {
        base_url: "http://legacy".to_string(),
        schema_format: "openai".to_string(),
        keys: vec![],
    });
    routing_state.state.store(Arc::new(initial_map));

    spawn_config_subscriber(redis_client.clone(), routing_state.clone());
    sleep(Duration::from_millis(300)).await;

    // 1. Publish MALFORMED JSON (Garbage)
    let mut conn = redis_client.get_multiplexed_tokio_connection().await.unwrap();
    let _: i64 = conn.publish("redeye:routing_updates", "!!! NOT JSON !!!").await.unwrap();

    // 2. Verify Gateway is still healthy and preserved state
    sleep(Duration::from_millis(300)).await;
    let current_state = routing_state.state.load();
    assert!(current_state.contains_key("stable-model"), "State should remain intact after garbage input");
    assert_eq!(current_state["stable-model"].base_url, "http://legacy");
    
    // 3. Verify it recovers with a valid update
    let mut config_map = HashMap::new();
    config_map.insert("recovered-model".to_string(), ModelConfig {
        base_url: "http://recovered".to_string(),
        schema_format: "openai".to_string(),
        keys: vec![],
    });
    let payload = serde_json::to_string(&config_map).unwrap();
    let _: i64 = conn.publish("redeye:routing_updates", payload).await.unwrap();

    let mut success = false;
    for _ in 0..20 {
        let current_state = routing_state.state.load();
        if current_state.contains_key("recovered-model") {
            success = true;
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }
    assert!(success, "Gateway failed to recover after receiving invalid JSON");
}
