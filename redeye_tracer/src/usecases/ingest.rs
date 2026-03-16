//! Ingest use case — receives trace + audit data and writes to ClickHouse.

use tracing::info;

use crate::domain::models::TraceIngestPayload;
use crate::infrastructure::clickhouse_repo::ClickHouseRepo;

pub async fn ingest_trace(
    repo: &ClickHouseRepo,
    payload: &TraceIngestPayload,
) -> Result<(), String> {
    info!(
        trace_id = %payload.trace_id,
        session_id = %payload.session_id,
        tenant_id = %payload.tenant_id,
        "Ingesting trace + audit"
    );

    // Write to agent_traces
    repo.insert_trace(payload).await?;

    // Write to compliance_audit_log
    repo.insert_audit(payload).await?;

    info!("Trace and audit ingested successfully");
    Ok(())
}
