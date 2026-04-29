//! Axum request handlers for the `redeye_config` REST API.
//!
//! # Routes (mounted by [`crate::api::router`])
//!
//! | Method | Path                                        | Handler              |
//! |--------|---------------------------------------------|----------------------|
//! | GET    | `/v1/config/:tenant_id`                     | [`get_config`]       |
//! | PUT    | `/v1/config/:tenant_id`                     | [`upsert_config`]    |
//! | GET    | `/v1/config/:tenant_id/api-keys`            | [`list_api_keys`]    |
//! | DELETE | `/v1/config/:tenant_id/api-keys/:key_id`    | [`revoke_api_key`]   |
//!
//! # Design
//!
//! * Handlers receive injected [`crate::AppState`] containing `Arc<dyn ConfigRepository>`
//!   and `Arc<dyn RedisSync>` — this trait-object approach is what enables the
//!   mock-based unit tests in the `tests` module below.
//! * The Redis sync step is performed **after** a successful Postgres write and
//!   with a **fail-open** contract: if Redis is temporarily unavailable, the error
//!   is logged at `error!` level but the HTTP response is still `200`/`204`.
//!   Postgres is the authoritative source of truth; Redis is an acceleration layer.
//! * No `unwrap()` or `expect()` appear anywhere in production code paths.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    domain::models::{ApiKeyRecord, ClientConfig, UpdateConfigRequest},
    error::ConfigError,
    AppState,
};

// =============================================================================
// Response types
// =============================================================================

/// Wrapper returned by [`get_config`] and [`upsert_config`].
///
/// Re-exposes [`ClientConfig`] directly; the wrapper exists so we can add
/// envelope fields (e.g. `_links`) later without a breaking change.
#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    #[serde(flatten)]
    pub config: ClientConfig,
}

/// Lightweight API-key summary returned by [`list_api_keys`].  The `key_hash`
/// field is *intentionally omitted* (`#[serde(skip)]` on [`ApiKeyRecord`]).
#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: bool,
}

impl From<ApiKeyRecord> for ApiKeyResponse {
    fn from(rec: ApiKeyRecord) -> Self {
        Self {
            id: rec.id,
            name: rec.name,
            created_at: rec.created_at,
            expires_at: rec.expires_at,
            is_active: rec.is_active,
        }
    }
}

// =============================================================================
// GET /v1/config/:tenant_id
// =============================================================================

/// Returns the current control-plane configuration for `tenant_id`.
///
/// responds with `404` if the tenant has never saved a configuration.
/// The client should issue a `PUT` to create the initial config.
pub async fn get_config(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> Result<impl IntoResponse, ConfigError> {
    let config = state.repo.get_config(tenant_id).await?;

    tracing::debug!(%tenant_id, "GET /v1/config — config retrieved");

    Ok((StatusCode::OK, Json(ConfigResponse { config })))
}

// =============================================================================
// PUT /v1/config/:tenant_id
// =============================================================================

/// Creates or partially updates the configuration for `tenant_id`.
///
/// Implements PATCH-style semantics: absent JSON fields preserve their
/// existing value.  When no config exists yet, system defaults are used as
/// the base and the provided fields are applied on top.
///
/// After a successful Postgres write, the new config is pushed to Redis and a
/// Pub/Sub event is published.  Redis failures are logged but do **not** cause
/// this handler to return an error.
pub async fn upsert_config(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<UpdateConfigRequest>,
) -> Result<impl IntoResponse, ConfigError> {
    // ── 1. Domain validation ─────────────────────────────────────────────────
    request.validate().map_err(ConfigError::Validation)?;

    // ── 2. Load existing config or fall back to system defaults ─────────────
    let base = match state.repo.get_config(tenant_id).await {
        Ok(existing) => existing,
        Err(ConfigError::NotFound(_)) => ClientConfig::default_for(tenant_id),
        Err(other) => return Err(other),
    };

    // ── 3. Merge the partial update onto the base ────────────────────────────
    let merged = request.apply_to(&base);

    // ── 4. Persist to Postgres (UPSERT) ─────────────────────────────────────
    let saved = state.repo.upsert_config(&merged).await?;

    tracing::info!(%tenant_id, "Client config upserted successfully");

    // ── 5. Sync to Redis — fail-open ─────────────────────────────────────────
    // A Redis hiccup must never roll back a committed config change.
    // The gateway will re-read from Postgres on its next cache miss.
    if let Err(redis_err) = state.redis.push_config_update(&saved).await {
        tracing::error!(
            %tenant_id,
            error = %redis_err,
            "Redis config push failed after successful DB write; \
             gateway may serve stale config briefly until TTL expiry"
        );
    }

    Ok((StatusCode::OK, Json(ConfigResponse { config: saved })))
}

// =============================================================================
// GET /v1/config/:tenant_id/api-keys
// =============================================================================

/// Lists all API keys associated with `tenant_id`, newest first.
///
/// Key hashes are never returned; the response contains only metadata
/// (id, name, created_at, expires_at, is_active).
pub async fn list_api_keys(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> Result<impl IntoResponse, ConfigError> {
    let keys = state.repo.list_api_keys(tenant_id).await?;
    let response: Vec<ApiKeyResponse> = keys.into_iter().map(ApiKeyResponse::from).collect();

    tracing::debug!(%tenant_id, count = response.len(), "GET /v1/config/api-keys — keys listed");

    Ok((StatusCode::OK, Json(response)))
}

// =============================================================================
// DELETE /v1/config/:tenant_id/api-keys/:key_id
// =============================================================================

/// Irrevocably revokes an API key.
///
/// The operation is atomic at the Postgres layer: the key row is hard-deleted
/// in a single `DELETE … RETURNING` statement, which guarantees that the key
/// cannot be used even if the Redis invalidation step fails.
///
/// # Revocation workflow
///
/// 1. Hard-delete the key from Postgres (`api_keys` table, owned by redeye_auth).
/// 2. Delete `api_key:{hash}` from Redis (targeted DEL, no scan required).
/// 3. Publish a [`KeyRevocationEvent`] on `redeye:key_revocations` so the
///    gateway can flush its in-process `moka` cache entry immediately.
///
/// Steps 2 and 3 are **fail-open**: a Redis error causes a warning log entry
/// but does not affect the HTTP response code.  Since the key is already gone
/// from Postgres (the authoritative store), it cannot be validated even if
/// a stale Redis cache entry persists until its TTL expires.
///
/// Returns `204 No Content` on success.
pub async fn revoke_api_key(
    State(state): State<AppState>,
    Path((tenant_id, key_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ConfigError> {
    // ── 1. Hard-delete from Postgres; retrieve the deleted row ───────────────
    let revoked = state.repo.revoke_api_key(key_id, tenant_id).await?;

    tracing::info!(
        %key_id,
        %tenant_id,
        key_name = %revoked.name,
        "API key revoked from Postgres"
    );

    // ── 2 & 3. Invalidate Redis cache + publish revocation event ─────────────
    if let Err(redis_err) = state
        .redis
        .invalidate_api_key(&revoked.key_hash, tenant_id, key_id)
        .await
    {
        tracing::error!(
            %key_id,
            %tenant_id,
            error = %redis_err,
            "Redis key invalidation failed after successful DB revocation; \
             the key is revoked but a stale cache entry may persist for up to 1h"
        );
    }

    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use mockall::predicate::*;
    use tower::ServiceExt; // for `oneshot`

    use crate::{
        api::router::create_router,
        domain::models::ClientConfig,
        infrastructure::{db::MockConfigRepository, redis_sync::MockRedisSync},
    };

    // ── Fixtures ────────────────────────────────────────────────────────────

    fn fixture_config(tenant_id: Uuid) -> ClientConfig {
        ClientConfig {
            tenant_id,
            pii_masking_enabled: true,
            semantic_caching_enabled: true,
            routing_fallback_enabled: false,
            rate_limit_rpm: Some(1_000),
            preferred_model: Some("gpt-4o-mini".into()),
            updated_at: chrono::Utc::now(),
        }
    }

    fn fixture_api_key(tenant_id: Uuid) -> ApiKeyRecord {
        ApiKeyRecord {
            id: Uuid::new_v4(),
            tenant_id,
            key_hash: "sha256hashdeadbeef1234567890abcdef".into(),
            name: "Production Key".into(),
            created_at: chrono::Utc::now(),
            expires_at: None,
            is_active: true,
        }
    }

    /// Builds a fully-configured Axum router with injected mocks.
    fn build_app(mock_repo: MockConfigRepository, mock_redis: MockRedisSync) -> axum::Router {
        let state = AppState {
            repo: Arc::new(mock_repo),
            redis: Arc::new(mock_redis),
        };
        create_router(state)
    }

    // ── GET /v1/config/:tenant_id ────────────────────────────────────────────

    #[tokio::test]
    async fn get_config_returns_200_with_config_body() {
        let tenant_id = Uuid::new_v4();
        let config = fixture_config(tenant_id);
        let config_clone = config.clone();

        let mut mock_repo = MockConfigRepository::new();
        mock_repo
            .expect_get_config()
            .with(eq(tenant_id))
            .once()
            .returning(move |_| Ok(config_clone.clone()));

        let app = build_app(mock_repo, MockRedisSync::new());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/v1/config/{tenant_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body: serde_json::Value = {
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            serde_json::from_slice(&bytes).unwrap()
        };

        assert_eq!(body["tenant_id"], tenant_id.to_string());
        assert_eq!(body["pii_masking_enabled"], true);
        assert_eq!(body["rate_limit_rpm"], 1_000);
    }

    #[tokio::test]
    async fn get_config_returns_404_when_not_found() {
        let tenant_id = Uuid::new_v4();

        let mut mock_repo = MockConfigRepository::new();
        mock_repo
            .expect_get_config()
            .once()
            .returning(|_| Err(ConfigError::NotFound("config not found".into())));

        let app = build_app(mock_repo, MockRedisSync::new());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/v1/config/{tenant_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body: serde_json::Value = {
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            serde_json::from_slice(&bytes).unwrap()
        };
        assert_eq!(body["error"]["code"], "NOT_FOUND");
    }

    #[tokio::test]
    async fn get_config_returns_500_on_db_error() {
        let tenant_id = Uuid::new_v4();

        let mut mock_repo = MockConfigRepository::new();
        mock_repo
            .expect_get_config()
            .once()
            .returning(|_| Err(ConfigError::Database("connection refused".into())));

        let app = build_app(mock_repo, MockRedisSync::new());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/v1/config/{tenant_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: serde_json::Value = {
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            serde_json::from_slice(&bytes).unwrap()
        };
        // Verify no raw DB detail is leaked to the client.
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
        assert!(!body["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("connection refused"));
    }

    // ── PUT /v1/config/:tenant_id ─────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_config_returns_200_and_pushes_to_redis() {
        let tenant_id = Uuid::new_v4();
        let saved = fixture_config(tenant_id);
        let saved_clone = saved.clone();

        let mut mock_repo = MockConfigRepository::new();
        // The handler tries to GET first (lazy-init base); return NotFound.
        mock_repo
            .expect_get_config()
            .once()
            .returning(|_| Err(ConfigError::NotFound("no config yet".into())));
        mock_repo
            .expect_upsert_config()
            .once()
            .returning(move |_| Ok(saved_clone.clone()));

        let mut mock_redis = MockRedisSync::new();
        mock_redis
            .expect_push_config_update()
            .once()
            .returning(|_| Ok(()));

        let app = build_app(mock_repo, mock_redis);

        let body = serde_json::json!({
            "pii_masking_enabled": true,
            "semantic_caching_enabled": true,
            "rate_limit_rpm": 1000
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/v1/config/{tenant_id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn upsert_config_returns_422_for_invalid_rpm() {
        let tenant_id = Uuid::new_v4();
        let app = build_app(MockConfigRepository::new(), MockRedisSync::new());

        let body = serde_json::json!({ "rate_limit_rpm": -1 });

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/v1/config/{tenant_id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let resp_body: serde_json::Value = {
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            serde_json::from_slice(&bytes).unwrap()
        };
        assert_eq!(resp_body["error"]["code"], "UNPROCESSABLE_ENTITY");
    }

    #[tokio::test]
    async fn upsert_config_succeeds_even_when_redis_fails() {
        // Redis failure must NOT cause the handler to return an error —
        // the DB write is the authoritative operation.
        let tenant_id = Uuid::new_v4();
        let saved = fixture_config(tenant_id);
        let saved_clone = saved.clone();

        let mut mock_repo = MockConfigRepository::new();
        mock_repo
            .expect_get_config()
            .once()
            .returning(|tid| Ok(fixture_config(tid)));
        mock_repo
            .expect_upsert_config()
            .once()
            .returning(move |_| Ok(saved_clone.clone()));

        let mut mock_redis = MockRedisSync::new();
        mock_redis
            .expect_push_config_update()
            .once()
            .returning(|_| Err(ConfigError::Redis("timeout".into())));

        let app = build_app(mock_repo, mock_redis);

        let body = serde_json::json!({ "pii_masking_enabled": false });

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/v1/config/{tenant_id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Must still return 200 — Postgres is source of truth.
        assert_eq!(response.status(), StatusCode::OK);
    }

    // ── GET /v1/config/:tenant_id/api-keys ───────────────────────────────────

    #[tokio::test]
    async fn list_api_keys_returns_200_with_empty_array_when_no_keys() {
        let tenant_id = Uuid::new_v4();

        let mut mock_repo = MockConfigRepository::new();
        mock_repo
            .expect_list_api_keys()
            .with(eq(tenant_id))
            .once()
            .returning(|_| Ok(vec![]));

        let app = build_app(mock_repo, MockRedisSync::new());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/v1/config/{tenant_id}/api-keys"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value = {
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            serde_json::from_slice(&bytes).unwrap()
        };
        assert!(body.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_api_keys_returns_keys_without_key_hash() {
        let tenant_id = Uuid::new_v4();
        let key = fixture_api_key(tenant_id);
        let key_id = key.id;

        let mut mock_repo = MockConfigRepository::new();
        mock_repo
            .expect_list_api_keys()
            .once()
            .returning(move |_| Ok(vec![key.clone()]));

        let app = build_app(mock_repo, MockRedisSync::new());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/v1/config/{tenant_id}/api-keys"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value = {
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            serde_json::from_slice(&bytes).unwrap()
        };
        let keys = body.as_array().unwrap();
        assert_eq!(keys.len(), 1);
        // key_hash must NOT appear in the response (it is skip_serializing on ApiKeyRecord,
        // but double-check via the response DTO).
        assert!(
            keys[0].get("key_hash").is_none(),
            "key_hash must never be serialised"
        );
        assert_eq!(keys[0]["id"], key_id.to_string());
        assert_eq!(keys[0]["name"], "Production Key");
    }

    // ── DELETE /v1/config/:tenant_id/api-keys/:key_id ────────────────────────

    #[tokio::test]
    async fn revoke_api_key_returns_204_on_success() {
        let tenant_id = Uuid::new_v4();
        let key = fixture_api_key(tenant_id);
        let key_id = key.id;

        let mut mock_repo = MockConfigRepository::new();
        mock_repo
            .expect_revoke_api_key()
            .with(eq(key_id), eq(tenant_id))
            .once()
            .returning(move |_, _| Ok(key.clone()));

        let mut mock_redis = MockRedisSync::new();
        mock_redis
            .expect_invalidate_api_key()
            .once()
            .returning(|_, _, _| Ok(()));

        let app = build_app(mock_repo, mock_redis);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/v1/config/{tenant_id}/api-keys/{key_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn revoke_api_key_returns_404_when_key_not_found() {
        let tenant_id = Uuid::new_v4();
        let key_id = Uuid::new_v4();

        let mut mock_repo = MockConfigRepository::new();
        mock_repo
            .expect_revoke_api_key()
            .once()
            .returning(move |_, _| {
                Err(ConfigError::NotFound(format!(
                    "API key {} not found for this tenant.",
                    key_id
                )))
            });

        let app = build_app(mock_repo, MockRedisSync::new());

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/v1/config/{tenant_id}/api-keys/{key_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn revoke_api_key_returns_204_even_when_redis_invalidation_fails() {
        // After DB hard-delete succeeds, a Redis failure must NOT cause a 5xx.
        // The key is already gone from the authoritative store.
        let tenant_id = Uuid::new_v4();
        let key = fixture_api_key(tenant_id);
        let key_id = key.id;

        let mut mock_repo = MockConfigRepository::new();
        mock_repo
            .expect_revoke_api_key()
            .once()
            .returning(move |_, _| Ok(key.clone()));

        let mut mock_redis = MockRedisSync::new();
        mock_redis
            .expect_invalidate_api_key()
            .once()
            .returning(|_, _, _| Err(ConfigError::Redis("connection lost".into())));

        let app = build_app(mock_repo, mock_redis);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/v1/config/{tenant_id}/api-keys/{key_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Postgres is the source of truth: revocation succeeded.
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    // ── ApiKeyResponse — key_hash exclusion guarantee ────────────────────────

    #[test]
    fn api_key_response_excludes_key_hash() {
        let rec = ApiKeyRecord {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            key_hash: "super_secret_hash".into(),
            name: "My Key".into(),
            created_at: chrono::Utc::now(),
            expires_at: None,
            is_active: true,
        };
        let response = ApiKeyResponse::from(rec);
        let json = serde_json::to_value(&response).unwrap();
        assert!(
            json.get("key_hash").is_none(),
            "key_hash must never appear in ApiKeyResponse JSON"
        );
    }
}
