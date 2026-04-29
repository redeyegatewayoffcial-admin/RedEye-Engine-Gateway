use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage, runners::AsyncRunner};
use tower::ServiceExt;

use redeye_tracer::api::handlers::{
    audit_handler, compliance_metrics_handler, ingest_handler, traces_handler,
};
use redeye_tracer::infrastructure::clickhouse_repo::ClickHouseRepo;

pub struct TestEnv {
    pub repo: Arc<ClickHouseRepo>,
    pub app: axum::Router,
    pub clickhouse_url: String,
    pub _clickhouse_node: ContainerAsync<GenericImage>,
}

async fn setup_test_environment() -> TestEnv {
    let clickhouse_image = GenericImage::new("clickhouse/clickhouse-server", "23.8")
        .with_exposed_port(testcontainers::core::ContainerPort::Tcp(8123));

    let clickhouse_node = clickhouse_image
        .start()
        .await
        .expect("Failed to start ClickHouse container");
    let ch_port = clickhouse_node
        .get_host_port_ipv4(8123)
        .await
        .expect("Failed to get Clickhouse port");

    // HTTP interface for ClickHouse default port is 8123
    let clickhouse_url = format!("http://localhost:{}", ch_port);

    // 2. Wrap the ClickHouse HTTP Repo abstraction
    let repo = ClickHouseRepo::new(clickhouse_url.clone());

    // Attempt connecting to Clickhouse container natively with retries
    let client = reqwest::Client::new();
    let mut db_ready = false;
    for _ in 0..20 {
        if client.get(&clickhouse_url).send().await.is_ok() {
            db_ready = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }
    assert!(
        db_ready,
        "Fatal error: Clickhouse testcontainer failed network binding"
    );

    // Manually read and execute the DDL queries from infra/clickhouse/init.sql
    // to strictly respect user instruction and instantiate raw tables
    let raw_ddl =
        std::fs::read_to_string("../infra/clickhouse/init.sql").expect("Missing init.sql");
    for query in raw_ddl.split(';') {
        let q = query.trim();
        if !q.is_empty() {
            client
                .post(&clickhouse_url)
                .body(q.to_string())
                .send()
                .await
                .expect("Failed to execute DDL query manually");
        }
    }

    let shared_repo = Arc::new(repo);

    // 3. Mount Test Router internally tracking state mappings natively
    let app = axum::Router::new()
        .route("/v1/traces/ingest", axum::routing::post(ingest_handler))
        .route("/v1/traces", axum::routing::get(traces_handler))
        .route("/v1/audit", axum::routing::get(audit_handler))
        .route(
            "/v1/compliance/metrics",
            axum::routing::get(compliance_metrics_handler),
        )
        .with_state(shared_repo.clone());

    TestEnv {
        repo: shared_repo,
        app,
        clickhouse_url,
        _clickhouse_node: clickhouse_node,
    }
}

/// Black Box Test: Data ingestion mapping telemetry payloads and asserting DB execution routines
#[tokio::test]
async fn test_ingest_and_assert_clickhouse() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();

    let env = setup_test_environment().await;

    // 1. Dispatch Telemetry ingest payload
    let ingest_payload = json!({
        "trace_id": "00000000-0000-0000-0000-000000000002",
        "session_id": "00000000-0000-0000-0000-000000000003",
        "parent_trace_id": null,
        "tenant_id": "00000000-0000-0000-0000-000000000001",
        "model": "gpt-4",
        "status": 200,
        "latency_ms": 150,
        "total_tokens": 30,
        "cache_hit": false,
        "prompt_content": "hello world",
        "response_content": "hi there"
    });

    let req_ingest = Request::builder()
        .method("POST")
        .uri("/v1/traces/ingest")
        .header("content-type", "application/json")
        .body(Body::from(ingest_payload.to_string()))
        .unwrap();

    let res_ingest = env.app.clone().oneshot(req_ingest).await.unwrap();
    assert_eq!(
        res_ingest.status(),
        StatusCode::OK,
        "Failed to ingest telemetry logs"
    );

    // Clickhouse writes are asynchronous generally but the Repo `ensure_schema` mapped native inserts.
    // Let's assert physical DB rows using raw ClickHouse HTTP execution to bypass the Application Abstraction!
    let client = reqwest::Client::new();
    let query = "SELECT count() FROM RedEye_telemetry.agent_traces WHERE model = 'gpt-4' AND tenant_id = '00000000-0000-0000-0000-000000000001' FORMAT JSON";

    // Explicit 1s wait interval enforcing ClickHouse log insertions buffer queue (Optional, ensures stability)
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    let raw_db_req = client
        .post(&env.clickhouse_url)
        .body(query.to_string())
        .send()
        .await
        .expect("Failed to execute external raw query against ClickHouse Container HTTP wrapper");

    assert!(
        raw_db_req.status().is_success(),
        "Raw DB query failed execution"
    );

    let db_body: serde_json::Value = raw_db_req.json().await.unwrap();

    let count: i32 = db_body["data"][0]["count()"]
        .as_str()
        .unwrap_or("0")
        .parse()
        .unwrap();

    assert_eq!(
        count, 1,
        "Clickhouse failed to persist and return exact ingested metric mapping traces"
    );
}
