-- =============================================================================
-- redeye_config — Fix llm_models schema for routing mesh support
-- =============================================================================
-- The original migration (20260501000001) was missing:
--   1. tenant_id      — needed to scope models per-tenant
--   2. schema_format  — needed to identify the API schema for translation
--   3. The UNIQUE constraint was global (model_name only), not per-tenant
-- This migration safely adds the missing columns and fixes the constraint.
-- =============================================================================

-- Step 1: Add tenant_id column
ALTER TABLE llm_models
    ADD COLUMN IF NOT EXISTS tenant_id UUID;

-- Step 2: Add schema_format column (defaults to 'openai' for existing rows)
ALTER TABLE llm_models
    ADD COLUMN IF NOT EXISTS schema_format VARCHAR NOT NULL DEFAULT 'openai';

-- Step 3: Drop the old global constraint on model_name alone
--         Use ALTER TABLE DROP CONSTRAINT (not DROP INDEX directly in Postgres)
ALTER TABLE llm_models DROP CONSTRAINT IF EXISTS llm_models_model_name_key;

-- Step 4: Add the correct per-tenant unique index (non-partial for ON CONFLICT support)
CREATE UNIQUE INDEX IF NOT EXISTS unique_llm_models_tenant_model
    ON llm_models (tenant_id, model_name);
