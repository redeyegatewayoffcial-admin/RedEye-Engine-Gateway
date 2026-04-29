//! Postgres-backed configuration repository.
//!
//! # Architecture
//!
//! [`ConfigRepository`] is a trait that defines the complete storage contract
//! for this service.  The production implementation [`PgConfigRepository`]
//! wraps a SQLx [`PgPool`].  The trait is [`mockall::automock`]-annotated so
//! that handler unit tests can substitute a zero-clone in-memory mock without
//! spinning up a database container.
//!
//! # SQL Notes
//!
//! * `client_configs` is owned by this service (migration 20260421000000).
//! * `api_keys` is owned by `redeye_auth`; this service reads and hard-deletes
//!   rows from it during key-revocation (`DELETE … RETURNING *`).  Coordinate
//!   schema changes with the auth team.

use async_trait::async_trait;
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

use crate::{
    domain::models::{ApiKeyRecord, ClientConfig},
    error::ConfigError,
};

// =============================================================================
// Repository trait
// =============================================================================

/// Storage abstraction for all `redeye_config` persistence operations.
///
/// Implementations must be [`Send`] + [`Sync`] so they can be stored behind
/// an [`std::sync::Arc`] in Axum's shared state.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ConfigRepository: Send + Sync {
    /// Fetches the configuration for `tenant_id`.
    ///
    /// Returns [`ConfigError::NotFound`] if no row exists yet (lazy init model).
    async fn get_config(&self, tenant_id: Uuid) -> Result<ClientConfig, ConfigError>;

    /// Upserts (INSERT … ON CONFLICT DO UPDATE) a full [`ClientConfig`].
    ///
    /// Returns the row as it exists in the database after the write.
    async fn upsert_config(&self, config: &ClientConfig) -> Result<ClientConfig, ConfigError>;

    /// Lists all API keys belonging to `tenant_id`, newest first.
    async fn list_api_keys(&self, tenant_id: Uuid) -> Result<Vec<ApiKeyRecord>, ConfigError>;

    /// Fetches a single API key by its UUID, scoped to `tenant_id`.
    ///
    /// Returns [`ConfigError::NotFound`] if the key does not exist or belongs
    /// to a different tenant (prevents cross-tenant information leakage).
    async fn get_api_key_by_id(
        &self,
        key_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<ApiKeyRecord, ConfigError>;

    /// Hard-deletes an API key from `api_keys`, returning the deleted row.
    ///
    /// The returned [`ApiKeyRecord`] contains the `key_hash` needed for the
    /// subsequent Redis `DEL` + `PUBLISH` in the revocation workflow.
    ///
    /// Returns [`ConfigError::NotFound`] if the key does not exist for `tenant_id`.
    async fn revoke_api_key(
        &self,
        key_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<ApiKeyRecord, ConfigError>;
}

// =============================================================================
// PostgreSQL implementation
// =============================================================================

/// Production [`ConfigRepository`] backed by a SQLx Postgres connection pool.
pub struct PgConfigRepository {
    pool: PgPool,
}

impl PgConfigRepository {
    /// Wraps an existing connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Bootstraps a connection pool from the `DATABASE_URL` environment variable.
///
/// # Errors
///
/// Returns a [`sqlx::Error`] if the URL is not set or if initial connections
/// cannot be established.  The caller should map this to a startup failure.
pub async fn create_pool() -> Result<PgPool, sqlx::Error> {
    let url = std::env::var("DATABASE_URL").map_err(|_| {
        // sqlx::Error does not have a direct "missing env var" variant, so we
        // surface it as a configuration error through the io channel.
        sqlx::Error::Configuration("DATABASE_URL environment variable must be set".into())
    })?;

    tracing::info!("Connecting to PostgreSQL (redeye_config)…");

    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&url)
        .await
}

#[async_trait]
impl ConfigRepository for PgConfigRepository {
    async fn get_config(&self, tenant_id: Uuid) -> Result<ClientConfig, ConfigError> {
        let config = sqlx::query_as::<_, ClientConfig>(
            r#"
            SELECT
                tenant_id,
                pii_masking_enabled,
                semantic_caching_enabled,
                routing_fallback_enabled,
                rate_limit_rpm,
                preferred_model,
                updated_at
            FROM client_configs
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?; // sqlx::Error::RowNotFound auto-converts to ConfigError::NotFound

        tracing::debug!(%tenant_id, "Fetched client config from Postgres");
        Ok(config)
    }

    async fn upsert_config(&self, config: &ClientConfig) -> Result<ClientConfig, ConfigError> {
        let saved = sqlx::query_as::<_, ClientConfig>(
            r#"
            INSERT INTO client_configs (
                tenant_id,
                pii_masking_enabled,
                semantic_caching_enabled,
                routing_fallback_enabled,
                rate_limit_rpm,
                preferred_model,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (tenant_id) DO UPDATE
                SET pii_masking_enabled      = EXCLUDED.pii_masking_enabled,
                    semantic_caching_enabled = EXCLUDED.semantic_caching_enabled,
                    routing_fallback_enabled = EXCLUDED.routing_fallback_enabled,
                    rate_limit_rpm           = EXCLUDED.rate_limit_rpm,
                    preferred_model          = EXCLUDED.preferred_model,
                    updated_at               = NOW()
            RETURNING
                tenant_id,
                pii_masking_enabled,
                semantic_caching_enabled,
                routing_fallback_enabled,
                rate_limit_rpm,
                preferred_model,
                updated_at
            "#,
        )
        .bind(config.tenant_id)
        .bind(config.pii_masking_enabled)
        .bind(config.semantic_caching_enabled)
        .bind(config.routing_fallback_enabled)
        .bind(config.rate_limit_rpm)
        .bind(&config.preferred_model)
        .fetch_one(&self.pool)
        .await?;

        tracing::info!(
            tenant_id   = %config.tenant_id,
            pii_masking = config.pii_masking_enabled,
            caching     = config.semantic_caching_enabled,
            fallback    = config.routing_fallback_enabled,
            "Client config upserted to Postgres"
        );
        Ok(saved)
    }

    async fn list_api_keys(&self, tenant_id: Uuid) -> Result<Vec<ApiKeyRecord>, ConfigError> {
        let keys = sqlx::query_as::<_, ApiKeyRecord>(
            r#"
            SELECT
                id,
                tenant_id,
                key_hash,
                name,
                created_at,
                expires_at,
                is_active
            FROM api_keys
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        tracing::debug!(%tenant_id, key_count = keys.len(), "Listed API keys from Postgres");
        Ok(keys)
    }

    async fn get_api_key_by_id(
        &self,
        key_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<ApiKeyRecord, ConfigError> {
        let key = sqlx::query_as::<_, ApiKeyRecord>(
            r#"
            SELECT
                id,
                tenant_id,
                key_hash,
                name,
                created_at,
                expires_at,
                is_active
            FROM api_keys
            WHERE id = $1
              AND tenant_id = $2
            "#,
        )
        .bind(key_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(key)
    }

    async fn revoke_api_key(
        &self,
        key_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<ApiKeyRecord, ConfigError> {
        // Hard delete, returning the deleted row so the caller can extract
        // key_hash for the Redis DEL + PUBLISH event.
        let deleted = sqlx::query_as::<_, ApiKeyRecord>(
            r#"
            DELETE FROM api_keys
            WHERE id        = $1
              AND tenant_id = $2
            RETURNING
                id,
                tenant_id,
                key_hash,
                name,
                created_at,
                expires_at,
                is_active
            "#,
        )
        .bind(key_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        // sqlx returns RowNotFound if the DELETE matched zero rows.
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ConfigError::NotFound(format!(
                "API key {} not found for this tenant or already revoked.",
                key_id
            )),
            other => ConfigError::from(other),
        })?;

        tracing::info!(
            key_id    = %key_id,
            tenant_id = %tenant_id,
            key_name  = %deleted.name,
            "API key hard-deleted from Postgres"
        );
        Ok(deleted)
    }
}
