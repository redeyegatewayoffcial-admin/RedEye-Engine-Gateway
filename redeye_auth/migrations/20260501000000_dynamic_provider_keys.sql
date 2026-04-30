-- Drop the existing unique index to allow the new unique constraint 
DROP INDEX IF EXISTS idx_provider_keys_tenant_provider;

-- Fallback to drop a constraint if it exists via an ALTER TABLE
ALTER TABLE provider_keys DROP CONSTRAINT IF EXISTS provider_keys_tenant_id_provider_name_key;

-- Safely add the new columns
ALTER TABLE provider_keys ADD COLUMN IF NOT EXISTS key_alias VARCHAR NOT NULL DEFAULT 'default';
ALTER TABLE provider_keys ADD COLUMN IF NOT EXISTS priority INT NOT NULL DEFAULT 1;
ALTER TABLE provider_keys ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT true;

-- Add the new unique constraint combining tenant_id, provider_name, and key_alias
CREATE UNIQUE INDEX IF NOT EXISTS unique_tenant_provider_alias ON provider_keys(tenant_id, provider_name, key_alias);
