//! `redeye_config` — Control-plane microservice for the RedEye AI Gateway.
//!
//! # Responsibilities
//!
//! * Manage per-tenant feature toggles (PII masking, semantic caching,
//!   routing fallback) via a REST API backed by Postgres.
//! * Provide lifecycle management for virtual API keys (list, revoke).
//! * Synchronise every state change to Redis in real time so that the
//!   `redeye_gateway` can react with sub-millisecond latency via its
//!   Redis-backed config cache and Pub/Sub subscriber.
//!
//! # Crate structure
//!
//! ```text
//! src/
//! ├── error.rs              — ConfigError + IntoResponse mapping
//! ├── lib.rs                — AppState + module declarations
//! ├── main.rs               — Tokio entry-point
//! ├── api/
//! │   ├── handlers.rs       — Axum route handlers (+ unit tests)
//! │   └── router.rs         — Router factory
//! ├── domain/
//! │   └── models.rs         — ClientConfig, ApiKeyRecord, event payloads
//! └── infrastructure/
//!     ├── db.rs             — ConfigRepository trait + PgConfigRepository
//!     └── redis_sync.rs     — RedisSync trait + RedisSyncClient
//! ```

pub mod api;
pub mod domain;
pub mod error;
pub mod infrastructure;

use std::sync::Arc;

use infrastructure::{db::ConfigRepository, redis_sync::RedisSync};

// =============================================================================
// Shared application state
// =============================================================================

/// Shared, cheaply-cloneable state injected into every Axum handler via
/// [`axum::extract::State`].
///
/// Both `repo` and `redis` are trait objects behind [`Arc`], which enables handler
/// unit tests to substitute lightweight [`mockall`] mocks without touching
/// any I/O layer.  `redis_client` is the raw client used for the standalone
/// `publish_routing_mesh` free function (which opens its own connection).
///
/// # Clone semantics
///
/// [`Arc`] clone increments the reference count only — no heap allocation.
/// Axum clones the state once per worker thread on startup, so the cost is
/// effectively zero.
#[derive(Clone)]
pub struct AppState {
    /// Postgres-backed configuration repository.
    pub repo: Arc<dyn ConfigRepository>,

    /// Redis synchronisation client (cache write + Pub/Sub for config updates).
    pub redis: Arc<dyn RedisSync>,

    /// Raw Redis client used by `publish_routing_mesh()` to open its own
    /// multiplexed connection for routing table Pub/Sub.
    pub redis_client: redis::Client,
}
