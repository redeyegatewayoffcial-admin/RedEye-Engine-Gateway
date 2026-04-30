use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use std::sync::Arc;
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::postgres::Postgres;
use tower::ServiceExt;

use redeye_auth::api::router::create_router;
use redeye_auth::AppState;
use sqlx::PgPool;
use sqlx::Row;

/// Holds the environment guards so they live as long as the test does.
pub struct TestEnv {
    pub db_pool: PgPool,
    pub app: axum::Router,
    pub _postgres_node: ContainerAsync<Postgres>,
}

async fn setup_test_environment() -> TestEnv {
    // Inject standard env variable mocked values for AES and JWT internals
    std::env::set_var("AES_MASTER_KEY", "01234567890123456789012345678901");
    std::env::set_var("JWT_SECRET", "super_secret_test_key");

    // Spin up an ephemeral testcontainer for Postgres
    let postgres_node = Postgres::default()
        .start()
        .await
        .expect("Failed to start Postgres container");
    let pg_port = postgres_node
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to bind postgres port");
    let db_url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        pg_port
    );

    // Bootstrap Pool
    let db_pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect SQLx to local container");

    // Pre-initialize schemas manually since standard testing migrations block
    let init_queries = [
        "CREATE EXTENSION IF NOT EXISTS pgcrypto;",
        "CREATE TABLE tenants (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name TEXT, active BOOLEAN, onboarding_status BOOLEAN, account_type TEXT);",
        "CREATE TABLE users (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), email TEXT UNIQUE, password_hash TEXT, active BOOLEAN, tenant_id UUID, is_app_admin BOOLEAN);",
        "CREATE TABLE tenant_users (tenant_id UUID, user_id UUID, role TEXT, PRIMARY KEY(tenant_id, user_id));",
        "CREATE TABLE api_keys (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), tenant_id UUID, key_hash TEXT, name TEXT, is_active BOOLEAN);",
        "CREATE TABLE llm_routes (tenant_id UUID, provider TEXT, model TEXT, is_default BOOLEAN, encrypted_api_key BYTEA, PRIMARY KEY(tenant_id, provider));",
        "CREATE TABLE provider_keys (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), tenant_id UUID, provider_name TEXT, encrypted_key BYTEA, key_alias TEXT, created_at TIMESTAMPTZ DEFAULT NOW(), UNIQUE(tenant_id, provider_name, key_alias));",
        "CREATE TABLE auth_otps (email TEXT PRIMARY KEY, otp_code TEXT, expires_at TIMESTAMPTZ);",
        "CREATE TABLE refresh_tokens (user_id UUID, token_hash TEXT, expires_at TIMESTAMPTZ);",
    ];

    for query in init_queries {
        sqlx::query(query)
            .execute(&db_pool)
            .await
            .expect("Migration failed query execution");
    }

    let state = AppState {
        db_pool: db_pool.clone(),
    };
    let app = create_router(state);

    TestEnv {
        db_pool,
        app,
        _postgres_node: postgres_node,
    }
}

/// Black Box Test: Signup, Verification & AES mapping DB Assertions
#[tokio::test]
async fn test_auth_pipeline() {
    let env = setup_test_environment().await;

    // 1. SIGNUP TEST (Happy Path)
    let signup_payload = json!({
        "email": "test@redeye.ai",
        "password": "securepassword123",
        "company_name": "Test Org"
    });

    let req_signup = Request::builder()
        .method("POST")
        .uri("/v1/auth/signup")
        .header("content-type", "application/json")
        .body(Body::from(signup_payload.to_string()))
        .unwrap();

    // Since `app` consumes the router on `oneshot`, we clone the router to bypass ownership issues natively.
    let res_signup = env.app.clone().oneshot(req_signup).await.unwrap();
    assert_eq!(res_signup.status(), StatusCode::OK, "Signup failed");

    let body_bytes = res_signup.into_body().collect().await.unwrap().to_bytes();
    let signup_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let jwt = signup_json["token"].as_str().expect("No token found");

    // 2. Add Provider Key Test (AES Execution Verification)
    let provider_payload = json!({
        "provider_name": "mock",
        "api_key": "sk-my-super-secret-key",
        "key_alias": "primary"
    });

    let req_provider = Request::builder()
        .method("POST")
        .uri("/v1/auth/provider-keys") // Note: /v1/auth wraps the protected routes
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", jwt))
        .body(Body::from(provider_payload.to_string()))
        .unwrap();

    let res_provider = env.app.clone().oneshot(req_provider).await.unwrap();
    assert_eq!(
        res_provider.status(),
        StatusCode::OK,
        "Failed to submit provider keys dynamically"
    );

    // 3. Database AES Assertions (Validates DB isolation and AES encryption engine functionality)
    let route_row =
        sqlx::query("SELECT encrypted_key FROM provider_keys WHERE provider_name = 'mock'")
            .fetch_one(&env.db_pool)
            .await
            .expect("Provider key missing from mocked SQL db");

    let db_ciphertext: Vec<u8> = route_row.get("encrypted_key");

    // Validate that the underlying text isn't raw and AES properly chunked it (12 byte nonce + payload + tag bounds)
    assert!(
        db_ciphertext.len() > 12,
        "AES Ciphertext missing cryptographic padding/nonce bindings"
    );

    // We shouldn't be able to just cast string to utf8 and see our secret
    let raw_string_fallback = String::from_utf8(db_ciphertext.clone()).unwrap_or_default();
    assert!(
        !raw_string_fallback.contains("sk-my-super-secret-key"),
        "CRITICAL ERROR: DB Stored the key in plaintext!"
    );
}
