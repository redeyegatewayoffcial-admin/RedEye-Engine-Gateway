-- ==============================================================================
-- RedEye Policy Engine - ClickHouse Schema
-- Runs automatically on first container boot via docker-entrypoint-initdb.d
-- ==============================================================================

CREATE DATABASE IF NOT EXISTS RedEye_telemetry;

-- ── Request Logs (Immutable Telemetry) ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS RedEye_telemetry.request_logs
(
    id                 UUID,
    tenant_id          String,
    status             UInt16,
    latency_ms         UInt32,
    model              String,
    tokens             UInt32,
    requested_provider String,
    executed_provider  String,
    is_hot_swapped     UInt8,
    created_at         DateTime DEFAULT now()
)
ENGINE = MergeTree
ORDER BY (tenant_id, created_at)
TTL created_at + INTERVAL 90 DAY;

-- ── Agent Traces (Deep Observability) ──────────────────────────────────────────
-- One row per LLM call with session/trace linkage for multi-turn agent chains.
CREATE TABLE IF NOT EXISTS RedEye_telemetry.agent_traces
(
    trace_id           UUID,
    session_id         UUID,
    parent_trace_id    Nullable(UUID),
    tenant_id          String,
    model              String,
    status             UInt16,
    latency_ms         UInt32,
    prompt_tokens      UInt32  DEFAULT 0,
    completion_tokens  UInt32  DEFAULT 0,
    total_tokens       UInt32  DEFAULT 0,
    cache_hit          Bool    DEFAULT false,
    created_at         DateTime DEFAULT now()
)
ENGINE = MergeTree
ORDER BY (session_id, created_at)
TTL created_at + INTERVAL 180 DAY;

-- ── Compliance Audit Log (Regulatory) ──────────────────────────────────────────
-- Full prompt + response content for regulatory review and PII audit.
-- Separate table for access control isolation.
CREATE TABLE IF NOT EXISTS RedEye_telemetry.compliance_audit_log
(
    trace_id           UUID,
    session_id         UUID,
    tenant_id          String,
    prompt_content     String,
    response_content   String,
    model              String,
    flagged            Bool    DEFAULT false,
    flag_reason        String  DEFAULT '',
    created_at         DateTime DEFAULT now()
)
ENGINE = MergeTree
ORDER BY (tenant_id, created_at)
TTL created_at + INTERVAL 365 DAY;

CREATE TABLE IF NOT EXISTS RedEye_telemetry.compliance_engine_audit
(
    tenant_id              String,
    timestamp              DateTime,
    redacted_entity_count  UInt32,
    request_id             String,
    policy_triggered       String
)
ENGINE = MergeTree
ORDER BY (tenant_id, timestamp)
TTL timestamp + INTERVAL 90 DAY;
