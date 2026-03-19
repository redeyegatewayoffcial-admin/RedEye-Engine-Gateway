use axum::{extract::State, Json, http::{HeaderMap, HeaderValue, header::SET_COOKIE}};
use serde::{Deserialize, Serialize};
use crate::{AppState, error::AppError, infrastructure::security::{hash_password, verify_password, generate_jwt, encrypt_api_key, generate_redeye_api_key, verify_jwt, generate_refresh_token}};
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
    let hashed_pw = hash_password(&payload.password)?;

    let mut tx = state.db_pool.begin().await?;

    let tenant_id: Uuid = sqlx::query(
        "INSERT INTO tenants (name) VALUES ($1) RETURNING id"
    )
    .bind(&payload.company_name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.constraint() == Some("tenants_name_key") {
                return AppError::BadRequest("Company name already exists".into());
            }
        }
        AppError::from(e)
    })?
    .get("id");

    let user_id: Uuid = sqlx::query(
        "INSERT INTO users (email, password_hash, tenant_id) VALUES ($1, $2, $3) RETURNING id"
    )
    .bind(&payload.email)
    .bind(&hashed_pw)
    .bind(tenant_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.constraint() == Some("users_email_key") {
                return AppError::BadRequest("Email already exists".into());
            }
        }
        AppError::from(e)
    })?
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
        "SELECT u.id, u.password_hash, u.tenant_id, t.name as workspace_name, t.onboarding_status, t.redeye_api_key FROM users u JOIN tenants t ON u.tenant_id = t.id WHERE u.email = $1"
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
    let redeye_api_key: Option<String> = user_row.get("redeye_api_key");

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
        redeye_api_key,
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
        
    let row = sqlx::query("SELECT name, onboarding_status, redeye_api_key FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .fetch_one(&state.db_pool)
        .await?;
        
    let workspace_name: String = row.get("name");
    let onboarding_complete: bool = row.get("onboarding_status");
    let redeye_api_key: Option<String> = row.get("redeye_api_key");

    Ok(Json(AuthResponse {
        id: user_id,
        email,
        tenant_id,
        workspace_name,
        onboarding_complete,
        token,
        redeye_api_key,
    }))
}

// --- POST /v1/auth/onboard ---
#[derive(Deserialize)]
pub struct OnboardRequest {
    pub openai_api_key: String,
    pub workspace_name: Option<String>,
}

pub async fn onboard(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OnboardRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    
    // Extract JWT from Authorization header
    let auth_header = headers.get(axum::http::header::AUTHORIZATION)
        .and_then(|val| val.to_str().ok())
        .and_then(|val| val.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("Missing or invalid Authorization header".into()))?;

    let claims = verify_jwt(auth_header)?;

    let tenant_id = Uuid::parse_str(&claims.tenant_id).map_err(|_| {
        AppError::Internal("Invalid tenant ID in token".into())
    })?;
    
    let user_id = Uuid::parse_str(&claims.sub).unwrap_or_default();

    // Encrypt OpenAI API key
    let encrypted_key = encrypt_api_key(&payload.openai_api_key)?;
    let redeye_api_key = generate_redeye_api_key();

    // Save to database
    sqlx::query(
        "UPDATE tenants SET encrypted_openai_key = $1, redeye_api_key = $2, onboarding_status = true WHERE id = $3"
    )
    .bind(&encrypted_key)
    .bind(&redeye_api_key)
    .bind(tenant_id)
    .execute(&state.db_pool)
    .await?;
    
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

    Ok(Json(AuthResponse {
        id: user_id,
        email,
        tenant_id,
        workspace_name: final_workspace_name,
        onboarding_complete: true,
        token: auth_header.to_string(),
        redeye_api_key: Some(redeye_api_key),
    }))
}
