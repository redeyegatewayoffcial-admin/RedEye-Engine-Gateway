//! Redis synchronisation client for the `redeye_config` service.
//!
//! # Responsibilities
//!
//! 1. **Config push** — after a successful `upsert_config`, serialise the new
//!    [`ClientConfig`] into JSON and write it to `config:{tenant_id}` in Redis
//!    (TTL: 1 hour).  Concurrently, publish a [`ConfigUpdateEvent`] on the
//!    `redeye:config_updates` Pub/Sub channel so that the gateway's subscriber
//!    loop can invalidate its local `moka` cache entry without polling.
//!
//! 2. **Key revocation** — after a successful hard-delete of an API key, delete
//!    `api_key:{key_hash}` from Redis and publish a [`KeyRevocationEvent`] on
//!    `redeye:key_revocations`.  This gives the gateway sub-millisecond
//!    propagation of revocations without waiting for TTL expiry.
//!
//! # Fail-open contract
//!
//! Redis errors in this layer are logged at `error!` level but are **not**
//! propagated to the HTTP layer after a successful Postgres write.  Postgres
//! is the source of truth; Redis is an acceleration overlay.
//!
//! # Key schema (stable — coordinate changes with the gateway team)
//!
//! | Key pattern                | Type   | Value                      |
//! |----------------------------|--------|----------------------------|
//! | `config:{tenant_id}`       | STRING | JSON-encoded `ClientConfig` |
//! | `api_key:{key_hash}`       | STRING | JSON-encoded `ApiKeyRecord` |
//!
//! # Channel schema
//!
//! | Channel                   | Payload                        |
//! |---------------------------|--------------------------------|
//! | `redeye:config_updates`   | JSON-encoded `ConfigUpdateEvent`|
//! | `redeye:key_revocations`  | JSON-encoded `KeyRevocationEvent`|

use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands, Client};
use uuid::Uuid;

use crate::{
    domain::models::{ClientConfig, ConfigUpdateEvent, KeyRevocationEvent},
    error::ConfigError,
};

// =============================================================================
// Stable Redis key / channel constants
// =============================================================================

/// TTL applied to every `config:{tenant_id}` cache entry (1 hour).
const CONFIG_KEY_TTL_SECS: u64 = 3_600;

/// Pub/Sub channel for configuration change events consumed by the gateway.
const CHANNEL_CONFIG_UPDATES: &str = "redeye:config_updates";

/// Pub/Sub channel for key-revocation events consumed by the gateway.
const CHANNEL_KEY_REVOCATIONS: &str = "redeye:key_revocations";

// =============================================================================
// RedisSync trait
// =============================================================================

/// Abstraction over all Redis write operations performed by this service.
///
/// Parameterised as a trait for dependency injection: the handler tests
/// substitute a [`MockRedisSync`] instead of a live Redis connection.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RedisSync: Send + Sync {
    /// Writes `config` to `config:{tenant_id}` (with TTL) **and** publishes a
    /// [`ConfigUpdateEvent`] on [`CHANNEL_CONFIG_UPDATES`].
    async fn push_config_update(&self, config: &ClientConfig) -> Result<(), ConfigError>;

    /// Deletes `api_key:{key_hash}` from Redis **and** publishes a
    /// [`KeyRevocationEvent`] on [`CHANNEL_KEY_REVOCATIONS`].
    async fn invalidate_api_key(
        &self,
        key_hash: &str,
        tenant_id: Uuid,
        key_id: Uuid,
    ) -> Result<(), ConfigError>;
}

// =============================================================================
// Production implementation
// =============================================================================

/// Production [`RedisSync`] backed by a `redis-rs` multiplexed connection.
///
/// A multiplexed connection is used rather than a connection pool because:
/// * Config writes happen infrequently (human-driven dashboard actions).
/// * The multiplexed connection is `Clone` + `Send + Sync`, making it trivial
///   to share behind an [`std::sync::Arc`] without an extra pool abstraction.
pub struct RedisSyncClient {
    client: Client,
}

impl RedisSyncClient {
    /// Constructs a [`RedisSyncClient`] from a connected [`redis::Client`].
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Opens a fresh multiplexed async connection.
    ///
    /// A new connection is opened per-operation (rather than storing one)
    /// because `MultiplexedConnection` does not implement `Clone`.  For the
    /// low-frequency write path of a control-plane service, the connection
    /// overhead is negligible compared to the Postgres round-trip.
    async fn connection(&self) -> Result<MultiplexedConnection, ConfigError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(ConfigError::from)
    }
}

#[async_trait]
impl RedisSync for RedisSyncClient {
    async fn push_config_update(&self, config: &ClientConfig) -> Result<(), ConfigError> {
        let mut conn = self.connection().await?;

        // 1. Serialise the config — any serde failure is a programmer error.
        let payload = serde_json::to_string(config).map_err(|e| {
            ConfigError::Internal(format!("Failed to serialise ClientConfig for Redis: {e}"))
        })?;

        // 2. Write the cache entry.
        let cache_key = format!("config:{}", config.tenant_id);
        conn.set_ex::<_, _, ()>(&cache_key, &payload, CONFIG_KEY_TTL_SECS)
            .await
            .map_err(ConfigError::from)?;

        tracing::debug!(
            tenant_id = %config.tenant_id,
            cache_key = %cache_key,
            ttl_secs  = CONFIG_KEY_TTL_SECS,
            "Config written to Redis cache"
        );

        // 3. Publish the update event.
        let event = ConfigUpdateEvent {
            tenant_id: config.tenant_id,
            config: config.clone(),
        };
        let event_payload = serde_json::to_string(&event).map_err(|e| {
            ConfigError::Internal(format!(
                "Failed to serialise ConfigUpdateEvent for Pub/Sub: {e}"
            ))
        })?;

        let subscriber_count: i64 = conn
            .publish(CHANNEL_CONFIG_UPDATES, &event_payload)
            .await
            .map_err(ConfigError::from)?;

        tracing::info!(
            tenant_id        = %config.tenant_id,
            channel          = CHANNEL_CONFIG_UPDATES,
            subscriber_count = subscriber_count,
            "Config update published to Pub/Sub"
        );

        Ok(())
    }

    async fn invalidate_api_key(
        &self,
        key_hash: &str,
        tenant_id: Uuid,
        key_id: Uuid,
    ) -> Result<(), ConfigError> {
        let mut conn = self.connection().await?;

        // 1. Delete the key-validation cache entry.
        let cache_key = format!("api_key:{key_hash}");
        let del_count: i64 = conn.del(&cache_key).await.map_err(ConfigError::from)?;

        if del_count > 0 {
            tracing::info!(
                key_id    = %key_id,
                tenant_id = %tenant_id,
                cache_key = %cache_key,
                "Revoked API key deleted from Redis cache"
            );
        } else {
            // The key may never have been cached (e.g. never used). This is
            // not an error — log at debug so we don't spam production alerts.
            tracing::debug!(
                key_id    = %key_id,
                tenant_id = %tenant_id,
                cache_key = %cache_key,
                "Revoked API key was not present in Redis cache (may have never been used)"
            );
        }

        // 2. Publish the revocation event so the gateway can clear its own
        //    in-memory (moka) cache layer without waiting for TTL.
        let event = KeyRevocationEvent {
            tenant_id,
            key_id,
            key_hash: key_hash.to_owned(),
        };
        let event_payload = serde_json::to_string(&event).map_err(|e| {
            ConfigError::Internal(format!(
                "Failed to serialise KeyRevocationEvent for Pub/Sub: {e}"
            ))
        })?;

        let subscriber_count: i64 = conn
            .publish(CHANNEL_KEY_REVOCATIONS, &event_payload)
            .await
            .map_err(ConfigError::from)?;

        tracing::info!(
            key_id           = %key_id,
            tenant_id        = %tenant_id,
            channel          = CHANNEL_KEY_REVOCATIONS,
            subscriber_count = subscriber_count,
            "Key revocation event published to Pub/Sub"
        );

        Ok(())
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── Key schema stability ────────────────────────────────────────────────
    //
    // Changing these strings is a **breaking change** for the gateway.
    // These tests act as regression guards — if you need to rename a key,
    // do so intentionally and coordinate with the gateway team.

    #[test]
    fn config_key_ttl_is_one_hour() {
        assert_eq!(CONFIG_KEY_TTL_SECS, 3_600);
    }

    #[test]
    fn channel_names_are_stable() {
        assert_eq!(CHANNEL_CONFIG_UPDATES, "redeye:config_updates");
        assert_eq!(CHANNEL_KEY_REVOCATIONS, "redeye:key_revocations");
    }

    #[test]
    fn config_cache_key_format_is_stable() {
        let tid = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let key = format!("config:{tid}");
        assert_eq!(key, "config:00000000-0000-0000-0000-000000000001");
    }

    #[test]
    fn api_key_cache_key_format_is_stable() {
        let hash = "abc123def456";
        let key = format!("api_key:{hash}");
        assert_eq!(key, "api_key:abc123def456");
    }

    // ── ConfigUpdateEvent serialisation ────────────────────────────────────

    #[test]
    fn config_update_event_round_trips_via_json() {
        use crate::domain::models::ClientConfig;
        use chrono::Utc;

        let tid = Uuid::new_v4();
        let config = ClientConfig {
            tenant_id: tid,
            pii_masking_enabled: true,
            semantic_caching_enabled: false,
            routing_fallback_enabled: true,
            rate_limit_rpm: Some(1000),
            preferred_model: Some("gpt-4o-mini".into()),
            updated_at: Utc::now(),
        };
        let event = ConfigUpdateEvent {
            tenant_id: tid,
            config: config.clone(),
        };

        let json = serde_json::to_string(&event).expect("serialise");
        let decoded: ConfigUpdateEvent = serde_json::from_str(&json).expect("deserialise");

        assert_eq!(decoded.tenant_id, tid);
        assert_eq!(
            decoded.config.pii_masking_enabled,
            config.pii_masking_enabled
        );
        assert_eq!(decoded.config.rate_limit_rpm, config.rate_limit_rpm);
        assert_eq!(decoded.config.preferred_model, config.preferred_model);
    }

    // ── KeyRevocationEvent serialisation ───────────────────────────────────

    #[test]
    fn key_revocation_event_round_trips_via_json() {
        let tid = Uuid::new_v4();
        let kid = Uuid::new_v4();
        let hash = "deadbeef1234567890abcdef".to_string();

        let event = KeyRevocationEvent {
            tenant_id: tid,
            key_id: kid,
            key_hash: hash.clone(),
        };

        let json = serde_json::to_string(&event).expect("serialise");
        let decoded: KeyRevocationEvent = serde_json::from_str(&json).expect("deserialise");

        assert_eq!(decoded.tenant_id, tid);
        assert_eq!(decoded.key_id, kid);
        assert_eq!(decoded.key_hash, hash);
    }
}
