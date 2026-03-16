//! Query use case — retrieves traces and audit entries from ClickHouse.

use serde_json::Value;
use tracing::info;

use crate::infrastructure::clickhouse_repo::ClickHouseRepo;

pub async fn query_traces_by_session(
    repo: &ClickHouseRepo,
    session_id: &str,
    limit: u32,
) -> Result<Value, String> {
    info!(session_id = %session_id, "Querying agent traces");
    repo.query_traces(session_id, limit).await
}

pub async fn query_audit_by_tenant(
    repo: &ClickHouseRepo,
    tenant_id: &str,
    limit: u32,
) -> Result<Value, String> {
    info!(tenant_id = %tenant_id, "Querying compliance audit log");
    repo.query_audit(tenant_id, limit).await
}
