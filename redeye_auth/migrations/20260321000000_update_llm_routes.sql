-- Allow multiple providers per tenant and expand provider CHECK constraint.
-- Drop the old constraint and re-add with new values.
ALTER TABLE llm_routes DROP CONSTRAINT IF EXISTS llm_routes_provider_check;
ALTER TABLE llm_routes ADD CONSTRAINT llm_routes_provider_check
    CHECK (provider IN ('openai', 'gemini', 'groq', 'anthropic'));

-- Drop the unique constraint on tenant_id if it exists (allow multiple rows per tenant)
ALTER TABLE llm_routes DROP CONSTRAINT IF EXISTS llm_routes_tenant_id_key;

-- Ensure we have a unique constraint on (tenant_id, provider) to prevent duplicate providers per tenant
CREATE UNIQUE INDEX IF NOT EXISTS idx_llm_routes_tenant_provider ON llm_routes(tenant_id, provider);
