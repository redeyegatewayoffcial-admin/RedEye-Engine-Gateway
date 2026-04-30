use crate::error::AppError;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng as AesOsRng},
    Aes256Gcm, Key, Nonce,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::STANDARD as b64, Engine as _};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use uuid::Uuid;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| {
            tracing::error!("Failed to hash password: {}", e);
            AppError::Internal("Password hashing failed".into())
        })?
        .to_string();
    Ok(password_hash)
}

pub fn verify_password(hash: &str, password: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|e| {
        tracing::error!("Invalid password hash format: {}", e);
        AppError::Internal("Invalid hash format".into())
    })?;

    let is_valid = Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();

    Ok(is_valid)
}

pub fn encrypt_api_key(plaintext: &str) -> Result<Vec<u8>, AppError> {
    let master_key = env::var("AES_MASTER_KEY")
        .map_err(|_| AppError::Internal("AES_MASTER_KEY missing".into()))?;

    if master_key.as_bytes().len() != 32 {
        return Err(AppError::Internal(
            "AES_MASTER_KEY must be exactly 32 bytes long".into(),
        ));
    }

    let key = Key::<Aes256Gcm>::from_slice(master_key.as_bytes());
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut AesOsRng);

    let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes()).map_err(|e| {
        tracing::error!("AES encryption failed: {}", e);
        AppError::Internal("Encryption failed".into())
    })?;

    // Prepend nonce to ciphertext for storage
    let mut stored_data = nonce.to_vec();
    stored_data.extend_from_slice(&ciphertext);
    Ok(stored_data)
}

pub fn decrypt_api_key(encrypted_data: &[u8]) -> Result<String, AppError> {
    let master_key = env::var("AES_MASTER_KEY")
        .map_err(|_| AppError::Internal("AES_MASTER_KEY missing".into()))?;

    if master_key.as_bytes().len() != 32 {
        return Err(AppError::Internal(
            "AES_MASTER_KEY must be exactly 32 bytes long".into(),
        ));
    }

    if encrypted_data.len() < 12 {
        return Err(AppError::Internal("Encrypted data too short".into()));
    }

    let key = Key::<Aes256Gcm>::from_slice(master_key.as_bytes());
    let cipher = Aes256Gcm::new(key);

    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
        tracing::error!("AES decryption failed: {}", e);
        AppError::Internal("Decryption failed".into())
    })?;

    String::from_utf8(plaintext)
        .map_err(|_| AppError::Internal("Invalid UTF-8 in decrypted data".into()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // User ID
    pub tenant_id: String,
    pub exp: usize,
}

pub fn generate_jwt(user_id: Uuid, tenant_id: Uuid) -> Result<String, AppError> {
    let secret = env::var("JWT_SECRET").map_err(|_| {
        tracing::error!("JWT_SECRET environment variable is missing");
        AppError::Internal("JWT configuration error".into())
    })?;

    let expiration = Utc::now()
        .checked_add_signed(Duration::days(7))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        exp: expiration,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| {
        tracing::error!("JWT encoding failed: {}", e);
        AppError::Internal("Token generation failed".into())
    })
}

// O(1) Time, O(1) Space
#[tracing::instrument(skip(_user_id))]
pub fn generate_refresh_token(_user_id: &Uuid) -> Result<(String, String), AppError> {
    use rand::RngCore;
    let mut token_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut token_bytes);

    let raw_token = hex::encode(token_bytes);

    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    let token_hash = hex::encode(hasher.finalize());

    Ok((raw_token, token_hash))
}

pub fn verify_jwt(token: &str) -> Result<Claims, AppError> {
    let secret = env::var("JWT_SECRET").map_err(|_| {
        tracing::error!("JWT_SECRET environment variable is missing");
        AppError::Internal("JWT configuration error".into())
    })?;

    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        tracing::error!("JWT verification failed: {}", e);
        AppError::Unauthorized("Invalid token".into())
    })?;

    Ok(token_data.claims)
}

pub fn generate_redeye_api_key() -> String {
    let random_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    format!("re_live_{}", random_string)
}

pub fn hash_api_key(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use std::env;
    use uuid::Uuid;

    #[test]
    fn test_jwt_lifecycle() {
        env::set_var("JWT_SECRET", "super_secret_key");

        // 1. Success
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let token = generate_jwt(user_id.clone(), tenant_id.clone()).unwrap();
        let claims = verify_jwt(&token).unwrap();
        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.tenant_id, tenant_id.to_string());

        // 2. Failure: Expired
        let expired_claims = Claims {
            sub: Uuid::new_v4().to_string(),
            tenant_id: Uuid::new_v4().to_string(),
            exp: Utc::now()
                .checked_sub_signed(Duration::days(1))
                .unwrap()
                .timestamp() as usize,
        };
        let expired_token = encode(
            &Header::new(Algorithm::HS256),
            &expired_claims,
            &EncodingKey::from_secret("super_secret_key".as_bytes()),
        )
        .unwrap();
        assert!(verify_jwt(&expired_token).is_err());

        // 3. Failure: Invalid Signature
        let mut invalid_token = generate_jwt(Uuid::new_v4(), Uuid::new_v4()).unwrap();
        invalid_token.push_str("invalid");
        assert!(verify_jwt(&invalid_token).is_err());
    }

    #[test]
    fn test_aes_lifecycle() {
        // 1. Success
        env::set_var("AES_MASTER_KEY", "12345678901234567890123456789012");
        let plaintext = "re_live_testing123";
        let encrypted = encrypt_api_key(plaintext).unwrap();
        let decrypted = decrypt_api_key(&encrypted).unwrap();
        assert_eq!(plaintext, decrypted);

        // 2. Failure: Wrong Key
        let encrypted_for_fail = encrypt_api_key("test_data").unwrap();
        env::set_var("AES_MASTER_KEY", "00000000000000000000000000000000");
        assert!(decrypt_api_key(&encrypted_for_fail).is_err());

        // 3. Failure: Malformed Input
        env::set_var("AES_MASTER_KEY", "12345678901234567890123456789012");
        let short_data = vec![0u8; 10]; // Minimum is 12 bytes
        assert!(decrypt_api_key(&short_data).is_err());
    }
}
