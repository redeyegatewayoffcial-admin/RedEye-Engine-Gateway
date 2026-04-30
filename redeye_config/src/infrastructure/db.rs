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
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

use crate::{
    domain::models::{ApiKeyRecord, ClientConfig},
    error::ConfigError,
};

// =============================================================================
// RoutingEntry — flattened row from the routing JOIN query
// =============================================================================

/// A single row from the JOIN of `llm_models` and `provider_keys`.
/// One row per (model, key) combination. The handler assembles these into
/// `HashMap<model_name, ModelConfig { base_url, schema_format, keys: [...] }>`.
#[derive(Debug, sqlx::FromRow)]
pub struct RoutingEntry {
    pub model_name: String,
    pub base_url: String,
    pub schema_format: String,
    pub key_alias: String,
    /// Plaintext API key — decrypted by the infrastructure layer before storage
    /// in this struct. Never persisted after assembly.
    pub api_key: String,
    pub priority: i32,
    pub weight: i32,
}

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

    /// Upserts a row in `llm_models` for the given tenant.
    ///
    /// On conflict (same `tenant_id` + `model_name`) the `base_url` and
    /// `schema_format` columns are updated in place.
    async fn upsert_llm_model(
        &self,
        tenant_id: Uuid,
        model_name: &str,
        base_url: &str,
        schema_format: &str,
    ) -> Result<(), ConfigError>;

    /// Returns all (model, key) routing pairs for `tenant_id`.
    ///
    /// Each [`RoutingEntry`] contains the decrypted API key and the model
    /// metadata needed to build a `ModelConfig`.  The handler groups entries
    /// by `model_name` to produce `HashMap<String, ModelConfig>`.
    async fn get_routing_entries(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<RoutingEntry>, ConfigError>;

    /// Lists all LLM model rows registered for `tenant_id`, ordered by name.
    ///
    /// Used by `GET /v1/config/:tenant_id/models` to populate the routing
    /// strategy UI in the dashboard without exposing key material.
    async fn list_models(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<crate::domain::models::LlmModel>, ConfigError>;
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

    async fn upsert_llm_model(
        &self,
        tenant_id: Uuid,
        model_name: &str,
        base_url: &str,
        schema_format: &str,
    ) -> Result<(), ConfigError> {
        // provider_name is derived from schema_format (they share the same value
        // e.g. "openai", "anthropic", "gemini") — the model knows which provider
        // schema it uses and that IS the provider name for key JOIN purposes.
        sqlx::query(
            r#"
            INSERT INTO llm_models (id, tenant_id, model_name, provider_name, base_url, schema_format)
            VALUES (gen_random_uuid(), $1, $2, $3, $4, $3)
            ON CONFLICT (tenant_id, model_name)
            DO UPDATE SET
                base_url      = EXCLUDED.base_url,
                schema_format = EXCLUDED.schema_format,
                provider_name = EXCLUDED.provider_name
            "#,
        )
        .bind(tenant_id)
        .bind(model_name)
        .bind(schema_format)   // $3 = provider_name (same value as schema_format)
        .bind(base_url)        // $4 = base_url
        .execute(&self.pool)
        .await?;

        tracing::debug!(
            %tenant_id,
            model = %model_name,
            "llm_models row upserted"
        );
        Ok(())
    }

    async fn get_routing_entries(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<RoutingEntry>, ConfigError> {
        // JOIN llm_models → provider_keys via:
        //   - same tenant
        //   - m.provider_name = pk.provider_name  (the shared "openai"/"anthropic" discriminant)
        // The encrypted_key is read as bytes and decrypted in Rust — never plaintext on the wire.
        let rows = sqlx::query(
            r#"
            SELECT
                m.model_name,
                m.base_url,
                m.schema_format,
                pk.key_alias,
                pk.encrypted_key,
                pk.priority,
                pk.weight
            FROM llm_models m
            JOIN provider_keys pk
              ON pk.tenant_id    = m.tenant_id
             AND pk.provider_name = m.provider_name
            WHERE m.tenant_id  = $1
              AND pk.is_active  = true
            ORDER BY m.model_name, pk.priority ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let encrypted_key: Vec<u8> = row
                .try_get("encrypted_key")
                .map_err(|_| ConfigError::Internal("Failed to read encrypted_key".into()))?;

            // Decrypt using the same AES-256-GCM key the Auth service used.
            let api_key = crate::infrastructure::crypto::decrypt_api_key(&encrypted_key)
                .map_err(|e| ConfigError::Internal(format!("Key decryption failed: {e}")))?;

            entries.push(RoutingEntry {
                model_name:    row.try_get("model_name").unwrap_or_default(),
                base_url:      row.try_get("base_url").unwrap_or_default(),
                schema_format: row.try_get("schema_format").unwrap_or_default(),
                key_alias:     row.try_get("key_alias").unwrap_or_default(),
                api_key,
                priority:      row.try_get("priority").unwrap_or(1),
                weight:        row.try_get("weight").unwrap_or(100),
            });
        }

        tracing::debug!(%tenant_id, count = entries.len(), "Routing entries loaded");
        Ok(entries)
    }

    async fn list_models(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<crate::domain::models::LlmModel>, ConfigError> {
        let models = sqlx::query_as::<_, crate::domain::models::LlmModel>(
            r#"
            SELECT id, model_name, provider_name, base_url, created_at
            FROM   llm_models
            WHERE  tenant_id = $1
            ORDER  BY model_name ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        tracing::debug!(%tenant_id, count = models.len(), "LLM models listed");
        Ok(models)
    }
}
