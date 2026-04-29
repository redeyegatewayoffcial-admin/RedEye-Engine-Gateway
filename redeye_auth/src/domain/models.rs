//! Domain models for the RedEye Auth service.
//! These structs represent the database entities and are used across the application.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Account type for tenant workspaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    /// Individual user account (default)
    Individual,
    /// Team workspace account supporting multiple users and API keys
    Team,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Individual
    }
}

/// A tenant represents an organization or individual workspace.
/// All resources are scoped to a tenant.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
    pub onboarding_status: bool,
    /// Account type: 'individual' or 'team'
    pub account_type: AccountType,
}

/// A virtual API key issued to tenant applications.
/// The raw key is never stored; only a SHA-256 hash is persisted.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub tenant_id: Uuid,
    /// SHA-256 hash of the raw key (hex-encoded)
    pub key_hash: String,
    /// Human-readable name for the key (e.g., "Default Project", "Dev Key")
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// An encrypted upstream LLM provider API key.
/// Each tenant can store multiple provider keys for multi-LLM support.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProviderKey {
    pub id: Uuid,
    pub tenant_id: Uuid,
    /// The LLM provider (e.g., 'openai', 'anthropic')
    pub provider_name: String,
    /// AES-256-GCM encrypted provider API key
    #[serde(skip_serializing)]
    pub encrypted_key: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

/// A user belonging to a tenant workspace.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    /// Argon2id password hash (never plaintext)
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub tenant_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// LLM routing configuration for a tenant.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LlmRoute {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub provider: String,
    pub model: String,
    pub is_default: bool,
    #[serde(skip_serializing)]
    pub encrypted_api_key: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
}

/// Refresh token for session management.
#[derive(Debug, Clone, FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    /// SHA-256 hash of the raw refresh token
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// OTP code for email-based authentication.
#[derive(Debug, Clone, FromRow)]
pub struct AuthOtp {
    pub id: Uuid,
    pub email: String,
    pub otp_code: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
