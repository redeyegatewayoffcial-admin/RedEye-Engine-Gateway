//! api/grpc_server.rs — tonic gRPC server implementation for CacheService.
//!
//! Implements both `LookupCache` and `StoreCache` RPCs by delegating
//! to the existing `SemanticSearchUseCase`. All errors are mapped to
//! typed `tonic::Status` codes — no panics, no unwraps.

use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};

use crate::domain::models::{CacheLookupRequest, CacheStoreRequest};
use crate::usecases::semantic_search::SemanticSearchUseCase;

// Include generated stubs. The `semantic_cache` name matches the proto package.
pub mod proto {
    tonic::include_proto!("semantic_cache");
}

use proto::cache_service_server::CacheService;
use proto::{CacheRequest, CacheResponse, StoreAck, StoreRequest};

// ── Service struct ────────────────────────────────────────────────────────────

/// The live implementation of the CacheService gRPC contract.
pub struct CacheServiceImpl {
    use_case: Arc<SemanticSearchUseCase>,
}

impl CacheServiceImpl {
    pub fn new(use_case: Arc<SemanticSearchUseCase>) -> Self {
        Self { use_case }
    }
}

// ── RPC implementations ───────────────────────────────────────────────────────

#[tonic::async_trait]
impl CacheService for CacheServiceImpl {
    /// Look up a semantic cache entry.
    /// Maps domain results to `CacheResponse { hit, content }`.
    async fn lookup_cache(
        &self,
        request: Request<CacheRequest>,
    ) -> Result<Response<CacheResponse>, Status> {
        let req = request.into_inner();
        debug!(
            tenant_id = %req.tenant_id,
            trace_id  = %req.trace_id,
            "gRPC LookupCache received"
        );

        let domain_req = CacheLookupRequest {
            tenant_id: req.tenant_id.clone(),
            model: req.model.clone(),
            prompt: req.prompt.clone(),
        };

        match self.use_case.check_cache(&domain_req).await {
            Ok(Some(cached)) => {
                info!(tenant_id = %req.tenant_id, "L2 gRPC cache HIT");
                Ok(Response::new(CacheResponse {
                    hit: true,
                    content: cached.content,
                }))
            }
            Ok(None) => {
                debug!(tenant_id = %req.tenant_id, "L2 gRPC cache MISS");
                Ok(Response::new(CacheResponse {
                    hit: false,
                    content: String::new(),
                }))
            }
            Err(e) => {
                error!(error = %e, tenant_id = %req.tenant_id, "LookupCache use-case failed");
                Err(Status::internal(format!("Cache lookup error: {}", e)))
            }
        }
    }

    /// Store a prompt → response pair in the semantic cache.
    /// Returns `StoreAck { stored: true }` on success.
    async fn store_cache(
        &self,
        request: Request<StoreRequest>,
    ) -> Result<Response<StoreAck>, Status> {
        let req = request.into_inner();
        debug!(
            tenant_id = %req.tenant_id,
            trace_id  = %req.trace_id,
            "gRPC StoreCache received"
        );

        let domain_req = CacheStoreRequest {
            tenant_id: req.tenant_id.clone(),
            model: req.model.clone(),
            prompt: req.prompt.clone(),
            response_content: req.response_content.clone(),
        };

        match self.use_case.store_response(&domain_req).await {
            Ok(_) => {
                debug!(tenant_id = %req.tenant_id, "L2 gRPC cache stored");
                Ok(Response::new(StoreAck { stored: true }))
            }
            Err(e) => {
                warn!(error = %e, tenant_id = %req.tenant_id, "StoreCache use-case failed");
                // Non-fatal: return Internal but don't escalate to the gateway.
                Err(Status::internal(format!("Cache store error: {}", e)))
            }
        }
    }
}
