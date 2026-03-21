use axum::{extract::{State, Extension}, Json, http::{HeaderMap, HeaderValue, header::SET_COOKIE}};
use serde::{Deserialize, Serialize};
use crate::{AppState, error::AppError, infrastructure::security::{hash_password, verify_password, generate_jwt, encrypt_api_key, generate_redeye_api_key, verify_jwt, generate_refresh_token, Claims}};
use uuid::Uuid;
use sqlx::Row;

// --- POST /v1/auth/signup ---
#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub company_name: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub id: Uuid,
    pub email: String,
    pub tenant_id: Uuid,
    pub workspace_name: String,
    pub onboarding_complete: bool,
    pub token: String,
    pub redeye_api_key: Option<String>,
}

pub async fn signup(
    State(state): State<AppState>,
    Json(payload): Json<SignupRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    
    // 1. Check if email already exists
    let email_exists: bool = sqlx::query("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
        .bind(&payload.email)
        .fetch_one(&state.db_pool)
        .await?
        .get(0);
        
    if email_exists {
        return Err(AppError::Conflict("Email already registered".into()));
    }
    
    // 2. Check if company_name already exists
    let workspace_exists: bool = sqlx::query("SELECT EXISTS(SELECT 1 FROM tenants WHERE name = $1)")
        .bind(&payload.company_name)
        .fetch_one(&state.db_pool)
        .await?
        .get(0);
        
    if workspace_exists {
        return Err(AppError::Conflict("Workspace name already taken".into()));
    }

    let hashed_pw = hash_password(&payload.password)?;

    // 3. Begin Atomic Transaction
    let mut tx = state.db_pool.begin().await?;

    let tenant_id: Uuid = sqlx::query(
        "INSERT INTO tenants (name) VALUES ($1) RETURNING id"
    )
    .bind(&payload.company_name)
    .fetch_one(&mut *tx)
    .await?
    .get("id");

    let user_id: Uuid = sqlx::query(
        "INSERT INTO users (email, password_hash, tenant_id) VALUES ($1, $2, $3) RETURNING id"
    )
    .bind(&payload.email)
    .bind(&hashed_pw)
    .bind(tenant_id)
    .fetch_one(&mut *tx)
    .await?
    .get("id");

    tx.commit().await?;

    let token = generate_jwt(user_id, tenant_id)?;
    let refresh_token = generate_refresh_token(user_id, tenant_id)?;

    let cookie = format!(
        "refresh_token={}; HttpOnly; Path=/; Max-Age=604800; SameSite=Strict",
        refresh_token
    );

    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());

    Ok((headers, Json(AuthResponse { 
        id: user_id,
        email: payload.email,
        tenant_id,
        workspace_name: payload.company_name,
        onboarding_complete: false,
        token,
        redeye_api_key: None,
    })))
}

// --- POST /v1/auth/login ---
#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let row = sqlx::query(
        "SELECT u.id, u.password_hash, u.tenant_id, t.name as workspace_name, t.onboarding_status FROM users u JOIN tenants t ON u.tenant_id = t.id WHERE u.email = $1"
    )
        .bind(&payload.email)
        .fetch_optional(&state.db_pool)
        .await?;

    let user_row = match row {
        Some(r) => r,
        None => return Err(AppError::Unauthorized("Invalid email or password".into())),
    };

    let p_hash: String = user_row.get("password_hash");
    let is_valid = verify_password(&p_hash, &payload.password)?;
    
    if !is_valid {
        return Err(AppError::Unauthorized("Invalid email or password".into()));
    }

    let user_id: Uuid = user_row.get("id");
    let tenant_id: Uuid = user_row.get("tenant_id");
    let workspace_name: String = user_row.get("workspace_name");
    let onboarding_complete: bool = user_row.get("onboarding_status");

    let token = generate_jwt(user_id, tenant_id)?;
    let refresh_token = generate_refresh_token(user_id, tenant_id)?;

    let cookie = format!(
        "refresh_token={}; HttpOnly; Path=/; Max-Age=604800; SameSite=Strict",
        refresh_token
    );

    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());

    Ok((headers, Json(AuthResponse {
        id: user_id,
        email: payload.email,
        tenant_id,
        workspace_name,
        onboarding_complete,
        token,
        redeye_api_key: None,
    })))
}

// --- POST /v1/auth/refresh ---
pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthResponse>, AppError> {
    // Read the refresh_token from cookies
    let cookie_header = headers.get(axum::http::header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing refresh token cookie".into()))?;
        
    let refresh_token = cookie_header.split(';')
        .map(|s| s.trim())
        .find(|s| s.starts_with("refresh_token="))
        .map(|s| &s["refresh_token=".len()..])
        .ok_or_else(|| AppError::Unauthorized("Refresh token cookie not found".into()))?;

    let claims = verify_jwt(refresh_token)?;
    let user_id = Uuid::parse_str(&claims.sub).unwrap_or_default();
    let tenant_id = Uuid::parse_str(&claims.tenant_id).unwrap_or_default();

    // Re-issue a short-lived token
    let token = generate_jwt(user_id, tenant_id)?;

    // Return partial response (or full if you want, but for simplicity partial is okay)
    // To keep the dashboard working, we can return the same AuthResponse.
    let email: String = sqlx::query("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db_pool)
        .await?
        .get("email");
        
    let row = sqlx::query("SELECT name, onboarding_status FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .fetch_one(&state.db_pool)
        .await?;
        
    let workspace_name: String = row.get("name");
    let onboarding_complete: bool = row.get("onboarding_status");

    Ok(Json(AuthResponse {
        id: user_id,
        email,
        tenant_id,
        workspace_name,
        onboarding_complete,
        token,
        redeye_api_key: None,
    }))
}

// --- POST /v1/auth/onboard ---
#[derive(Deserialize)]
pub struct OnboardRequest {
    pub provider: String,
    pub api_key: String,
    pub workspace_name: Option<String>,
}

pub async fn onboard(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<OnboardRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    
    let tenant_id = Uuid::parse_str(&claims.tenant_id).map_err(|_| {
        AppError::Internal("Invalid tenant ID in token".into())
    })?;
    
    let user_id = Uuid::parse_str(&claims.sub).unwrap_or_default();

    // Validate the API key against the provider
    let is_valid = crate::infrastructure::llm_validator::validate_api_key(&payload.provider, &payload.api_key)
        .await
        .map_err(|e| AppError::Internal(e))?;

    if !is_valid {
        return Err(AppError::BadRequest("Invalid API Key".into()));
    }

    // Encrypt the validated API key
    let encrypted_key = encrypt_api_key(&payload.api_key)?;
    let redeye_api_key = generate_redeye_api_key();
    let key_hash = crate::infrastructure::security::hash_api_key(&redeye_api_key);

    let mut tx = state.db_pool.begin().await?;

    // Update ONBOARDING STATUS
    sqlx::query(
        "UPDATE tenants SET onboarding_status = true WHERE id = $1"
    )
    .bind(tenant_id)
    .execute(&mut *tx)
    .await?;

    // Insert keys into API_KEYS
    sqlx::query(
        "INSERT INTO api_keys (tenant_id, key_hash, name) VALUES ($1, $2, 'default') ON CONFLICT DO NOTHING"
    )
    .bind(tenant_id)
    .bind(&key_hash)
    .execute(&mut *tx)
    .await?;

    // UPSERT into LLM_ROUTES (supports multiple providers per tenant)
    sqlx::query(
        "INSERT INTO llm_routes (tenant_id, provider, model, is_default, encrypted_api_key)
         VALUES ($1, $2, 'default', true, $3)
         ON CONFLICT (tenant_id, provider) DO UPDATE SET encrypted_api_key = $3"
    )
    .bind(tenant_id)
    .bind(&payload.provider)
    .bind(&encrypted_key)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    
    // Optionally update workspace_name
    let final_workspace_name = if let Some(ws_name) = &payload.workspace_name {
        sqlx::query("UPDATE tenants SET name = $1 WHERE id = $2")
            .bind(ws_name)
            .bind(tenant_id)
            .execute(&state.db_pool)
            .await?;
        ws_name.clone()
    } else {
        sqlx::query("SELECT name FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_one(&state.db_pool)
            .await?
            .get("name")
    };
    
    let email: String = sqlx::query("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db_pool)
        .await?
        .get("email");

    // Note: In onboard, we don't have the Bearer token handy since it was stripped by middleware.
    // However, for onboarding response, the dashboard might expect the token to be returned to stay logged in.
    // We can potentially re-issue or just skip it if the dashboard doesn't strictly need it here.
    // Given AuthResponse REQUIRES token, let's re-generate it.
    let token = generate_jwt(user_id, tenant_id)?;

    Ok(Json(AuthResponse {
        id: user_id,
        email,
        tenant_id,
        workspace_name: final_workspace_name,
        onboarding_complete: true,
        token,
        redeye_api_key: Some(redeye_api_key),
    }))
}

// --- GET /v1/auth/api-keys ---
#[derive(Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub status: String,
}

pub async fn get_api_keys(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<ApiKeyResponse>>, AppError> {
    
    let tenant_id = Uuid::parse_str(&claims.tenant_id).map_err(|_| {
        AppError::Internal("Invalid tenant ID in token".into())
    })?;

    let rows = sqlx::query(
        "SELECT id, name, key_hash, created_at FROM api_keys WHERE tenant_id = $1 ORDER BY created_at DESC"
    )
    .bind(tenant_id)
    .fetch_all(&state.db_pool)
    .await?;

    let keys = rows.into_iter().map(|row| ApiKeyResponse {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        key_hash: row.try_get("key_hash").unwrap_or_default(),
        created_at: row.try_get("created_at").unwrap_or_else(|_| chrono::Utc::now()),
        status: "Active".to_string(), // In a real app we'd track revoked status in DB. For now they are Active
    }).collect();

    Ok(Json(keys))
}
