//! AES-256-GCM key decryption for the `redeye_config` service.
//!
//! Provider API keys are encrypted by `redeye_auth` using AES-256-GCM with a
//! random 12-byte nonce prepended to the ciphertext.  This module provides
//! the **decryption** half so the config service can reconstruct plaintext keys
//! when building the routing mesh published to Redis.
//!
//! Both services share the same `AES_MASTER_KEY` environment variable.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};

use crate::error::ConfigError;

/// Decrypts an AES-256-GCM blob produced by `redeye_auth`'s `encrypt_api_key`.
///
/// # Format
/// `encrypted_data` = 12-byte nonce ‖ ciphertext (GCM tag included)
///
/// # Errors
/// Returns [`ConfigError::Internal`] if:
/// - `AES_MASTER_KEY` env var is missing or not exactly 32 bytes.
/// - The data is shorter than the minimum (12 nonce bytes + 1 byte ciphertext).
/// - AES-GCM decryption fails (wrong key, corrupted data, tampered tag).
/// - The plaintext is not valid UTF-8.
pub fn decrypt_api_key(encrypted_data: &[u8]) -> Result<String, ConfigError> {
    let master_key = std::env::var("AES_MASTER_KEY")
        .map_err(|_| ConfigError::Internal("AES_MASTER_KEY env var is missing".into()))?;

    if master_key.as_bytes().len() != 32 {
        return Err(ConfigError::Internal(
            "AES_MASTER_KEY must be exactly 32 bytes long".into(),
        ));
    }

    if encrypted_data.len() < 12 {
        return Err(ConfigError::Internal(
            "Encrypted data is too short to contain a valid nonce".into(),
        ));
    }

    let key = Key::<Aes256Gcm>::from_slice(master_key.as_bytes());
    let cipher = Aes256Gcm::new(key);

    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
        tracing::error!(error = %e, "AES-256-GCM decryption failed in redeye_config");
        ConfigError::Internal("API key decryption failed".into())
    })?;

    String::from_utf8(plaintext)
        .map_err(|_| ConfigError::Internal("Decrypted API key is not valid UTF-8".into()))
}
