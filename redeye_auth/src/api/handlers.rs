use axum::{extract::{State, Extension}, Json, http::{HeaderMap, HeaderValue, header::SET_COOKIE}};
use serde::{Deserialize, Serialize};
use crate::{AppState, error::AppError, infrastructure::security::{hash_password, verify_password, generate_jwt, encrypt_api_key, generate_redeye_api_key, verify_jwt, generate_refresh_token, Claims}};
use uuid::Uuid;
use sqlx::Row;
use rand::Rng;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret,
    CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl, reqwest::async_http_client,
};

use reqwest::Client;

async fn send_real_otp_email(to_email: &str, otp_code: &str) -> Result<(), AppError> {
    let api_key = std::env::var("RESEND_API_KEY").unwrap_or_default();
    let client = Client::new();

    let email_html = format!(
        "<div style=\"font-family: sans-serif; max-width: 500px; margin: 0 auto;\">
            <h2>Welcome to RedEye Gateway</h2>
            <p>Your magic login code is:</p>
            <h1 style=\"font-size: 40px; letter-spacing: 5px; color: #22d3ee;\">{}</h1>
            <p>This code will expire in 10 minutes.</p>
        </div>",
        otp_code
    );

    let payload = serde_json::json!({
        "from": "RedEye Auth <onboarding@resend.dev>", // Resend provides this test email address
        "to": [to_email],
        "subject": "Your RedEye Login Code",
        "html": email_html
    });

    let res = client.post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to send email via Resend: {}", e);
            AppError::Internal("Failed to send email".into())
        })?;

    if !res.status().is_success() {
        tracing::error!("Resend API error status: {}", res.status());
    }

    Ok(())
}

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
    let (raw_refresh, hash_refresh) = generate_refresh_token(&user_id)?;

    sqlx::query(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, NOW() + INTERVAL '7 days')"
    )
    .bind(user_id)
    .bind(&hash_refresh)
    .execute(&state.db_pool)
    .await?;

    let refresh_cookie = format!(
        "refresh_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Strict",
        raw_refresh
    );

    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, HeaderValue::from_str(&refresh_cookie).unwrap());

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
    let (raw_refresh, hash_refresh) = generate_refresh_token(&user_id)?;

    sqlx::query(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, NOW() + INTERVAL '7 days')"
    )
    .bind(user_id)
    .bind(&hash_refresh)
    .execute(&state.db_pool)
    .await?;

    let refresh_cookie = format!(
        "refresh_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Strict",
        raw_refresh
    );

    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, HeaderValue::from_str(&refresh_cookie).unwrap());

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
) -> Result<impl axum::response::IntoResponse, AppError> {
    // Read the refresh_token from cookies
    let cookie_header = headers.get(axum::http::header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing refresh token cookie".into()))?;
        
    let raw_refresh = cookie_header.split(';')
        .map(|s| s.trim())
        .find(|s| s.starts_with("refresh_token="))
        .map(|s| &s["refresh_token=".len()..])
        .ok_or_else(|| AppError::Unauthorized("Refresh token cookie not found".into()))?;

    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(raw_refresh.as_bytes());
    let old_token_hash = hex::encode(hasher.finalize());

    // Verify the old refresh token exists and is valid
    let row = sqlx::query(
        "SELECT user_id FROM refresh_tokens WHERE token_hash = $1 AND expires_at > NOW()"
    )
    .bind(&old_token_hash)
    .fetch_optional(&state.db_pool)
    .await?;

    let user_id: Uuid = match row {
        Some(r) => r.get("user_id"),
        None => return Err(AppError::Unauthorized("Invalid or expired refresh token".into())),
    };

    // REFRESH TOKEN ROTATION: Delete the old refresh token (invalidate it)
    sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = $1")
        .bind(&old_token_hash)
        .execute(&state.db_pool)
        .await?;

    // Generate new tokens
    let user_row = sqlx::query("SELECT email, tenant_id FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db_pool)
        .await?;
        
    let email: String = user_row.get("email");
    let tenant_id: Uuid = user_row.get("tenant_id");

    let tenant_row = sqlx::query("SELECT name, onboarding_status FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .fetch_one(&state.db_pool)
        .await?;
        
    let workspace_name: String = tenant_row.get("name");
    let onboarding_complete: bool = tenant_row.get("onboarding_status");

    // Generate new JWT and refresh token
    let jwt = generate_jwt(user_id, tenant_id)?;
    let (new_raw_refresh, new_hash_refresh) = generate_refresh_token(&user_id)?;

    // Save the NEW refresh token hash to database
    sqlx::query(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, NOW() + INTERVAL '7 days')"
    )
    .bind(user_id)
    .bind(&new_hash_refresh)
    .execute(&state.db_pool)
    .await?;

    // Set new HttpOnly, Secure cookies with the new tokens
    let jwt_cookie = format!(
        "auth_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Strict",
        jwt
    );
    let refresh_cookie = format!(
        "refresh_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Strict",
        new_raw_refresh
    );

    let mut response_headers = HeaderMap::new();
    response_headers.append(SET_COOKIE, HeaderValue::from_str(&jwt_cookie).unwrap());
    response_headers.append(SET_COOKIE, HeaderValue::from_str(&refresh_cookie).unwrap());

    let response = AuthResponse {
        id: user_id,
        email,
        tenant_id,
        workspace_name,
        onboarding_complete,
        token: jwt,
        redeye_api_key: None,
    };

    Ok((response_headers, Json(response)))
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

// --- OTP Handlers ---
#[derive(Deserialize, Debug)]
pub struct OtpRequestPayload {
    pub email: String,
}

#[axum::debug_handler]
pub async fn request_otp(
    State(state): State<AppState>,
    Json(payload): Json<OtpRequestPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 1. Generate secure 6-digit OTP
    let otp_code: String = {
        let mut rng = rand::thread_rng();
        (0..6).map(|_| rng.gen_range(0..10).to_string()).collect()
    };

    // 2. Set expiry (10 minutes from now)
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);

    // 3. Save to DB
    sqlx::query(
        "INSERT INTO auth_otps (email, otp_code, expires_at) VALUES ($1, $2, $3)"
    )
    .bind(&payload.email)
    .bind(&otp_code)
    .bind(expires_at)
    .execute(&state.db_pool)
    .await?;

    // --- REAL EMAIL SENDER ---
    send_real_otp_email(&payload.email, &otp_code).await?;
    tracing::info!("✉️ Real OTP Email sent to: {}", payload.email);

    Ok(Json(serde_json::json!({"message": "OTP sent to email successfully"})))
}

#[derive(Deserialize)]
pub struct OtpVerifyPayload {
    pub email: String,
    pub otp_code: String,
}

pub async fn verify_otp(
    State(state): State<AppState>,
    Json(payload): Json<OtpVerifyPayload>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let row = sqlx::query(
        "SELECT id FROM auth_otps WHERE email = $1 AND otp_code = $2 AND expires_at > NOW()"
    )
    .bind(&payload.email)
    .bind(&payload.otp_code)
    .fetch_optional(&state.db_pool)
    .await?;

    if row.is_none() {
        return Err(AppError::Unauthorized("Invalid or expired OTP".into()));
    }

    let mut tx = state.db_pool.begin().await?;

    sqlx::query("DELETE FROM auth_otps WHERE email = $1 AND otp_code = $2")
        .bind(&payload.email)
        .bind(&payload.otp_code)
        .execute(&mut *tx)
        .await?;

    let user_row = sqlx::query("SELECT id, tenant_id FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&mut *tx)
        .await?;

    let (user_id, tenant_id, workspace_name, onboarding_complete) = if let Some(r) = user_row {
        let u_id: Uuid = r.get("id");
        let t_id: Uuid = r.get("tenant_id");
        let t_row = sqlx::query("SELECT name, onboarding_status FROM tenants WHERE id = $1")
            .bind(t_id)
            .fetch_one(&mut *tx)
            .await?;
        let t_name: String = t_row.get("name");
        let onboarding_sts: bool = t_row.get("onboarding_status");
        (u_id, t_id, t_name, onboarding_sts)
    } else {
        let new_t_id: Uuid = sqlx::query("INSERT INTO tenants (name) VALUES ($1) RETURNING id")
            .bind("My Workspace")
            .fetch_one(&mut *tx)
            .await?
            .get("id");
        
        let new_u_id: Uuid = sqlx::query(
            "INSERT INTO users (email, tenant_id, auth_provider) VALUES ($1, $2, 'email_otp') RETURNING id"
        )
        .bind(&payload.email)
        .bind(new_t_id)
        .fetch_one(&mut *tx)
        .await?
        .get("id");

        (new_u_id, new_t_id, "My Workspace".to_string(), false)
    };

    tx.commit().await?;

    let token = generate_jwt(user_id, tenant_id)?;
    let (raw_refresh, hash_refresh) = generate_refresh_token(&user_id)?;

    sqlx::query(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, NOW() + INTERVAL '7 days')"
    )
    .bind(user_id)
    .bind(&hash_refresh)
    .execute(&state.db_pool)
    .await?;

    let refresh_cookie = format!(
        "refresh_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Strict",
        raw_refresh
    );
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, HeaderValue::from_str(&refresh_cookie).unwrap());

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

// --- Google OAuth Handlers ---
fn google_oauth_client() -> BasicClient {
    BasicClient::new(
        ClientId::new(std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default()),
        Some(ClientSecret::new(std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default())),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap(),
        Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(std::env::var("GOOGLE_REDIRECT_URI").unwrap_or_else(|_| "http://localhost:8084/v1/auth/google/callback".to_string())).unwrap())
}

pub async fn google_login() -> impl axum::response::IntoResponse {
    let (auth_url, csrf_token) = google_oauth_client()
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();

    let cookie = format!(
        "oauth_state={}; HttpOnly; Path=/; Max-Age=600; SameSite=Lax",
        csrf_token.secret()
    );
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(axum::http::header::SET_COOKIE, axum::http::HeaderValue::from_str(&cookie).unwrap());

    (headers, axum::response::Redirect::to(auth_url.as_ref()))
}

#[derive(Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: String,
}

pub async fn google_callback(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<OAuthCallbackQuery>,
    headers: axum::http::HeaderMap,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let cookie_header = headers.get(axum::http::header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing OAuth state cookie".into()))?;
        
    let saved_state = cookie_header.split(';')
        .map(|s| s.trim())
        .find(|s| s.starts_with("oauth_state="))
        .map(|s| &s["oauth_state=".len()..])
        .ok_or_else(|| AppError::Unauthorized("OAuth state cookie not found".into()))?;

    if query.state != saved_state {
        return Err(AppError::Unauthorized("Invalid CSRF state".into()));
    }
    let token = google_oauth_client()
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(async_http_client)
        .await
        .map_err(|e| AppError::Unauthorized(format!("OAuth failed: {}", e)))?;

    let client = reqwest::Client::new();
    let user_info_res = client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let user_info: serde_json::Value = user_info_res
        .json()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let email = user_info.get("email").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let sub = user_info.get("sub").and_then(|v| v.as_str()).unwrap_or_default().to_string();

    if email.is_empty() {
        return Err(AppError::Unauthorized("No email from Google".into()));
    }

    let mut tx = state.db_pool.begin().await?;

    let user_row = sqlx::query("SELECT id, tenant_id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await?;

    let (user_id, tenant_id, workspace_name, onboarding_complete) = if let Some(r) = user_row {
        let u_id: Uuid = r.get("id");
        let t_id: Uuid = r.get("tenant_id");
        let t_row = sqlx::query("SELECT name, onboarding_status FROM tenants WHERE id = $1")
            .bind(t_id)
            .fetch_one(&mut *tx)
            .await?;
        let t_name: String = t_row.get("name");
        let onboarding_sts: bool = t_row.get("onboarding_status");
        
        sqlx::query("UPDATE users SET auth_provider = 'google', provider_id = $1 WHERE id = $2")
            .bind(&sub)
            .bind(u_id)
            .execute(&mut *tx)
            .await?;

        (u_id, t_id, t_name, onboarding_sts)
    } else {
        let new_t_id: Uuid = sqlx::query("INSERT INTO tenants (name) VALUES ($1) RETURNING id")
            .bind("My Workspace")
            .fetch_one(&mut *tx)
            .await?
            .get("id");
        
        let new_u_id: Uuid = sqlx::query(
            "INSERT INTO users (email, tenant_id, auth_provider, provider_id) VALUES ($1, $2, 'google', $3) RETURNING id"
        )
        .bind(&email)
        .bind(new_t_id)
        .bind(&sub)
        .fetch_one(&mut *tx)
        .await?
        .get("id");

        (new_u_id, new_t_id, "My Workspace".to_string(), false)
    };

    tx.commit().await?;

    let jwt = generate_jwt(user_id, tenant_id)?;
    let (raw_refresh, hash_refresh) = generate_refresh_token(&user_id)?;

    sqlx::query(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, NOW() + INTERVAL '7 days')"
    )
    .bind(user_id)
    .bind(&hash_refresh)
    .execute(&state.db_pool)
    .await?;

    // Clear oauth_state cookie, set refresh_token cookie with Secure flag
    let refresh_cookie = format!(
        "refresh_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Strict",
        raw_refresh
    );
    let state_clear_cookie = "oauth_state=; HttpOnly; Path=/; Max-Age=0; SameSite=Lax".to_string();
    // Set JWT as HttpOnly Secure cookie instead of URL parameter
    let jwt_cookie = format!(
        "auth_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Strict",
        jwt
    );

    let mut headers = HeaderMap::new();
    headers.append(SET_COOKIE, HeaderValue::from_str(&refresh_cookie).unwrap());
    headers.append(SET_COOKIE, HeaderValue::from_str(&state_clear_cookie).unwrap());
    headers.append(SET_COOKIE, HeaderValue::from_str(&jwt_cookie).unwrap());

    // Redirect without token in URL - client reads from cookie
    let dashboard_url = std::env::var("DASHBOARD_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
    let redirect_url = format!("{}{}?onboarding_complete={}", dashboard_url, "/oauth/callback", onboarding_complete);

    Ok((headers, axum::response::Redirect::to(&redirect_url)))
}

// --- GitHub OAuth Handlers ---
fn github_oauth_client() -> BasicClient {
    BasicClient::new(
        ClientId::new(std::env::var("GITHUB_CLIENT_ID").unwrap_or_default()),
        Some(ClientSecret::new(std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_default())),
        AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(std::env::var("GITHUB_REDIRECT_URI").unwrap_or_else(|_| "http://localhost:8084/v1/auth/github/callback".to_string())).unwrap())
}

pub async fn github_login() -> impl axum::response::IntoResponse {
    let (auth_url, csrf_token) = github_oauth_client()
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("user:email".to_string()))
        .url();

    let cookie = format!(
        "oauth_state={}; HttpOnly; Path=/; Max-Age=600; SameSite=Lax",
        csrf_token.secret()
    );
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(axum::http::header::SET_COOKIE, axum::http::HeaderValue::from_str(&cookie).unwrap());

    (headers, axum::response::Redirect::to(auth_url.as_ref()))
}

pub async fn github_callback(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<OAuthCallbackQuery>,
    headers: axum::http::HeaderMap,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let cookie_header = headers.get(axum::http::header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing OAuth state cookie".into()))?;
        
    let saved_state = cookie_header.split(';')
        .map(|s| s.trim())
        .find(|s| s.starts_with("oauth_state="))
        .map(|s| &s["oauth_state=".len()..])
        .ok_or_else(|| AppError::Unauthorized("OAuth state cookie not found".into()))?;

    if query.state != saved_state {
        return Err(AppError::Unauthorized("Invalid CSRF state".into()));
    }

    let token = github_oauth_client()
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(async_http_client)
        .await
        .map_err(|e| AppError::Unauthorized(format!("OAuth failed: {}", e)))?;

    let client = reqwest::Client::new();
    let user_info_res = client
        .get("https://api.github.com/user")
        .bearer_auth(token.access_token().secret())
        .header("User-Agent", "RedEye-Auth-Backend")
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let user_info: serde_json::Value = user_info_res
        .json()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let sub = user_info.get("id").map(|v| v.to_string()).unwrap_or_default();
    
    // Github primary email might be null in /user, need to fetch /user/emails
    let mut email = user_info.get("email").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    
    if email.is_empty() {
        let emails_res = client
            .get("https://api.github.com/user/emails")
            .bearer_auth(token.access_token().secret())
            .header("User-Agent", "RedEye-Auth-Backend")
            .send()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
            
        let emails: Vec<serde_json::Value> = emails_res
            .json()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
            
        for e in emails {
            if e.get("primary").and_then(|v| v.as_bool()).unwrap_or(false) {
                email = e.get("email").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                break;
            }
        }
    }

    if email.is_empty() {
        return Err(AppError::Unauthorized("No primary email from GitHub".into()));
    }

    let mut tx = state.db_pool.begin().await?;

    let user_row = sqlx::query("SELECT id, tenant_id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await?;

    let (user_id, tenant_id, workspace_name, onboarding_complete) = if let Some(r) = user_row {
        let u_id: Uuid = r.get("id");
        let t_id: Uuid = r.get("tenant_id");
        let t_row = sqlx::query("SELECT name, onboarding_status FROM tenants WHERE id = $1")
            .bind(t_id)
            .fetch_one(&mut *tx)
            .await?;
        let t_name: String = t_row.get("name");
        let onboarding_sts: bool = t_row.get("onboarding_status");
        
        sqlx::query("UPDATE users SET auth_provider = 'github', provider_id = $1 WHERE id = $2")
            .bind(&sub)
            .bind(u_id)
            .execute(&mut *tx)
            .await?;

        (u_id, t_id, t_name, onboarding_sts)
    } else {
        let new_t_id: Uuid = sqlx::query("INSERT INTO tenants (name) VALUES ($1) RETURNING id")
            .bind("My Workspace")
            .fetch_one(&mut *tx)
            .await?
            .get("id");
        
        let new_u_id: Uuid = sqlx::query(
            "INSERT INTO users (email, tenant_id, auth_provider, provider_id) VALUES ($1, $2, 'github', $3) RETURNING id"
        )
        .bind(&email)
        .bind(new_t_id)
        .bind(&sub)
        .fetch_one(&mut *tx)
        .await?
        .get("id");

        (new_u_id, new_t_id, "My Workspace".to_string(), false)
    };

    tx.commit().await?;

    let jwt = generate_jwt(user_id, tenant_id)?;
    let (raw_refresh, hash_refresh) = generate_refresh_token(&user_id)?;

    sqlx::query(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, NOW() + INTERVAL '7 days')"
    )
    .bind(user_id)
    .bind(&hash_refresh)
    .execute(&state.db_pool)
    .await?;

    let refresh_cookie = format!(
        "refresh_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Strict",
        raw_refresh
    );
    let state_clear_cookie = "oauth_state=; HttpOnly; Path=/; Max-Age=0; SameSite=Lax".to_string();
    // Set JWT as HttpOnly Secure cookie instead of URL fragment
    let jwt_cookie = format!(
        "auth_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Strict",
        jwt
    );

    let mut headers = axum::http::HeaderMap::new();
    headers.append(axum::http::header::SET_COOKIE, axum::http::HeaderValue::from_str(&refresh_cookie).unwrap());
    headers.append(axum::http::header::SET_COOKIE, axum::http::HeaderValue::from_str(&state_clear_cookie).unwrap());
    headers.append(axum::http::header::SET_COOKIE, axum::http::HeaderValue::from_str(&jwt_cookie).unwrap());

    let dashboard_url = std::env::var("DASHBOARD_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
    let redirect_path = if onboarding_complete { "/dashboard" } else { "/onboarding" };
    
    // Redirect without token in URL - client reads from cookie
    let redirect_url = format!("{}{}?onboarding_complete={}", dashboard_url, redirect_path, onboarding_complete);

    Ok((headers, axum::response::Redirect::to(&redirect_url)))
}
