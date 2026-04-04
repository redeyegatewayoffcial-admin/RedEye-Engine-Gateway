use std::sync::Arc;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;
use crate::domain::models::AppState;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    // 1. CORS Preflight Bypass
    if req.method() == axum::http::Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    // 2. Public Endpoints Bypass
    let path = req.uri().path();
    if path == "/health" || path == "/v1/health" {
        return Ok(next.run(req).await);
    }
    
    // 3. Strict Auth & Claims Extraction
    let mut token_opt: Option<(String, bool)> = None; // (token, is_api_key)
    
    if let Some(auth_header) = req.headers().get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if token.starts_with("re_live_") {
                    token_opt = Some((token.to_string(), true));
                } else {
                    token_opt = Some((token.to_string(), false));
                }
            }
        }
    }
    
    if token_opt.is_none() {
        if let Some(api_key_header) = req.headers().get("x-api-key") {
            if let Ok(token) = api_key_header.to_str() {
                if token.starts_with("re_live_") {
                    token_opt = Some((token.to_string(), true));
                }
            }
        }
    }
    
    // 4. Cookie Fallback: Check for re_token cookie if no header auth found
    if token_opt.is_none() {
        if let Some(cookie_header) = req.headers().get(header::COOKIE) {
            if let Ok(cookie_str) = cookie_header.to_str() {
                // Parse cookies and look for re_token
                for cookie in cookie_str.split(';') {
                    let cookie = cookie.trim();
                    if let Some(token) = cookie.strip_prefix("re_token=") {
                        // Validate JWT format: 3 base64url parts separated by dots
                        if token.split('.').count() == 3 && !token.is_empty() {
                            token_opt = Some((token.to_string(), false));
                            tracing::debug!("JWT extracted from re_token cookie");
                            break;
                        }
                    }
                }
            }
        }
    }
    
    match token_opt {
        Some((token, true)) => handle_api_key(&state, &token, req, next).await,
        Some((token, false)) => handle_jwt(&token, req, next).await,
        None => {
            tracing::warn!("Missing authentication credentials for {} {}", req.method(), path);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

pub fn verify_jwt(token: &str) -> Result<Claims, StatusCode> {
    // BUG FIX 1: No more fallback to "secret". Returns 500 Error if missing.
    let secret = std::env::var("JWT_SECRET").map_err(|_| {
        tracing::error!("CRITICAL SECURITY ERROR: JWT_SECRET environment variable is missing!");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Security Boost: Explicitly restrict to HS256 algorithm to prevent algorithm confusion attacks
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;

    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    ) {
        Ok(token_data) => Ok(token_data.claims),
        Err(e) => {
            tracing::warn!("JWT verification failed: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

async fn handle_jwt(
    token: &str,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let claims = verify_jwt(token)?;
    
    // BUG FIX 2: Reject invalid UUIDs (401 Unauthorized) instead of silently defaulting
    let tenant_id = Uuid::parse_str(&claims.tenant_id).map_err(|_| {
        tracing::warn!("Invalid UUID format for tenant_id in token");
        StatusCode::UNAUTHORIZED
    })?;

    req.headers_mut().insert("x-tenant-id", tenant_id.to_string().parse().unwrap());
    
    // CRUCIAL: Inject decoded claims into request extensions
    req.extensions_mut().insert(claims);
    
    Ok(next.run(req).await)
}

async fn handle_api_key(
    state: &Arc<AppState>,
    api_key: &str,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let key_hash = hex::encode(hasher.finalize());

    let row = sqlx::query("SELECT tenant_id FROM api_keys WHERE key_hash = $1 AND is_active = true")
        .bind(&key_hash)
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
    
    let tenant_id: Uuid = tenant_row.get("tenant_id");
    req.headers_mut().insert("x-tenant-id", tenant_id.to_string().parse().unwrap());
    
    // Inject synthetic claims into extensions for API keys
    req.extensions_mut().insert(Claims {
        sub: "api_key".to_string(),
        tenant_id: tenant_id.to_string(),
        exp: 0, // Not applicable for API keys
    });
    
    Ok(next.run(req).await)
}

// Duplicated decryption logic since gateway is a separate crate
pub fn decrypt_api_key(encrypted_data: &[u8]) -> Result<String, ()> {
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
