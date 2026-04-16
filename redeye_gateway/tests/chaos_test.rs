//! Chaos Engineering & Resilience Tests for redeye_gateway.
//!
//! These tests simulate critical infrastructure failures (Redis crashes, LLM Timeouts)
//! and verify the gateway's ability to gracefully degrade or hot-swap without
//! returning unhandled 500 errors to the client.

use std::net::SocketAddr;
use std::sync::Arc;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::{postgres::Postgres, redis::Redis};
use tower::ServiceExt; // Provides `.oneshot()`
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use uuid::Uuid;

use redeye_gateway::api::middleware::auth::Claims;
use redeye_gateway::api::routes::create_router;
use redeye_gateway::domain::models::AppState;
use redeye_gateway::infrastructure::cache_client::CacheGrpcClient;

/// Helper: encrypts a dummy plaintext api key (matches AES logic in auth).
fn encrypt_test_api_key(plaintext: &str, master_key: &str) -> Vec<u8> {
    use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};
    let key = Key::<Aes256Gcm>::from_slice(master_key.as_bytes());
    let cipher = Aes256Gcm::new(key);
    let nonce_bytes = [0u8; 12];
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).unwrap();
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    result
}

/// Helper: generates a valid JWT.
fn generate_test_jwt(tenant_id: &str) -> String {
    let claims = Claims {
        sub: "chaos_user".to_string(),
        tenant_id: tenant_id.to_string(),
        exp: (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 3600) as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(b"secret")).unwrap()
}

pub struct ChaosTestEnv {
    pub state: Arc<AppState>,
    pub mock_server: MockServer,
    pub postgres_node: ContainerAsync<Postgres>,
    // Kept as an Option so we can explicitly drop/kill it during the test to simulate crashes.
    pub redis_node: Option<ContainerAsync<Redis>>,
    pub tenant_id: String,
}

impl ChaosTestEnv {
    /// Builds the environment with configurable HTTP client timeouts
    async fn setup(client_timeout_secs: u64) -> Self {
        std::env::set_var("AES_MASTER_KEY", "01234567890123456789012345678901");
        std::env::set_var("JWT_SECRET", "secret");

        let redis_node = Redis::default().start().await.expect("Failed to start Redis");
        let postgres_node = Postgres::default().start().await.expect("Failed to start Postgres");

        let redis_port = redis_node.get_host_port_ipv4(6379).await.expect("Failed to get Redis port");
        let pg_port = postgres_node.get_host_port_ipv4(5432).await.expect("Failed to get Postgres port");

        let redis_url = format!("redis://127.0.0.1:{}", redis_port);
        let db_url = format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", pg_port);

        let redis_conn = redis::Client::open(redis_url)
            .expect("Failed to create Redis client")
            .get_multiplexed_tokio_connection()
            .await
            .expect("Failed to connect to Redis");

        let db_pool = sqlx::PgPool::connect(&db_url).await.expect("Failed to connect to Postgres");

        sqlx::query("CREATE EXTENSION IF NOT EXISTS \"pgcrypto\"").execute(&db_pool).await.unwrap();
        sqlx::query("CREATE TABLE tenants (id UUID PRIMARY KEY DEFAULT gen_random_uuid())").execute(&db_pool).await.unwrap();
        sqlx::query("CREATE TABLE api_keys (tenant_id UUID, key_hash TEXT, is_active BOOLEAN)").execute(&db_pool).await.unwrap();
        sqlx::query("CREATE TABLE provider_keys (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), tenant_id UUID, provider_name TEXT, encrypted_key BYTEA, created_at TIMESTAMPTZ DEFAULT NOW(), UNIQUE(tenant_id, provider_name))").execute(&db_pool).await.unwrap();

        let tenant_id = Uuid::new_v4().to_string();
        let encrypted_key = encrypt_test_api_key("sk-mock-openai-key-123", "01234567890123456789012345678901");

        sqlx::query("INSERT INTO provider_keys (tenant_id, provider_name, encrypted_key) VALUES ($1, $2, $3)")
            .bind(Uuid::parse_str(&tenant_id).unwrap())
            .bind("openai")
            .bind(&encrypted_key)
            .execute(&db_pool)
            .await
            .unwrap();

        let mock_server = MockServer::start().await;

        // Base Mock: Compliance succeeds quickly
        Mock::given(method("POST"))
            .and(path("/api/v1/compliance/redact"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sanitized_payload": {
                    "model": "gpt-4o",
                    "messages": [{"role": "user", "content": "Chaos input"}]
                }
            })))
            .mount(&mock_server)
            .await;

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(client_timeout_secs))
            .build()
            .unwrap();

        let (telemetry_tx, _telemetry_rx) = tokio::sync::mpsc::channel(100);

        let cache_grpc_client = {
            let channel = tonic::transport::Channel::from_static("http://127.0.0.1:1")
                .connect_lazy();
            CacheGrpcClient::new(channel)
        };

        let state = Arc::new(AppState {
            http_client,
            cache_grpc_client,
            compliance_url: mock_server.uri(),
            redis_conn,
            db_pool,
            rate_limit_max: 100,
            rate_limit_window: 60,
            clickhouse_url: "http://localhost:8123".to_string(),
            tracer_url: mock_server.uri(),
            dashboard_url: "http://localhost:3000".to_string(),
            llm_api_base_url: Some(mock_server.uri()),
            telemetry_tx,
            l1_cache: Arc::new(redeye_gateway::infrastructure::l1_cache::L1Cache::new(1024 * 1024).unwrap()),
        });

        Self {
            state,
            mock_server,
            postgres_node,
            redis_node: Some(redis_node),
            tenant_id,
        }
    }
}

/// Scenario 1: Ensure Redis crashes don't crash the gateway.
/// The gateway should bypass the cache and hit the LLM provider,
/// returning 200 OK.
#[tokio::test]
async fn test_redis_crash_resilience() {
    let mut env = ChaosTestEnv::setup(5).await;
    
    // Mock the upstream LLM to succeed, so we know it gracefully fell back to it.
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "Succeeded despite missing Redis!"}}]
        })))
        .mount(&env.mock_server)
        .await;

    // FORCE KILL REDIS by dropping the container handle. Testcontainers stops the container.
    env.redis_node.take();

    let app = create_router(env.state.clone());
    let jwt = generate_test_jwt(&env.tenant_id);

    let mut request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", jwt))
        .body(Body::from(
            serde_json::json!({"model": "gpt-4o", "messages": [{"role": "user", "content": "Chaos input"}]}).to_string(),
        ))
        .unwrap();

    // Mock ConnectInfo for the Rate Limiter which usually reads IP from it.
    request.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 8080))));

    let response = app.oneshot(request).await.expect("Router returned error");

    // ASSERT: Should survive Redis loss and receive the LLM response successfully.
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Gateway failed to bypass cache when Redis was unavailable"
    );
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(resp_json["choices"][0]["message"]["content"], "Succeeded despite missing Redis!");
}

/// Scenario 2: Upstream returns slowly, ensure Gateway respects its timeout,
/// failing gracefully (e.g., Timeout or GatewayTimeout error), rather than hanging.
#[tokio::test]
async fn test_llm_provider_timeout() {
    // Setup Gateway with a very short 2-second timeout.
    let env = ChaosTestEnv::setup(2).await;

    // Wiremock will inject a 5-second delay to force the timeout.
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"status": "Too slow!"}))
                .set_delay(std::time::Duration::from_secs(5)), // 5s delay > 2s timeout
        )
        .mount(&env.mock_server)
        .await;

    let app = create_router(env.state.clone());
    let jwt = generate_test_jwt(&env.tenant_id);

    let mut request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", jwt))
        .body(Body::from(
            serde_json::json!({"model": "gpt-4o", "messages": [{"role": "user", "content": "Chaos input"}]}).to_string(),
        ))
        .unwrap();

    request.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 8080))));

    let response = app.oneshot(request).await.expect("Router returned error");

    // ASSERT: Should return 504 Gateway Timeout or 503 instead of hanging or 500 Internal.
    // The codebase might map a reqwest timeout to 503 or 504. We ensure it's not a success and not an unhandled 500.
    let status = response.status();
    assert!(
        status == StatusCode::GATEWAY_TIMEOUT || status == StatusCode::SERVICE_UNAVAILABLE,
        "Expected timeout error (504 or 503), but got {}",
        status
    );
}

/// Scenario 3: OpenAI returns HTTP 500. Gateway intercepts, avoids returning 500 to user,
/// Hot-Swaps to Anthropic, and returns 200 via Anthropic.
#[tokio::test]
async fn test_hot_swap_on_critical_failure() {
    let env = ChaosTestEnv::setup(5).await;
    
    // Add Anthropic key to DB for Hot-Swap
    let encrypted_anthropic_key = encrypt_test_api_key("sk-mock-anthropic-key-456", "01234567890123456789012345678901");
    sqlx::query("INSERT INTO provider_keys (tenant_id, provider_name, encrypted_key) VALUES ($1, $2, $3)")
        .bind(Uuid::parse_str(&env.tenant_id).unwrap())
        .bind("anthropic")
        .bind(&encrypted_anthropic_key)
        .execute(&env.state.db_pool)
        .await
        .unwrap();

    // Primary Mock: OpenAI crashes and returns HTTP 500 (Critical failure)
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Critical Error at OpenAI"))
        .mount(&env.mock_server)
        .await;

    // Secondary Mock: Anthropic successfully fulfills the Hot-Swapped request
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
             "id": "msg_01",
             "type": "message",
             "role": "assistant",
             "content": [
                 {
                     "type": "text",
                     "text": "Hot-Swap Successful! Anthropic here."
                 }
             ],
             "model": "claude-3-opus-20240229",
             "stop_reason": "end_turn",
             "usage": { "input_tokens": 10, "output_tokens": 15 }
         })))
        .mount(&env.mock_server)
        .await;

    let app = create_router(env.state.clone());
    let jwt = generate_test_jwt(&env.tenant_id);

    let mut request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", jwt))
        .body(Body::from(
            serde_json::json!({"model": "gpt-4o", "messages": [{"role": "user", "content": "Chaos input"}]}).to_string(),
        ))
        .unwrap();

    request.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 8080))));

    let response = app.oneshot(request).await.expect("Router returned error");

    // ASSERT: Client gracefully receives a 200 OK because the gateway abstracted the 500 failure away.
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Expected Hot-Swap to recover with a 200 OK after OpenAI 500"
    );
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .or_else(|| resp_json["content"][0]["text"].as_str())
        .expect("Expected content field in response");
        
    assert!(
        content.contains("Anthropic here"),
        "Expected the translated fallback response from Anthropic, got: {}",
        content
    );
}
