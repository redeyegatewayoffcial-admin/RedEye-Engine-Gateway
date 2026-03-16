use reqwest::Client;
use serde_json::{json, Value};
use tracing::{debug, error, info};

use crate::domain::models::TraceIngestPayload;

#[derive(Clone)]
pub struct ClickHouseRepo {
    client: Client,
    url: String,
}

impl ClickHouseRepo {
    pub fn new(url: String) -> Self {
        Self {
            client: Client::new(),
            url,
        }
    }

    /// Ensures the ClickHouse tables exist (idempotent DDL).
    pub async fn ensure_schema(&self) -> Result<(), String> {
    let statements = vec![
        "CREATE DATABASE IF NOT EXISTS RedEye_telemetry;",
        r#"
            CREATE TABLE IF NOT EXISTS RedEye_telemetry.agent_traces (
                trace_id UUID, session_id UUID, parent_trace_id Nullable(UUID),
                tenant_id String, model String, status UInt16, latency_ms UInt32,
                prompt_tokens UInt32 DEFAULT 0, completion_tokens UInt32 DEFAULT 0,
                total_tokens UInt32 DEFAULT 0, cache_hit Bool DEFAULT false,
                created_at DateTime DEFAULT now()
            ) ENGINE = MergeTree ORDER BY (session_id, created_at)
            TTL created_at + INTERVAL 180 DAY;
        "#,
        r#"
            CREATE TABLE IF NOT EXISTS RedEye_telemetry.compliance_audit_log (
                trace_id UUID, session_id UUID, tenant_id String,
                prompt_content String, response_content String, model String,
                flagged Bool DEFAULT false, flag_reason String DEFAULT '',
                created_at DateTime DEFAULT now()
            ) ENGINE = MergeTree ORDER BY (tenant_id, created_at)
            TTL created_at + INTERVAL 365 DAY;
        "#,
    ];

    for stmt in statements {
        let res = self.client
            .post(&self.url)
            .body(stmt)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            error!("ClickHouse schema DDL failed: {}", err);
            return Err(err);
        }
    }

    info!("ClickHouse schema verified (agent_traces + compliance_audit_log)");
    Ok(())
}

    /// Inserts a trace row into `agent_traces`.
    pub async fn insert_trace(&self, payload: &TraceIngestPayload) -> Result<(), String> {
        let row = json!({
            "trace_id": payload.trace_id,
            "session_id": payload.session_id,
            "parent_trace_id": payload.parent_trace_id,
            "tenant_id": payload.tenant_id,
            "model": payload.model,
            "status": payload.status,
            "latency_ms": payload.latency_ms,
            "total_tokens": payload.total_tokens,
            "cache_hit": payload.cache_hit
        });

        self.insert_row("RedEye_telemetry.agent_traces", &row).await
    }

    /// Inserts an audit row into `compliance_audit_log`.
    pub async fn insert_audit(&self, payload: &TraceIngestPayload) -> Result<(), String> {
        let row = json!({
            "trace_id": payload.trace_id,
            "session_id": payload.session_id,
            "tenant_id": payload.tenant_id,
            "prompt_content": payload.prompt_content,
            "response_content": payload.response_content,
            "model": payload.model
        });

        self.insert_row("RedEye_telemetry.compliance_audit_log", &row).await
    }

    /// Queries traces by session_id.
    pub async fn query_traces(&self, session_id: &str, limit: u32) -> Result<Value, String> {
        let query = format!(
            "SELECT * FROM RedEye_telemetry.agent_traces WHERE session_id = '{}' ORDER BY created_at DESC LIMIT {} FORMAT JSON",
            session_id, limit
        );
        self.run_query(&query).await
    }

    /// Queries audit log by tenant_id.
    pub async fn query_audit(&self, tenant_id: &str, limit: u32) -> Result<Value, String> {
        let query = format!(
            "SELECT * FROM RedEye_telemetry.compliance_audit_log WHERE tenant_id = '{}' ORDER BY created_at DESC LIMIT {} FORMAT JSON",
            tenant_id, limit
        );
        self.run_query(&query).await
    }

    fn build_url(&self, extra: &str) -> String {
        let sep = if self.url.contains('?') { "&" } else { "?" };
        format!("{}{}{}", self.url, sep, extra)
    }

    async fn insert_row(&self, table: &str, row: &Value) -> Result<(), String> {
        let url = self.build_url(&format!("query=INSERT INTO {} FORMAT JSONEachRow", table));
        let res = self.client.post(&url).json(row).send().await.map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            error!("ClickHouse insert to {} failed: {}", table, err);
            return Err(err);
        }
        debug!("Inserted row into {}", table);
        Ok(())
    }

    async fn run_query(&self, query: &str) -> Result<Value, String> {
        let url = &self.url;
        let res = self.client.post(url).body(query.to_string()).send().await.map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(err);
        }

        res.json::<Value>().await.map_err(|e| e.to_string())
    }
}
