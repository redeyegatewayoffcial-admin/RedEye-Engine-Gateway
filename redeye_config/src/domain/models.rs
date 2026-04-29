//! Domain models for the `redeye_config` service.
//!
//! This module defines the core data structures that represent the control-plane
//! state of the RedEye AI Gateway. All structs are serialisation-aware and
//! derive SQLx's [`FromRow`] for zero-boilerplate row mapping.
//!
//! # Key invariants
//!
//! * Raw API key material is **never** stored.  The `key_hash` field holds a
//!   SHA-256 hex-encoded digest; the raw bearer token exists only at issuance.
//! * [`UpdateConfigRequest`] uses `Option<T>` for all mutable fields, enabling
//!   PATCH semantics: absent fields preserve their existing value.
//! * [`UpdateConfigRequest::validate`] encodes all domain constraints and must be
//!   called before the payload reaches the infrastructure layer.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// =============================================================================
// ClientConfig — persisted in `client_configs`
// =============================================================================

/// Per-tenant control-plane settings.
///
/// One row exists per tenant; rows are lazily created on the first PUT request.
/// The gateway reads these settings at query-time via a Redis-cached view,
/// falling back to Postgres if the cache is warm-but-stale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
pub struct ClientConfig {
    /// Tenant this config belongs to.  References `tenants.id` in redeye_auth.
    pub tenant_id: Uuid,

    /// When `true`, the compliance layer redacts PII before forwarding to LLMs.
    pub pii_masking_enabled: bool,

    /// When `true`, the gateway performs L2 semantic cache lookups.
    pub semantic_caching_enabled: bool,

    /// When `true`, the gateway hot-swaps to a fallback provider on upstream error.
    pub routing_fallback_enabled: bool,

    /// Optional per-tenant rate cap in **requests per minute**.
    ///
    /// `None` → the gateway applies its global default rate limit.
    pub rate_limit_rpm: Option<i32>,

    /// Optional preferred LLM model identifier (e.g. `"gpt-4o-mini"`).
    ///
    /// `None` → the upstream provider's default model is used.
    pub preferred_model: Option<String>,

    /// Timestamp of the last configuration write.  Maintained by the
    /// application layer, not a DB trigger, to keep the schema portable.
    pub updated_at: DateTime<Utc>,
}

impl ClientConfig {
    /// Constructs a default [`ClientConfig`] for a tenant that has never been
    /// configured.  All feature flags default to **enabled** (fail-open posture).
    pub fn default_for(tenant_id: Uuid) -> Self {
        Self {
            tenant_id,
            pii_masking_enabled: true,
            semantic_caching_enabled: true,
            routing_fallback_enabled: true,
            rate_limit_rpm: None,
            preferred_model: None,
            updated_at: Utc::now(),
        }
    }
}

// =============================================================================
// UpdateConfigRequest — HTTP request body for PUT /v1/config/{tenant_id}
// =============================================================================

/// Partial-update payload for a tenant's configuration (PATCH semantics).
///
/// Every field is optional.  Absent fields are **not changed**; only the fields
/// present in the JSON body are applied to the stored configuration.
///
/// # Validation
///
/// Call [`UpdateConfigRequest::validate`] before applying the update.
/// Callers should map the `Err(String)` to [`crate::error::ConfigError::Validation`].
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConfigRequest {
    pub pii_masking_enabled: Option<bool>,
    pub semantic_caching_enabled: Option<bool>,
    pub routing_fallback_enabled: Option<bool>,

    /// If `Some`, replaces the stored RPM cap.  Must be ≥ 1 if present.
    pub rate_limit_rpm: Option<i32>,

    /// If `Some`, replaces the stored model override.
    /// If `None`, the field is **not touched** (absent from the JSON body).
    /// To **clear** the preferred model, send `"preferred_model": null` —
    /// this deserialises to `Some(None)` via the double-`Option` pattern.
    pub preferred_model: Option<Option<String>>,
}

impl UpdateConfigRequest {
    /// Validates domain invariants on the request.
    ///
    /// Returns `Ok(())` if the request is semantically valid, or `Err(message)`
    /// with a user-safe description of the first violation found.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(rpm) = self.rate_limit_rpm {
            if rpm <= 0 {
                return Err(format!(
                    "`rate_limit_rpm` must be a positive integer, received {}.",
                    rpm
                ));
            }
        }

        if let Some(Some(ref model)) = self.preferred_model {
            let trimmed = model.trim();
            if trimmed.is_empty() {
                return Err("`preferred_model` must not be an empty string. \
                     Omit the field to preserve the current value, or send \
                     `null` to clear it."
                    .into());
            }
            if trimmed.len() > 128 {
                return Err(format!(
                    "`preferred_model` must not exceed 128 characters \
                     (received {} characters).",
                    trimmed.len()
                ));
            }
        }

        Ok(())
    }

    /// Applies this partial update on top of `base`, returning a fully-populated
    /// [`ClientConfig`].  Fields absent from `self` are copied verbatim from `base`.
    ///
    /// The resulting `updated_at` is set to [`Utc::now()`].
    pub fn apply_to(&self, base: &ClientConfig) -> ClientConfig {
        ClientConfig {
            tenant_id: base.tenant_id,

            pii_masking_enabled: self.pii_masking_enabled.unwrap_or(base.pii_masking_enabled),

            semantic_caching_enabled: self
                .semantic_caching_enabled
                .unwrap_or(base.semantic_caching_enabled),

            routing_fallback_enabled: self
                .routing_fallback_enabled
                .unwrap_or(base.routing_fallback_enabled),

            rate_limit_rpm: self.rate_limit_rpm.or(base.rate_limit_rpm),

            // Double-Option semantics:
            //   None           → field absent → keep base value
            //   Some(None)     → explicit null in JSON → clear the value
            //   Some(Some(s))  → explicit string in JSON → set the value
            preferred_model: match &self.preferred_model {
                None => base.preferred_model.clone(),
                Some(inner_opt) => inner_opt.clone(),
            },

            updated_at: Utc::now(),
        }
    }
}

// =============================================================================
// ApiKeyRecord — view of `api_keys` (owned by redeye_auth)
// =============================================================================

/// A virtual API key record as stored in the shared `api_keys` table.
///
/// `redeye_config` never issues keys — that is `redeye_auth`'s responsibility.
/// This service **reads and revokes** existing keys via the shared Postgres
/// instance and propagates revocations to the Redis cache.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ApiKeyRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,

    /// SHA-256 hex-encoded hash of the raw bearer token.
    /// Used to perform a targeted `DEL api_key:{key_hash}` in Redis on revocation.
    #[serde(skip_serializing)]
    pub key_hash: String,

    /// Human-readable label assigned at issuance (e.g. `"Production Key"`).
    pub name: String,

    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

// =============================================================================
// Redis Pub/Sub event payloads
// =============================================================================

/// Published to the `redeye:config_updates` channel when a tenant's settings
/// change.  The gateway subscribes to invalidate its in-memory config cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateEvent {
    pub tenant_id: Uuid,
    pub config: ClientConfig,
}

/// Published to the `redeye:key_revocations` channel when an API key is
/// irrevocably deleted.  The gateway uses `key_hash` to perform a targeted
/// `DEL api_key:{hash}` on its own Redis-backed key-validation cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRevocationEvent {
    pub tenant_id: Uuid,
    pub key_id: Uuid,
    /// Included so the gateway can delete `api_key:{key_hash}` without a
    /// second Postgres round-trip.
    pub key_hash: String,
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn base_config() -> ClientConfig {
        ClientConfig {
            tenant_id: Uuid::new_v4(),
            pii_masking_enabled: true,
            semantic_caching_enabled: false,
            routing_fallback_enabled: true,
            rate_limit_rpm: Some(500),
            preferred_model: Some("gpt-4o".into()),
            updated_at: Utc::now(),
        }
    }

    // ── ClientConfig::default_for ───────────────────────────────────────────

    #[test]
    fn default_config_enables_all_flags() {
        let tid = Uuid::new_v4();
        let cfg = ClientConfig::default_for(tid);
        assert_eq!(cfg.tenant_id, tid);
        assert!(cfg.pii_masking_enabled);
        assert!(cfg.semantic_caching_enabled);
        assert!(cfg.routing_fallback_enabled);
        assert!(cfg.rate_limit_rpm.is_none());
        assert!(cfg.preferred_model.is_none());
    }

    // ── UpdateConfigRequest::validate ───────────────────────────────────────

    #[test]
    fn validate_passes_for_empty_request() {
        let req = UpdateConfigRequest {
            pii_masking_enabled: None,
            semantic_caching_enabled: None,
            routing_fallback_enabled: None,
            rate_limit_rpm: None,
            preferred_model: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn validate_rejects_non_positive_rpm() {
        let req = UpdateConfigRequest {
            pii_masking_enabled: None,
            semantic_caching_enabled: None,
            routing_fallback_enabled: None,
            rate_limit_rpm: Some(0),
            preferred_model: None,
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("rate_limit_rpm"), "error message: {err}");
    }

    #[test]
    fn validate_rejects_negative_rpm() {
        let req = UpdateConfigRequest {
            rate_limit_rpm: Some(-100),
            ..UpdateConfigRequest {
                pii_masking_enabled: None,
                semantic_caching_enabled: None,
                routing_fallback_enabled: None,
                rate_limit_rpm: None,
                preferred_model: None,
            }
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_preferred_model() {
        let req = UpdateConfigRequest {
            pii_masking_enabled: None,
            semantic_caching_enabled: None,
            routing_fallback_enabled: None,
            rate_limit_rpm: None,
            preferred_model: Some(Some("   ".into())), // whitespace-only
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_model_exceeding_128_chars() {
        let long_model = "a".repeat(129);
        let req = UpdateConfigRequest {
            pii_masking_enabled: None,
            semantic_caching_enabled: None,
            routing_fallback_enabled: None,
            rate_limit_rpm: None,
            preferred_model: Some(Some(long_model)),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_accepts_exactly_128_char_model() {
        let model = "m".repeat(128);
        let req = UpdateConfigRequest {
            pii_masking_enabled: None,
            semantic_caching_enabled: None,
            routing_fallback_enabled: None,
            rate_limit_rpm: None,
            preferred_model: Some(Some(model)),
        };
        assert!(req.validate().is_ok());
    }

    // ── UpdateConfigRequest::apply_to ───────────────────────────────────────

    #[test]
    fn apply_to_absent_fields_preserves_base_values() {
        let base = base_config();
        let req = UpdateConfigRequest {
            pii_masking_enabled: None,
            semantic_caching_enabled: None,
            routing_fallback_enabled: None,
            rate_limit_rpm: None,
            preferred_model: None,
        };
        let merged = req.apply_to(&base);
        assert_eq!(merged.pii_masking_enabled, base.pii_masking_enabled);
        assert_eq!(
            merged.semantic_caching_enabled,
            base.semantic_caching_enabled
        );
        assert_eq!(
            merged.routing_fallback_enabled,
            base.routing_fallback_enabled
        );
        assert_eq!(merged.rate_limit_rpm, base.rate_limit_rpm);
        assert_eq!(merged.preferred_model, base.preferred_model);
    }

    #[test]
    fn apply_to_overrides_present_fields() {
        let base = base_config();
        let req = UpdateConfigRequest {
            pii_masking_enabled: Some(false),
            semantic_caching_enabled: Some(true),
            routing_fallback_enabled: None,
            rate_limit_rpm: Some(2000),
            preferred_model: Some(Some("claude-3-5-sonnet".into())),
        };
        let merged = req.apply_to(&base);
        assert!(!merged.pii_masking_enabled);
        assert!(merged.semantic_caching_enabled);
        assert_eq!(
            merged.routing_fallback_enabled,
            base.routing_fallback_enabled
        );
        assert_eq!(merged.rate_limit_rpm, Some(2000));
        assert_eq!(merged.preferred_model, Some("claude-3-5-sonnet".into()));
    }

    #[test]
    fn apply_to_explicit_null_clears_preferred_model() {
        let base = base_config(); // has preferred_model = Some("gpt-4o")
        let req = UpdateConfigRequest {
            pii_masking_enabled: None,
            semantic_caching_enabled: None,
            routing_fallback_enabled: None,
            rate_limit_rpm: None,
            preferred_model: Some(None), // explicit null → clear it
        };
        let merged = req.apply_to(&base);
        assert!(merged.preferred_model.is_none());
    }

    #[test]
    fn apply_to_preserves_tenant_id() {
        let base = base_config();
        let tid = base.tenant_id;
        let req = UpdateConfigRequest {
            pii_masking_enabled: Some(false),
            semantic_caching_enabled: None,
            routing_fallback_enabled: None,
            rate_limit_rpm: None,
            preferred_model: None,
        };
        let merged = req.apply_to(&base);
        assert_eq!(merged.tenant_id, tid);
    }
}
