-- Drop the hardcoded provider_name check constraint to allow dynamic LLM providers
ALTER TABLE provider_keys DROP CONSTRAINT IF EXISTS provider_keys_provider_name_check;
