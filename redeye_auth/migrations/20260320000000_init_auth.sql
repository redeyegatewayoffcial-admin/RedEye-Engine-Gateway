-- Add new columns to existing tenants table (which might be created by init.sql)
ALTER TABLE tenants
ADD COLUMN IF NOT EXISTS encrypted_openai_key BYTEA,
ADD COLUMN IF NOT EXISTS redeye_api_key TEXT UNIQUE,
ADD COLUMN IF NOT EXISTS onboarding_status BOOLEAN NOT NULL DEFAULT FALSE;

-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index tenant_id for fast lookup
CREATE INDEX IF NOT EXISTS idx_users_tenant_id ON users(tenant_id);
