-- =============================================================================
-- redeye_config — Initial Schema Migration
-- Service: redeye_config (control-plane)
-- =============================================================================
-- Creates the `client_configs` table for per-tenant feature-flag management.
--
-- Note: The `api_keys` table is owned by redeye_auth and shared via the same
--       Postgres instance. redeye_config performs DELETE operations on that
--       table during key-revocation workflows.
-- =============================================================================

CREATE TABLE IF NOT EXISTS client_configs (
    -- Primary key: one config row per tenant.
    tenant_id                UUID        PRIMARY KEY,

    -- Feature toggles — all default to enabled (fail-open posture).
    pii_masking_enabled      BOOLEAN     NOT NULL DEFAULT true,
    semantic_caching_enabled BOOLEAN     NOT NULL DEFAULT true,
    routing_fallback_enabled BOOLEAN     NOT NULL DEFAULT true,

    -- Optional per-tenant rate cap in requests-per-minute.
    -- NULL means the gateway falls back to its global default.
    rate_limit_rpm           INTEGER     CHECK (rate_limit_rpm IS NULL OR rate_limit_rpm > 0),

    -- Optional preferred LLM model identifier (e.g. 'gpt-4o-mini', 'claude-3-5-sonnet').
    -- NULL means the gateway uses the provider's default model.
    preferred_model          VARCHAR(128),

    -- Last-write timestamp, maintained by the application layer (not a DB trigger,
    -- to keep schema portable across environments).
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE  client_configs
    IS 'Per-tenant control-plane feature toggles for the RedEye AI Gateway.';
COMMENT ON COLUMN client_configs.tenant_id
    IS 'Foreign key to tenants.id in the redeye_auth service schema.';
COMMENT ON COLUMN client_configs.pii_masking_enabled
    IS 'When true, the compliance layer redacts PII before forwarding to upstream LLMs.';
COMMENT ON COLUMN client_configs.semantic_caching_enabled
    IS 'When true, the gateway performs L2 semantic cache lookups before routing upstream.';
COMMENT ON COLUMN client_configs.routing_fallback_enabled
    IS 'When true, the gateway hot-swaps to a fallback LLM provider on upstream failure.';
COMMENT ON COLUMN client_configs.rate_limit_rpm
    IS 'Per-tenant rate cap (requests/minute). NULL inherits the gateway global default.';
COMMENT ON COLUMN client_configs.preferred_model
    IS 'Preferred LLM model string passed to the provider. NULL uses the provider default.';
