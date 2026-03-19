use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use crate::{AppState, error::AppError};

// --- POST /v1/auth/signup ---
#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub company_name: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
}

pub async fn signup(
    State(_state): State<AppState>,
    Json(_payload): Json<SignupRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // TODO: 1. Validate payload
    // TODO: 2. Call Usecase (e.g., signup_usecase)
    //          - Hash password using argon2 (via infrastructure injected)
    //          - Create User & Tenant in DB
    //          - Generate JWT token
    
    // Placeholder response
    Ok(Json(AuthResponse {
        token: "jwt_token_placeholder".to_string(),
    }))
}

// --- POST /v1/auth/login ---
#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(_state): State<AppState>,
    Json(_payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // TODO: 1. Validate payload
    // TODO: 2. Call Usecase (login_usecase)
    //          - Fetch User from DB
    //          - Verify password hash
    //          - Generate JWT token

    // Placeholder response
    Ok(Json(AuthResponse {
        token: "jwt_token_placeholder".to_string(),
    }))
}

// --- POST /v1/auth/onboard ---
#[derive(Deserialize)]
pub struct OnboardRequest {
    pub openai_api_key: String,
}

#[derive(Serialize)]
pub struct OnboardResponse {
    pub status: String,
    pub redeye_api_key: String, // Prefix: re-sk-...
}

pub async fn onboard(
    State(_state): State<AppState>,
    // In real implementation, the authenticated user/tenant data is extracted via middleware
    // Extension(_user): Extension<AuthenticatedUser>, 
    Json(_payload): Json<OnboardRequest>,
) -> Result<Json<OnboardResponse>, AppError> {
    // TODO: 1. Identify Tenant associated with user
    // TODO: 2. Call Usecase (onboard_api_key_usecase)
    //          - Encrypt OpenAI API Key using AES placeholder
    //          - Generate redeye_api_key with "re-sk-" prefix
    //          - Save to Tenant table
    //          - publish_api_key_to_gateway(tenant_id, redeye_api_key)

    // Placeholder response
    Ok(Json(OnboardResponse {
        status: "success".to_string(),
        redeye_api_key: "re-sk-placeholder".to_string(),
    }))
}
