//! api/handlers.rs — Axum handlers for the RedEye Tracer microservice.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde_json::json;
use tracing::{error, info};

use crate::domain::models::{AuditQuery, TraceIngestPayload, TraceQuery};
use crate::error::AppError;
use crate::infrastructure::clickhouse_repo::ClickHouseRepo;
use crate::usecases::{ingest, query};

/// POST /v1/traces/ingest
/// Receives trace + audit data from the gateway and writes both to ClickHouse.
pub async fn ingest_handler(
    State(repo): State<Arc<ClickHouseRepo>>,
    Json(payload): Json<TraceIngestPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    info!(trace_id = %payload.trace_id, "Ingest request received");

    ingest::ingest_trace(&repo, &payload).await.map_err(|e| {
        error!(error = %e, "Ingest failed");
        AppError::Internal(format!("Failed to ingest trace: {}", e))
    })?;

    Ok(Json(json!({"ingested": true})))
}

/// GET /v1/traces
/// Query agent traces by session_id.
pub async fn traces_handler(
    State(repo): State<Arc<ClickHouseRepo>>,
    Query(params): Query<TraceQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let session_id = params.session_id.unwrap_or_default();
    let limit = params.limit.unwrap_or(50);

    if session_id.is_empty() {
        return Err(AppError::BadRequest("session_id query parameter is required".into()));
    }

    let data = query::query_traces_by_session(&repo, &session_id, limit).await.map_err(|e| {
        error!(error = %e, "Trace query failed");
        AppError::Internal(format!("Failed to query traces: {}", e))
    })?;

    Ok(Json(data))
}

/// GET /v1/audit
/// Query compliance audit log by tenant_id.
pub async fn audit_handler(
    State(repo): State<Arc<ClickHouseRepo>>,
    Query(params): Query<AuditQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tenant_id = params.tenant_id.unwrap_or_default();
    let limit = params.limit.unwrap_or(50);

    if tenant_id.is_empty() {
        return Err(AppError::BadRequest("tenant_id query parameter is required".into()));
    }

    let data = query::query_audit_by_tenant(&repo, &tenant_id, limit).await.map_err(|e| {
        error!(error = %e, "Audit query failed");
        AppError::Internal(format!("Failed to query audit: {}", e))
    })?;

    Ok(Json(data))
}
