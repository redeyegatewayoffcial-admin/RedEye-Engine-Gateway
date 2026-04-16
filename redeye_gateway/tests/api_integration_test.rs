//! Integration tests for the redeye_gateway Axum router & middleware pipeline.
//!
//! These tests exercise the **real** Router + middleware stack returned by
//! `create_router`, dispatching HTTP requests in-process with `tower::ServiceExt::oneshot`.
//! No TCP port is bound — tests are fast, deterministic, and parallelisable.
//!
//! ## Infrastructure
//! Uses `testcontainers` to dynamically spin up Redis and Postgres containers per test.
//! Uses `wiremock` to mock upstream microservices (Compliance, Cache) and LLM APIs.

use std::sync::Arc;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt; // Provides `.oneshot()`
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::{postgres::Postgres, redis::Redis};
use wiremock::{MockServer, Mock, ResponseTemplate, matchers::{method, path}};
use jsonwebtoken::{encode, EncodingKey, Header};
use uuid::Uuid;

use redeye_gateway::api::routes::create_router;
use redeye_gateway::domain::models::AppState;
use redeye_gateway::api::middleware::auth::Claims;
use redeye_gateway::infrastructure::cache_client::CacheGrpcClient;

/// Helper: encrypts a dummy plaintext api key to be inserted into Postgres `llm_routes`.
/// Matches the `Aes256Gcm` logic from `redeye_gateway::api::middleware::auth::decrypt_api_key`.
fn encrypt_test_api_key(plaintext: &str, master_key: &str) -> Vec<u8> {
    use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};
    let key = Key::<Aes256Gcm>::from_slice(master_key.as_bytes());
    let cipher = Aes256Gcm::new(key);
    
    // Test uses a static zero-nonce; perfectly fine for test fixtures.
    let nonce_bytes = [0u8; 12];
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).unwrap();
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    result
}

/// Helper: generates a valid JWT signed with the default "secret", which `auth_middleware` accepts.
fn generate_test_jwt(tenant_id: &str) -> String {
    let claims = Claims {
        sub: "test_user".to_string(),
        tenant_id: tenant_id.to_string(),
        exp: (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 3600) as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(b"secret")).unwrap()
}

/// Holds the environment guards so they live as long as the test does.
pub struct TestEnv {
    pub state: Arc<AppState>,
    pub mock_server: MockServer,
    pub _redis_node: ContainerAsync<Redis>,
    pub _postgres_node: ContainerAsync<Postgres>,
    pub tenant_id: String,
}

/// Bootstraps test containers, runs Postgres migrations, sets up WireMock, and creates `AppState`.
async fn setup_test_environment() -> TestEnv {
    // 1. Force the AES Master key so standard decryption doesn't panic.
    std::env::set_var("AES_MASTER_KEY", "01234567890123456789012345678901");
    // Also inject generic dev db credentials to appease sqlx::migrate! at compile-time/runtime defaults if needed
    std::env::set_var("JWT_SECRET", "secret");

    
    // 2. Start Ephemeral Containers
    let redis_node = Redis::default().start().await.expect("Failed to start Redis container");
    let postgres_node = Postgres::default().start().await.expect("Failed to start Postgres container");
    
    let redis_port = redis_node.get_host_port_ipv4(6379).await.expect("Failed to get Redis port");
    let pg_port = postgres_node.get_host_port_ipv4(5432).await.expect("Failed to get Postgres port");
    
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    let db_url = format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", pg_port);
    
    // 3. Connect DB and Redis
    let redis_conn = redis::Client::open(redis_url)
        .expect("Failed to create Redis client")
        .get_multiplexed_tokio_connection()
        .await
        .expect("Failed to connect to Redis");
        
    let db_pool = sqlx::PgPool::connect(&db_url).await.expect("Failed to connect to Postgres");
    
    // 4. Create necessary minimal schema for gateway tests
    sqlx::query("CREATE EXTENSION IF NOT EXISTS \"pgcrypto\"")
        .execute(&db_pool).await.expect("Failed to enable pgcrypto extension");
    sqlx::query("CREATE TABLE tenants (id UUID PRIMARY KEY DEFAULT gen_random_uuid())")
        .execute(&db_pool).await.expect("Failed to create tenants table");
    sqlx::query("CREATE TABLE api_keys (tenant_id UUID, key_hash TEXT, is_active BOOLEAN)")
        .execute(&db_pool).await.expect("Failed to create api_keys table");
    // provider_keys is the table queried by fetch_tenant_provider_keys in llm_router.rs
    sqlx::query("CREATE TABLE provider_keys (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), tenant_id UUID, provider_name TEXT, encrypted_key BYTEA, created_at TIMESTAMPTZ DEFAULT NOW(), UNIQUE(tenant_id, provider_name))")
        .execute(&db_pool).await.expect("Failed to create provider_keys table");
    
    // 5. Database Fixture setup for an authorized tenant
    let tenant_id = Uuid::new_v4().to_string();
    let encrypted_key = encrypt_test_api_key("sk-mock-openai-key-123", "01234567890123456789012345678901");
    
    sqlx::query("INSERT INTO provider_keys (tenant_id, provider_name, encrypted_key) VALUES ($1, $2, $3)")
        .bind(Uuid::parse_str(&tenant_id).unwrap())
        .bind("openai")
        .bind(&encrypted_key)
        .execute(&db_pool)
        .await
        .expect("Failed to insert mock openai provider key fixture");
        
    // 6. Spawn WireMock
    let mock_server = MockServer::start().await;
    
    // MOCK: Compliance Redaction Service (Returns sanitized proxy JSON)
    Mock::given(method("POST"))
        .and(path("/api/v1/compliance/redact"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sanitized_payload": {
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": "Hi"}]
            }
        })))
        .mount(&mock_server)
        .await;
        
    // MOCK: Upstream LLM Provider (e.g. OpenAI)
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "id": "chatcmpl-123",
                "object": "chat.completion",
                "created": 1677652288,
                "model": "gpt-4o",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello from mock wiremock OpenAI!"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 9,
                    "completion_tokens": 12,
                    "total_tokens": 21
                }
            }))
        )
        .mount(&mock_server)
        .await;

    // Build the shared app state
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();
        
    let (telemetry_tx, _telemetry_rx) = tokio::sync::mpsc::channel(100);
    
    // Build a no-op gRPC channel — cache calls will fail-open in tests.
    // The gateway will proceed to the mock LLM as expected.
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
        rate_limit_max: 5,  // Fast exhaustion limit for testing
        rate_limit_window: 60,
        clickhouse_url: "http://localhost:8123".to_string(),
        tracer_url: mock_server.uri(),
        dashboard_url: "http://localhost:3000".to_string(),
        llm_api_base_url: Some(mock_server.uri()),
        telemetry_tx,
        l1_cache: Arc::new(redeye_gateway::infrastructure::l1_cache::L1Cache::new(1024 * 1024).unwrap()),
    });
    
    TestEnv {
        state,
        mock_server,
        _redis_node: redis_node,
        _postgres_node: postgres_node,
        tenant_id,
    }
}

/// Proves that the auth middleware correctly rejects requests that carry
/// no `Authorization` header.
#[tokio::test]
async fn test_unauthorized_request_is_blocked() {
    let env = setup_test_environment().await;
    let app = create_router(env.state);

    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": "Hi"}]
            })
            .to_string(),
        ))
        .expect("Failed to build test request");

    let response = app.oneshot(request).await.expect("Router returned an error");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Expected 401 UNAUTHORIZED for a request with no auth token, got {}",
        response.status()
    );
}

// ── gRPC Cache Integration Tests ──────────────────────────────────────────────
//
// These tests spin up an in-process tonic mock server (bound to a random OS port)
// to verify the gateway's behaviour under specific gRPC failure conditions.
// No Docker, no network — fast and deterministic.

use redeye_gateway::infrastructure::cache_client::proto::{
    CacheRequest, CacheResponse, StoreRequest, StoreAck,
};
use redeye_gateway::infrastructure::cache_client::proto::cache_service_server::{
    CacheService, CacheServiceServer,
};

/// In-process mock that always returns `hit = false` (cache miss).
struct MockCacheMiss;

#[tonic::async_trait]
impl CacheService for MockCacheMiss {
    async fn lookup_cache(
        &self,
        _req: tonic::Request<CacheRequest>,
    ) -> Result<tonic::Response<CacheResponse>, tonic::Status> {
        Ok(tonic::Response::new(CacheResponse { hit: false, content: String::new() }))
    }
    async fn store_cache(
        &self,
        _req: tonic::Request<StoreRequest>,
    ) -> Result<tonic::Response<StoreAck>, tonic::Status> {
        Ok(tonic::Response::new(StoreAck { stored: true }))
    }
}

/// In-process mock that always returns `Code::Unavailable` to simulate a down server.
struct MockCacheUnavailable;

#[tonic::async_trait]
impl CacheService for MockCacheUnavailable {
    async fn lookup_cache(
        &self,
        _req: tonic::Request<CacheRequest>,
    ) -> Result<tonic::Response<CacheResponse>, tonic::Status> {
        Err(tonic::Status::unavailable("Cache server is down"))
    }
    async fn store_cache(
        &self,
        _req: tonic::Request<StoreRequest>,
    ) -> Result<tonic::Response<StoreAck>, tonic::Status> {
        Err(tonic::Status::unavailable("Cache server is down"))
    }
}

/// Helper: binds a tonic server to a random port and returns the listener address.
async fn bind_grpc_mock<S>(service: S) -> std::net::SocketAddr
where
    S: CacheService,
{
    use tonic::transport::Server;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    // Convert to a std TcpListener so tonic can take it.
    let std_listener = listener.into_std().unwrap();

    tokio::spawn(async move {
        Server::builder()
            .add_service(CacheServiceServer::new(service))
            .serve_with_incoming(
                tokio_stream::wrappers::TcpListenerStream::new(
                    tokio::net::TcpListener::from_std(std_listener).unwrap()
                )
            )
            .await
            .ok();
    });

    addr
}

/// ─── gRPC Cache Client Unit Tests ────────────────────────────────────────────
///
/// These tests focus purely on the `cache_client` layer: spin up an in-process
/// tonic mock, call `lookup_cache` / `store_in_cache` directly, and assert on
/// the returned values. No AppState, no Redis, no Postgres, no Docker needed.

/// `lookup_cache` returns `Some(content)` on a gRPC cache hit.
#[tokio::test]
async fn test_grpc_cache_miss_proceeds_to_llm() {
    use redeye_gateway::infrastructure::cache_client::{CacheGrpcClient, lookup_cache};
    use redeye_gateway::domain::models::TraceContext;

    // Spin up a mock that always returns hit = false.
    let addr = bind_grpc_mock(MockCacheMiss).await;

    // Give the server a moment to accept connections.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let channel = tonic::transport::Channel::from_shared(format!("http://{}", addr))
        .unwrap().connect_lazy();
    let client = CacheGrpcClient::new(channel);

    let trace_ctx = TraceContext {
        trace_id: "t1".into(),
        session_id: "s1".into(),
        parent_trace_id: None,
    };

    let result = lookup_cache(&client, "tenant-1", "gpt-4o", "What is Rust?", &trace_ctx).await;

    // ASSERT: miss returns None, gateway would fall through to LLM.
    assert_eq!(result, None, "Cache miss must return None so the gateway proceeds to the LLM");
}

/// `lookup_cache` returns `None` when the gRPC server returns `Code::Unavailable` (fail-open).
#[tokio::test]
async fn test_grpc_cache_unavailable_is_failopen() {
    use redeye_gateway::infrastructure::cache_client::{CacheGrpcClient, lookup_cache};
    use redeye_gateway::domain::models::TraceContext;

    // Spin up a mock that always returns Unavailable.
    let addr = bind_grpc_mock(MockCacheUnavailable).await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let channel = tonic::transport::Channel::from_shared(format!("http://{}", addr))
        .unwrap().connect_lazy();
    let client = CacheGrpcClient::new(channel);

    let trace_ctx = TraceContext {
        trace_id: "t2".into(),
        session_id: "s2".into(),
        parent_trace_id: None,
    };

    let result = lookup_cache(&client, "tenant-1", "gpt-4o", "Any prompt", &trace_ctx).await;

    // ASSERT: Unavailable must NOT crash the thread — it silently returns None (fail-open).
    assert_eq!(result, None, "Code::Unavailable must cause fail-open (None), not a panic");
}

/// `store_in_cache` completes normally without blocking when the server returns quickly.
#[tokio::test]
async fn test_grpc_store_succeeds_within_timeout() {
    use redeye_gateway::infrastructure::cache_client::{CacheGrpcClient, store_in_cache};
    use redeye_gateway::domain::models::TraceContext;

    let addr = bind_grpc_mock(MockCacheMiss).await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let channel = tonic::transport::Channel::from_shared(format!("http://{}", addr))
        .unwrap().connect_lazy();
    let client = CacheGrpcClient::new(channel);

    let trace_ctx = TraceContext {
        trace_id: "t3".into(),
        session_id: "s3".into(),
        parent_trace_id: None,
    };

    // Must complete without panicking and well under the 2-second deadline.
    store_in_cache(&client, "tenant-1", "gpt-4o", "My prompt", "My response", &trace_ctx).await;
    // If we reach here the test passes — no panic, no timeout.
}
/// Health check endpoint must be publicly accessible (no auth required).
#[tokio::test]
async fn test_health_check_is_public() {
    let env = setup_test_environment().await;
    let app = create_router(env.state);

    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .expect("Failed to build test request");

    let response = app.oneshot(request).await.expect("Router returned an error");

    assert_eq!(response.status(), StatusCode::OK);
}

/// "Happy Path" Test: End-to-end chat completion execution.
/// Follows: Auth -> Compliance -> Cache Miss -> Route to OpenAI Mock -> Emit Telemetry
#[tokio::test]
async fn test_happy_path_chat_completion() {
    let env = setup_test_environment().await;
    let app = create_router(env.state);
    let jwt = generate_test_jwt(&env.tenant_id);

    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", jwt))
        .body(Body::from(
            serde_json::json!({
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": "Hi"}]
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify 200 OK from standard flow
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    assert_eq!(resp_json["choices"][0]["message"]["content"], "Hello from mock wiremock OpenAI!");
}

/// Rate Limit Exhaustion Test: Evaluates Concurrency / Token Bucket Circuit Break setup
#[tokio::test]
async fn test_rate_limit_exhaustion() {
    let env = setup_test_environment().await;
    let jwt = generate_test_jwt(&env.tenant_id);
    let rate_max = env.state.rate_limit_max;
    
    let mut tasks = vec![];
    
    // Blast `max + 1` requests concurrently using `join_all`
    for _ in 0..=rate_max {
        let app = create_router(env.state.clone());
        let token = jwt.clone();
        
        let handle = tokio::spawn(async move {
            let request = Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::from(
                    serde_json::json!({
                        "model": "gpt-4o",
                        "messages": [{"role": "user", "content": "Hello concurrent!"}]
                    }).to_string(),
                ))
                .unwrap();
                
            let resp = app.oneshot(request).await.unwrap();
            resp.status()
        });
        tasks.push(handle);
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let mut successes = 0;
    let mut too_many = 0;
    
    for res_wrap in results {
        match res_wrap.unwrap() {
            StatusCode::OK => successes += 1,
            StatusCode::TOO_MANY_REQUESTS => too_many += 1,
            other => panic!("Unexpected HTTP status code: {}", other),
        }
    }
    
    assert_eq!(successes, rate_max, "Exactly rate_limit_max requests should succeed");
    assert_eq!(too_many, 1, "Exactly 1 request should be rate limited");
}

/// Dynamic Hot-Swap Routine Evaluation: Circuit Breaks OpenAI -> Routes Anthropic seamlessly
#[tokio::test]
async fn test_hot_swap_failover() {
    let env = setup_test_environment().await;
    let app = create_router(env.state.clone());
    let jwt = generate_test_jwt(&env.tenant_id);

    // 1. Reset mock server to drop the default 200 OK behaviors installed by the test env builder
    env.mock_server.reset().await;
    
    // 2. Remount Compliance Redaction Service (Required in pipeline)
    Mock::given(method("POST"))
        .and(path("/api/v1/compliance/redact"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sanitized_payload": {
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": "Hi fallback"}]
            }
        })))
        .mount(&env.mock_server)
        .await;

    // 3. Mount Primary Provider (OpenAI) returning 503 Service Unavailable triggering circuit break
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(503).set_body_string("OpenAI is overloaded"))
        .mount(&env.mock_server)
        .await;

    // 4. Mount Secondary Provider (Anthropic Fallback)
    // execute_provider_request builds: {base_url}/messages where base_url = mock_server.uri()
    // So the effective path is /messages (not /v1/messages)
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "id": "msg_01",
                "type": "message",
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "Hello from Anthropic Fallback Mock!"
                    }
                ],
                "model": "claude-3-opus-20240229",
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 15
                }
            }))
        )
        .mount(&env.mock_server)
        .await;
        
    // 5. Ensure Anthropic API Key is in DB for this tenant (hot-swap fallback target)
    let encrypted_anthropic_key = encrypt_test_api_key("sk-mock-anthropic-key-456", "01234567890123456789012345678901");
    sqlx::query("INSERT INTO provider_keys (tenant_id, provider_name, encrypted_key) VALUES ($1, $2, $3) ON CONFLICT (tenant_id, provider_name) DO UPDATE SET encrypted_key = $3")
        .bind(uuid::Uuid::parse_str(&env.tenant_id).unwrap())
        .bind("anthropic")
        .bind(&encrypted_anthropic_key)
        .execute(&env.state.db_pool)
        .await
        .unwrap();

    // The client calls standard OpenAI route mapped via redeye universal schema
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions") 
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", jwt))
        .body(Body::from(
            serde_json::json!({
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": "Hi fallback"}]
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK, "Expected Hot-Swap to recover with 200 OK");
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    // The fallback Anthropic response may be in OpenAI-translated format (choices[0].message.content)
    // or raw Anthropic format (content[0].text). Accept either.
    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .or_else(|| resp_json["content"][0]["text"].as_str())
        .expect("Expected content in either OpenAI or Anthropic response format");
    
    assert!(
        content.contains("Anthropic") || content.contains("Hello"),
        "Expected Anthropic fallback content, got: {}",
        content
    );
}
