use reqwest::Client;
use serde_json::{json, Value};
use tracing::{debug, error};

/// Fires an async ClickHouse JSONEachRow insert for request telemetry.
pub async fn log_request(
    client: &reqwest::Client,
    clickhouse_url: &str,
    tenant_id: &str,
    _trace_id: &str,
    _session_id: &str,
    status_code: u16,
    latency_ms: u32,
    model: &str,
    tokens: u32,
    _cache_hit: bool,
) {
    let log_entry = json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "tenant_id": tenant_id,
        "status": status_code,
        "latency_ms": latency_ms,
        "model": model,
        "tokens": tokens
    });

    let result = client
        .post(format!("{}/?query=INSERT INTO RedEye_telemetry.request_logs FORMAT JSONEachRow", clickhouse_url))
        .json(&log_entry)
        .send()
        .await;

    match result {
        Ok(r) if !r.status().is_success() => {
            let err_text = r.text().await.unwrap_or_default();
            error!(error = ?err_text, "ClickHouse insertion rejected");
        }
        Err(e) => {
            error!(error = %e, "ClickHouse network failure during async log");
        }
        _ => {
            debug!("Successfully wrote async telemetry row to ClickHouse");
        }
    }
}

/// Sends trace + audit data to the redeye_tracer microservice.
pub async fn send_trace_to_tracer(
    client: &Client,
    tracer_url: &str,
    payload: &Value,
) {
    let url = format!("{}/v1/traces/ingest", tracer_url);
    let result = client.post(&url).json(payload).send().await;

    match result {
        Ok(r) if !r.status().is_success() => {
            let err = r.text().await.unwrap_or_default();
            error!(error = ?err, "Tracer ingestion rejected");
        }
        Err(e) => {
            error!(error = %e, "Tracer network failure");
        }
        _ => {
            debug!("Trace payload dispatched to redeye_tracer");
        }
    }
}
