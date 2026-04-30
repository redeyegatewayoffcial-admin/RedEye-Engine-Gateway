-- =============================================================================
-- redeye_auth — Add weight column to provider_keys for load balancing
-- =============================================================================
-- The previous migration (20260501000000) added key_alias, priority, is_active
-- but omitted the weight column which is required for the routing mesh assembly.
-- =============================================================================

ALTER TABLE provider_keys
    ADD COLUMN IF NOT EXISTS weight INT NOT NULL DEFAULT 100;
