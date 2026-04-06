-- ==============================================================================
-- Migration: Virtual API Key Phase 1 - Multi-LLM Architecture
-- Date: 2026-04-04
-- ==============================================================================

-- ── 1. Add account_type to tenants table ────────────────────────────────────
-- Supports individual users and team workspaces for multi-key generation

ALTER TABLE tenants
ADD COLUMN IF NOT EXISTS account_type VARCHAR(10) NOT NULL DEFAULT 'individual'
    CHECK (account_type IN ('individual', 'team'));

-- Index for fast filtering by account type
CREATE INDEX IF NOT EXISTS idx_tenants_account_type ON tenants(account_type);

-- ── 2. Create provider_keys table ────────────────────────────────────────────
-- Stores encrypted upstream LLM provider API keys per tenant.
-- Decouples provider key storage from routing configuration for better security
-- and multi-provider support per tenant.

CREATE TABLE IF NOT EXISTS provider_keys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider_name   VARCHAR(50) NOT NULL CHECK (provider_name IN ('openai', 'anthropic', 'gemini', 'groq')),
    encrypted_key   BYTEA NOT NULL,              -- AES-256-GCM encrypted provider API key
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique constraint: one encrypted key per tenant+provider combination
CREATE UNIQUE INDEX IF NOT EXISTS idx_provider_keys_tenant_provider 
    ON provider_keys(tenant_id, provider_name);

-- Index for fast tenant-scoped lookups
CREATE INDEX IF NOT EXISTS idx_provider_keys_tenant_id 
    ON provider_keys(tenant_id);

-- ── 3. Verify api_keys.name column exists ────────────────────────────────────
-- The api_keys table should have a name column for multi-key support.
-- This migration ensures backward compatibility if it was missing.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 
        FROM information_schema.columns 
        WHERE table_name = 'api_keys' 
        AND column_name = 'name'
    ) THEN
        ALTER TABLE api_keys ADD COLUMN name TEXT NOT NULL DEFAULT 'Default Key';
    END IF;
END $$;

-- Index for searching keys by name within a tenant
CREATE INDEX IF NOT EXISTS idx_api_keys_tenant_name 
    ON api_keys(tenant_id, name);
