//! infrastructure/clickhouse.rs — Phase 8 Step 5 Async Audit Logger.
//!
//! Non-blocking ClickHouse HTTP client for writing append-only `ComplianceAuditRecord`s.
//! Ensures logging NEVER stalls the primary runtime request path.

use reqwest::Client;
use std::time::Duration;
use tracing::{error, info};

use crate::domain::models::ComplianceAuditRecord;

pub struct ClickHouseLogger {
    http_client: Client,
    clickhouse_url: String,
}

impl ClickHouseLogger {
    pub fn new(url: String) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_millis(500)) // Generous timeout for background tasks
            .build()
            .expect("Failed to build ClickHouse client");

        Self {
            http_client,
            clickhouse_url: url,
        }
    }

    /// Appends an audit record to ClickHouse.
    /// Spawns as a background Tokio task to NEVER block the gateway request.
    pub async fn log_audit_event(&self, record: ComplianceAuditRecord) {
        let url = format!(
            "{}/?query=INSERT INTO RedEye_telemetry.compliance_engine_audit FORMAT JSONEachRow",
            self.clickhouse_url
        );
        let client = self.http_client.clone();

        // Push logging completely to the background
        tokio::spawn(async move {
            match client.post(&url).json(&record).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        let err_text = resp.text().await.unwrap_or_default();
                        error!(
                            "Failed to write compliance audit to ClickHouse: Status {}, {}",
                            status, err_text
                        );
                    } else {
                        info!(
                            "✅ Compliance audit record committed to ClickHouse (Trace: {})",
                            record.trace_id
                        );
                    }
                }
                Err(e) => {
                    error!("ClickHouse telemetry endpoint unreachable: {}", e);
                }
            }
        });
    }
}
