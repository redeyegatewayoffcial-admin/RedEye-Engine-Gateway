//! infrastructure/cache_client.rs — gRPC client for the L2 semantic cache.
//!
//! ## Design
//! - A single `tonic::transport::Channel` is created at startup and shared via
//!   `Arc<Mutex<CacheServiceClient<Channel>>>` so all requests reuse the same
//!   multiplexed H2 connection (no per-request TCP handshake).
//! - **Fail-Open**: every error path returns `None` / unit so the gateway can
//!   continue to the upstream LLM without crashing.
//! - **Timeout on Store**: `store_in_cache` enforces a 2-second deadline via
//!   `tokio::time::timeout`; the future is silently dropped on expiry.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tonic::transport::Channel;
use tonic::{Request, Status};
use tracing::{debug, warn};

use crate::domain::models::TraceContext;

// Include the generated tonic/prost stubs.
// `semantic_cache` matches the `package` directive in the .proto file.
pub mod proto {
    tonic::include_proto!("semantic_cache");
}

use proto::cache_service_client::CacheServiceClient;
use proto::{CacheRequest, StoreRequest};

/// Shared, lazily-pooled gRPC channel to `redeye_cache`.
/// Clone is cheap — `Channel` uses Arc internally.
#[derive(Clone)]
pub struct CacheGrpcClient {
    inner: Arc<Mutex<CacheServiceClient<Channel>>>,
}

impl CacheGrpcClient {
    /// Build the client from a pre-established `Channel`.
    /// Call once at startup and inject into `AppState`.
    pub fn new(channel: Channel) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CacheServiceClient::new(channel))),
        }
    }
}

// ── Public API (mirrors old `cache_client` signatures) ───────────────────────

/// Look up a prompt in the L2 semantic cache over gRPC.
///
/// Returns `Some(content)` on a cache hit, `None` on a miss **or any error**.
/// Errors are logged as warnings but never propagated — fail-open semantics.
pub async fn lookup_cache(
    client: &CacheGrpcClient,
    tenant_id: &str,
    model: &str,
    raw_prompt: &str,
    trace_ctx: &TraceContext,
) -> Option<String> {
    let request = Request::new(CacheRequest {
        tenant_id: tenant_id.to_string(),
        model: model.to_string(),
        prompt: raw_prompt.to_string(),
        trace_id: trace_ctx.trace_id.clone(),
        session_id: trace_ctx.session_id.clone(),
    });

    let response = {
        let mut guard = client.inner.lock().await;
        guard.lookup_cache(request).await
    };

    match response {
        Ok(resp) => {
            let msg = resp.into_inner();
            if msg.hit {
                debug!("L2 gRPC Cache HIT");
                Some(msg.content)
            } else {
                debug!("L2 gRPC Cache MISS");
                None
            }
        }
        Err(status) => {
            warn!(
                code = ?status.code(),
                message = %status.message(),
                "L2 gRPC cache lookup failed — proceeding fail-open"
            );
            None
        }
    }
}

/// Store a prompt → response pair in the L2 semantic cache over gRPC.
///
/// Enforces a **2-second deadline**. If the cache server is slow or unreachable
/// the future is dropped and a warning is logged. Never blocks the hot path.
pub async fn store_in_cache(
    client: &CacheGrpcClient,
    tenant_id: &str,
    model: &str,
    raw_prompt: &str,
    response_content: &str,
    trace_ctx: &TraceContext,
) {
    let request = Request::new(StoreRequest {
        tenant_id: tenant_id.to_string(),
        model: model.to_string(),
        prompt: raw_prompt.to_string(),
        response_content: response_content.to_string(),
        trace_id: trace_ctx.trace_id.clone(),
        session_id: trace_ctx.session_id.clone(),
    });

    let store_future = async {
        let mut guard = client.inner.lock().await;
        guard.store_cache(request).await
    };

    match tokio::time::timeout(Duration::from_secs(2), store_future).await {
        Ok(Ok(_)) => debug!("L2 gRPC cache store acknowledged"),
        Ok(Err(status)) => warn!(
            code = ?status.code(),
            message = %status.message(),
            "L2 gRPC cache store RPC failed — dropping silently"
        ),
        Err(_elapsed) => warn!("L2 gRPC cache store timed out after 2s — dropping silently"),
    }
}
