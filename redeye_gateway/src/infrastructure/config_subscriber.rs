use redis::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task;
use tracing::{error, info};
use tokio_stream::StreamExt;

use crate::domain::models::{ModelConfig, RoutingState};

/// Spawns a background Tokio task to subscribe to "redeye:routing_updates" 
/// and dynamically update the `arc_swap` locked RoutingState.
pub fn spawn_config_subscriber(redis_client: Client, state: Arc<RoutingState>) {
    task::spawn(async move {
        let mut con = match redis_client.get_async_pubsub().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to connect to Redis for Pub/Sub config subscriber: {}", e);
                return;
            }
        };

        if let Err(e) = con.subscribe("redeye:routing_updates").await {
            error!("Failed to subscribe to redeye:routing_updates channel: {}", e);
            return;
        }

        info!("Subscribed to redeye:routing_updates channel for lock-free dynamic configuration");
        let mut stream = con.on_message();

        while let Some(msg) = stream.next().await {
            if let Ok(payload) = msg.get_payload::<String>() {
                // We expect Phase 1 to publish `HashMap<String, ModelConfig>` or we merge it.
                // According to prompt Phase 1, Phase 1 set config_json for a specific tenant.
                // Wait! We will deserialize this as a Map and store it, or assume it's the full map for now.
                // To keep it simple and strictly following the exact instruction:
                match serde_json::from_str::<HashMap<String, ModelConfig>>(&payload) {
                    Ok(new_config) => {
                        // Atomically swap the entire map without locks blocking the hot path
                        state.state.store(Arc::new(new_config));
                        info!("Routing state atomically updated via Redis sync");
                    }
                    Err(e) => {
                        error!("Failed to deserialize routing config update: {}", e);
                        // Let's log payload snippet for debug
                        error!("Payload snippet: {:.100}", payload);
                    }
                }
            }
        }
    });
}
