-- ==============================================================================
-- RedEye Policy Engine - PostgreSQL Schema
-- Runs automatically on first container boot via docker-entrypoint-initdb.d
-- ==============================================================================

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ── Tenants ───────────────────────────────────────────────────────────────────
-- Each enterprise customer is a "tenant". All resources are tenant-scoped.
CREATE TABLE IF NOT EXISTS tenants (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name              TEXT NOT NULL UNIQUE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active         BOOLEAN NOT NULL DEFAULT TRUE,
    onboarding_status BOOLEAN NOT NULL DEFAULT FALSE
);

-- ── Users ─────────────────────────────────────────────────────────────────────
-- Human operators. Each user belongs to exactly one tenant (workspace).
-- `password_hash` stores an Argon2id hash — plaintext is NEVER persisted.
CREATE TABLE IF NOT EXISTS users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,            -- Argon2id hash (never plaintext)
    tenant_id     UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_tenant_id ON users(tenant_id);

-- ── API Keys ──────────────────────────────────────────────────────────────────
-- Keys issued to tenant applications for authenticating with the gateway.
-- `key_hash` stores a SHA-256 hash — the raw key is never persisted.
CREATE TABLE IF NOT EXISTS api_keys (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    key_hash    TEXT NOT NULL UNIQUE,       -- SHA-256(raw_key) stored in hex
    name        TEXT NOT NULL,              -- Human-readable label (e.g. "prod-app-1")
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ,               -- NULL = never expires
    is_active   BOOLEAN NOT NULL DEFAULT TRUE
);

-- ── Rate Limit Policies ───────────────────────────────────────────────────────
-- Per-tenant rate limit configuration. Applied by the Redis layer in Phase 3.
CREATE TABLE IF NOT EXISTS rate_limit_policies (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    requests_per_min INTEGER NOT NULL DEFAULT 60,
    tokens_per_day   BIGINT  NOT NULL DEFAULT 1000000,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── LLM Routes ───────────────────────────────────────────────────────────────
-- Defines which upstream LLM provider a tenant's traffic is routed to.
CREATE TABLE IF NOT EXISTS llm_routes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider        TEXT NOT NULL CHECK (provider IN ('openai', 'gemini', 'groq', 'anthropic')),
    model           TEXT NOT NULL,             -- e.g. "gpt-4o", "claude-sonnet-4-20250514"
    is_default      BOOLEAN NOT NULL DEFAULT FALSE,
    encrypted_api_key BYTEA,                  -- AES-256-GCM encrypted upstream API key
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique constraint per tenant+provider for UPSERT support
CREATE UNIQUE INDEX IF NOT EXISTS idx_llm_routes_tenant_provider ON llm_routes(tenant_id, provider);

-- ── Seed Data (Development) ───────────────────────────────────────────────────
INSERT INTO tenants (id, name) VALUES
    ('00000000-0000-0000-0000-000000000001', 'acme-corp'),
    ('00000000-0000-0000-0000-000000000002', 'globex-inc')
ON CONFLICT DO NOTHING;

INSERT INTO rate_limit_policies (tenant_id, requests_per_min, tokens_per_day) VALUES
    ('00000000-0000-0000-0000-000000000001', 120, 5000000),
    ('00000000-0000-0000-0000-000000000002', 60,  1000000)
ON CONFLICT DO NOTHING;

INSERT INTO llm_routes (tenant_id, provider, model, is_default) VALUES
    ('00000000-0000-0000-0000-000000000001', 'openai',    'gpt-4o',                    TRUE),
    ('00000000-0000-0000-0000-000000000002', 'anthropic', 'claude-sonnet-4-20250514', TRUE)
ON CONFLICT DO NOTHING;
