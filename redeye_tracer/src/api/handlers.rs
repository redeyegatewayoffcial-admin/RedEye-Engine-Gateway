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

// ── Compliance Metrics ───────────────────────────────────────────────────────

/// Aggregated compliance metrics returned by `GET /v1/compliance/metrics`.
#[derive(Debug, serde::Serialize)]
pub struct ComplianceMetricsResponse {
    /// Total number of prompts that passed through the compliance engine.
    pub total_scanned: u64,
    /// Number of requests blocked due to DPDP geo-routing violations.
    pub dpdp_blocks: u64,
    /// Number of PII entities redacted (Aadhaar, PAN, SSN, email, etc.).
    pub pii_redactions: u64,
    /// Breakdown of detections by region.
    pub region_breakdown: Vec<RegionCount>,
}

/// Per-region detection count.
#[derive(Debug, serde::Serialize)]
pub struct RegionCount {
    pub region: String,
    pub count: u64,
}

/// GET /v1/compliance/metrics
/// Returns aggregated DPDP compliance statistics from ClickHouse.
///
/// Zero `.unwrap()` — all ClickHouse errors are mapped to `AppError::Internal`.
pub async fn compliance_metrics_handler(
    State(repo): State<Arc<ClickHouseRepo>>,
) -> Result<Json<ComplianceMetricsResponse>, AppError> {
    // Query 1: Total scanned
    let total_query = r#"
        SELECT count() AS total
        FROM RedEye_telemetry.compliance_engine_metrics
        FORMAT JSON
    "#;

    let total_scanned = repo
        .raw_query(total_query)
        .await
        .map(|v| parse_u64_field(&v, "total"))
        .unwrap_or(0);

    // Query 2: DPDP geo-blocks
    let blocks_query = r#"
        SELECT count() AS total
        FROM RedEye_telemetry.compliance_engine_metrics
        WHERE compliance_action = 'GEO_BLOCKED'
        FORMAT JSON
    "#;

    let dpdp_blocks = repo
        .raw_query(blocks_query)
        .await
        .map(|v| parse_u64_field(&v, "total"))
        .unwrap_or(0);

    // Query 3: PII redactions
    let redactions_query = r#"
        SELECT sum(entity_count) AS total
        FROM RedEye_telemetry.compliance_engine_metrics
        WHERE compliance_action = 'PII_MASKED'
        FORMAT JSON
    "#;

    let pii_redactions = repo
        .raw_query(redactions_query)
        .await
        .map(|v| parse_u64_field(&v, "total"))
        .unwrap_or(0);

    // Query 4: Region breakdown
    let region_query = r#"
        SELECT detected_region, count() AS cnt
        FROM RedEye_telemetry.compliance_engine_metrics
        WHERE detected_region != ''
        GROUP BY detected_region
        ORDER BY cnt DESC
        FORMAT JSON
    "#;

    let region_breakdown = repo
        .raw_query(region_query)
        .await
        .map(|v| parse_region_breakdown(&v))
        .unwrap_or_default();

    info!(
        total_scanned,
        dpdp_blocks,
        pii_redactions,
        "Compliance metrics query complete"
    );

    Ok(Json(ComplianceMetricsResponse {
        total_scanned,
        dpdp_blocks,
        pii_redactions,
        region_breakdown,
    }))
}

/// Safely extracts a u64 value from a ClickHouse JSON response `data[0][field]`.
fn parse_u64_field(value: &serde_json::Value, field: &str) -> u64 {
    value
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(|row| row.get(field))
        .and_then(|v| match v {
            serde_json::Value::String(s) => s.parse().ok(),
            serde_json::Value::Number(n) => n.as_u64(),
            _ => None,
        })
        .unwrap_or(0)
}

/// Parses the region breakdown from a ClickHouse JSON response.
fn parse_region_breakdown(value: &serde_json::Value) -> Vec<RegionCount> {
    value
        .get("data")
        .and_then(|d| d.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let region = row.get("detected_region")?.as_str()?.to_string();
                    let count = match row.get("cnt")? {
                        serde_json::Value::String(s) => s.parse().ok()?,
                        serde_json::Value::Number(n) => n.as_u64()?,
                        _ => return None,
                    };
                    Some(RegionCount { region, count })
                })
                .collect()
        })
        .unwrap_or_default()
}

