use std::sync::Arc;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;
use crate::domain::models::AppState;
use deadpool_redis::redis::AsyncCommands;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub tenant_id: String,
    pub exp: usize,
}

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    
    // Check for standard JWT auth OR RedEye API Key
    let mut token_opt: Option<(String, bool)> = None; // (token, is_api_key)
    
    if let Some(auth_header) = req.headers().get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                // Determine if it's a JWT or a red-eye key (assuming JWTs don't start with re-sk-)
                if token.starts_with("re-sk-") {
                    token_opt = Some((token.to_string(), true));
                } else {
                    token_opt = Some((token.to_string(), false));
                }
            }
        }
    }
    
    // Also check x-api-key header
    if token_opt.is_none() {
        if let Some(api_key_header) = req.headers().get("x-api-key") {
            if let Ok(token) = api_key_header.to_str() {
                if token.starts_with("re-sk-") {
                    token_opt = Some((token.to_string(), true));
                }
            }
        }
    }
    
    match token_opt {
        Some((token, true)) => handle_api_key(&state, &token, req, next).await,
        Some((token, false)) => handle_jwt(&state, &token, req, next).await,
        None => {
            tracing::warn!("Missing or invalid authentication credentials");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

async fn handle_jwt(
    state: &Arc<AppState>,
    token: &str,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    
    let token_data = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(e) => {
            tracing::warn!("Invalid JWT: {}", e);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    
    let tenant_id = Uuid::parse_str(&token_data.claims.tenant_id).unwrap_or_default();
    
    req.headers_mut().insert("x-tenant-id", tenant_id.to_string().parse().unwrap());
    
    // Fetch and inject OpenAI API Key
    inject_openai_key(state, tenant_id, &mut req).await?;
    
    Ok(next.run(req).await)
}

async fn handle_api_key(
    state: &Arc<AppState>,
    api_key: &str,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let row = sqlx::query("SELECT id FROM tenants WHERE redeye_api_key = $1 AND is_active = true")
        .bind(api_key)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("DB error during api key lookup: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
    let tenant_row = match row {
        Some(r) => r,
        None => {
            tracing::warn!("Invalid RedEye API Key");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    
    let tenant_id: Uuid = tenant_row.get("id");
    req.headers_mut().insert("x-tenant-id", tenant_id.to_string().parse().unwrap());
    
    // Fetch and inject OpenAI API Key
    inject_openai_key(state, tenant_id, &mut req).await?;
    
    Ok(next.run(req).await)
}

async fn inject_openai_key(state: &Arc<AppState>, tenant_id: Uuid, req: &mut Request<Body>) -> Result<(), StatusCode> {
    let redis_key = format!("tenant_openai_key:{}", tenant_id);
    
    // Check Redis first
    if let Ok(mut conn) = state.redis_pool.get().await {
        if let Ok(cached_key) = conn.get::<_, String>(&redis_key).await {
            req.headers_mut().insert(
                axum::http::header::AUTHORIZATION,
                format!("Bearer {}", cached_key).parse().unwrap() // overwrite JWT with cached OpenAI key
            );
            return Ok(());
        }
    }

    // Cache Miss, check DB
    let row = sqlx::query("SELECT encrypted_openai_key FROM tenants WHERE id = $1 AND onboarding_status = true")
        .bind(tenant_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("DB error fetching openai key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
    let tenant_row = match row {
        Some(r) => r,
        None => {
            tracing::warn!("Tenant not found or onboarding not complete");
            return Err(StatusCode::FORBIDDEN);
        }
    };
    
    let encrypted_data: Option<Vec<u8>> = tenant_row.get(0);
    
    if let Some(encrypted_bytes) = encrypted_data {
        match decrypt_api_key(&encrypted_bytes) {
            Ok(openai_key) => {
                // Set in Redis with 300s TTL (5 minutes)
                if let Ok(mut conn) = state.redis_pool.get().await {
                    let _: Result<(), _> = conn.set_ex(&redis_key, &openai_key, 300).await;
                }

                req.headers_mut().insert(
                    axum::http::header::AUTHORIZATION,
                    format!("Bearer {}", openai_key).parse().unwrap() 
                );
                Ok(())
            },
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    } else {
        tracing::warn!("No OpenAI key found for tenant");
        Err(StatusCode::FORBIDDEN)
    }
}

// Duplicated decryption logic since gateway is a separate crate
fn decrypt_api_key(encrypted_data: &[u8]) -> Result<String, ()> {
    let master_key = std::env::var("AES_MASTER_KEY").map_err(|_| ())?;
    if master_key.len() != 32 || encrypted_data.len() < 12 {
        return Err(());
    }

    let key = Key::<Aes256Gcm>::from_slice(master_key.as_bytes());
    let cipher = Aes256Gcm::new(key);
    
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| ())?;
    String::from_utf8(plaintext).map_err(|_| ())
}
