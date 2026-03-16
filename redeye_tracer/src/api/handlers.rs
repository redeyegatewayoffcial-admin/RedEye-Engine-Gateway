//! api/handlers.rs — Axum handlers for the RedEye Tracer microservice.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use tracing::{error, info};

use crate::domain::models::{AuditQuery, TraceIngestPayload, TraceQuery};
use crate::infrastructure::clickhouse_repo::ClickHouseRepo;
use crate::usecases::{ingest, query};

/// POST /v1/traces/ingest
/// Receives trace + audit data from the gateway and writes both to ClickHouse.
pub async fn ingest_handler(
    State(repo): State<Arc<ClickHouseRepo>>,
    Json(payload): Json<TraceIngestPayload>,
) -> impl IntoResponse {
    info!(trace_id = %payload.trace_id, "Ingest request received");

    match ingest::ingest_trace(&repo, &payload).await {
        Ok(()) => (StatusCode::CREATED, Json(json!({"ingested": true}))),
        Err(e) => {
            error!(error = %e, "Ingest failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e})))
        }
    }
}

/// GET /v1/traces
/// Query agent traces by session_id.
pub async fn traces_handler(
    State(repo): State<Arc<ClickHouseRepo>>,
    Query(params): Query<TraceQuery>,
) -> impl IntoResponse {
    let session_id = params.session_id.unwrap_or_default();
    let limit = params.limit.unwrap_or(50);

    if session_id.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "session_id query parameter is required"}))).into_response();
    }

    match query::query_traces_by_session(&repo, &session_id, limit).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "Trace query failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))).into_response()
        }
    }
}

/// GET /v1/audit
/// Query compliance audit log by tenant_id.
pub async fn audit_handler(
    State(repo): State<Arc<ClickHouseRepo>>,
    Query(params): Query<AuditQuery>,
) -> impl IntoResponse {
    let tenant_id = params.tenant_id.unwrap_or_default();
    let limit = params.limit.unwrap_or(50);

    if tenant_id.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "tenant_id query parameter is required"}))).into_response();
    }

    match query::query_audit_by_tenant(&repo, &tenant_id, limit).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "Audit query failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))).into_response()
        }
    }
}
