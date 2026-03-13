-- ==============================================================================
-- NexusAI Policy Engine - ClickHouse Schema
-- Runs automatically on first container boot via docker-entrypoint-initdb.d
-- ==============================================================================

CREATE DATABASE IF NOT EXISTS nexusai_telemetry;

-- ── Request Logs (Immutable Telemetry) ─────────────────────────────────────────
-- Stores one row per LLM request proxied through the gateway.
-- Optimized for insanely fast aggregations (e.g., total tokens per tenant per day).
CREATE TABLE IF NOT EXISTS nexusai_telemetry.request_logs
(
    id          UUID,
    tenant_id   String, -- Storing as String for easy JOIN equivalents if exported
    status      UInt16,
    latency_ms  UInt32,
    model       String,
    tokens      UInt32,
    created_at  DateTime DEFAULT now()
)
ENGINE = MergeTree
ORDER BY (tenant_id, created_at)
TTL created_at + INTERVAL 90 DAY; -- Auto-delete logs older than 90 days
