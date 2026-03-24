//! usecases/behavior_guard.rs — Agent loop detection circuit breaker.
//!
//! Detects recursive AI agent loops by SHA-256 hashing the request body
//! and comparing the last N hashes stored in a Redis list per session.
//! If 3 consecutive identical hashes are found, the session is blocked
//! to prevent runaway costs.
//!
//! ## Fail-open semantics
//! If Redis is down, the guard logs an error and permits the request.
//! We never break live traffic due to an observability failure.
//!
//! ## Complexity
//! - Time:  O(1) — constant number of Redis round-trips (LRANGE + LPUSH + LTRIM + EXPIRE).
//! - Space: O(1) — stores at most 5 hex hashes (64 bytes each) per session key.

use std::sync::Arc;
use std::time::SystemTime;

use redis::AsyncCommands;
use sha2::{Digest, Sha256};
use tracing::{error, instrument, warn};

use crate::domain::models::{AppState, GatewayError};

/// Number of prior hashes to compare against for loop detection.
const LOOP_WINDOW: isize = 3;
/// Maximum hashes retained in the Redis list per session.
const MAX_HISTORY: isize = 5;
/// TTL for the session hash list (1 hour). Prevents memory leaks for
/// abandoned sessions.
const SESSION_HASH_TTL_SECS: i64 = 3_600;

/// Maximum USD a single session may spend within one calendar minute.
const MAX_BURN_RATE_PER_MINUTE: f64 = 0.005;
/// TTL for burn-rate keys (2 minutes). Guarantees automatic cleanup even
/// if no further requests arrive.
const BURN_KEY_TTL_SECS: i64 = 120;

/// Returns the current epoch minute (seconds since UNIX epoch ÷ 60).
/// Used to bucket session spend into 1-minute windows.
#[inline]
fn current_epoch_minute() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        / 60
}

/// Computes a SHA-256 hex digest of the semantically significant parts
/// of the request body (`messages` + `tools` fields, or the full body
/// if neither is present).
///
/// Using only `messages` and `tools` avoids false negatives caused by
/// ephemeral fields like `temperature` or `stream` changing between
/// otherwise identical agentic retries.
#[inline]
fn compute_body_hash(body: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();

    // Hash only the semantically stable fields when available.
    let has_messages = body.get("messages").is_some();
    let has_tools = body.get("tools").is_some();

    if has_messages || has_tools {
        if let Some(m) = body.get("messages") {
            // `to_string()` gives a deterministic, compact JSON representation.
            hasher.update(m.to_string().as_bytes());
        }
        if let Some(t) = body.get("tools") {
            hasher.update(t.to_string().as_bytes());
        }
    } else {
        // Fallback: hash the entire body.
        hasher.update(body.to_string().as_bytes());
    }

    hex::encode(hasher.finalize())
}

/// Checks whether the current request body forms a recursive loop within
/// the given session.
///
/// Returns `Ok(())` if the request is allowed, or
/// `Err(GatewayError::LoopDetected)` if the last [`LOOP_WINDOW`] hashes
/// are all identical to the current one.
#[instrument(skip(state, body), fields(session_id = %session_id))]
pub async fn enforce_loop_detection(
    state: &Arc<AppState>,
    session_id: &str,
    body: &serde_json::Value,
) -> Result<(), GatewayError> {
    let current_hash = compute_body_hash(body);
    let key = format!("session:{}:hashes", session_id);
    let mut conn = state.redis_conn.clone();

    // ── Read the last LOOP_WINDOW hashes ────────────────────────────────────
    let recent_hashes: Vec<String> = match conn.lrange(&key, 0, LOOP_WINDOW - 1).await {
        Ok(v) => v,
        Err(e) => {
            // Fail-open: Redis is down, log and allow the request.
            error!(error = %e, "Loop detection: Redis LRANGE failed — failing open");
            return Ok(());
        }
    };

    // ── Detect loop ─────────────────────────────────────────────────────────
    if recent_hashes.len() >= LOOP_WINDOW as usize
        && recent_hashes.iter().all(|h| h == &current_hash)
    {
        warn!(
            session_id = %session_id,
            hash = %current_hash,
            window = LOOP_WINDOW,
            "Agent recursive loop detected — blocking session"
        );
        return Err(GatewayError::LoopDetected(
            "Agent recursive loop detected. Session blocked to prevent runaway costs.".into(),
        ));
    }

    // ── Record current hash (LPUSH + LTRIM + EXPIRE) ────────────────────────
    // Pipeline the three commands to minimise round-trips (~1 RTT).
    let push_res: Result<(), _> = redis::pipe()
        .atomic()
        .lpush(&key, &current_hash)
        .ltrim(&key, 0, MAX_HISTORY - 1)
        .expire(&key, SESSION_HASH_TTL_SECS)
        .query_async(&mut conn)
        .await;

    if let Err(e) = push_res {
        // Fail-open: log but don't block traffic.
        error!(error = %e, "Loop detection: Redis pipeline (LPUSH/LTRIM/EXPIRE) failed — failing open");
    }

    Ok(())
}

// ── Burn-Rate Control ───────────────────────────────────────────────────────

/// Pre-LLM guard: rejects the request if the session has already spent
/// ≥ `MAX_BURN_RATE_PER_MINUTE` USD within the current calendar minute.
///
/// Fail-open: if Redis is unreachable the request is permitted.
#[instrument(skip(state), fields(session_id = %session_id))]
pub async fn enforce_burn_rate(
    state: &Arc<AppState>,
    session_id: &str,
) -> Result<(), GatewayError> {
    let minute = current_epoch_minute();
    let key = format!("session:{}:burn:{}", session_id, minute);
    let mut conn = state.redis_conn.clone();

    let current_spend: f64 = match conn.get::<_, Option<String>>(&key).await {
        Ok(Some(v)) => v.parse::<f64>().unwrap_or(0.0),
        Ok(None) => 0.0,
        Err(e) => {
            error!(error = %e, "Burn rate: Redis GET failed — failing open");
            return Ok(());
        }
    };

    if current_spend >= MAX_BURN_RATE_PER_MINUTE {
        warn!(
            session_id = %session_id,
            current_spend,
            limit = MAX_BURN_RATE_PER_MINUTE,
            "Session burn rate exceeded — blocking request"
        );
        return Err(GatewayError::BurnRateExceeded(
            "Session burn rate exceeded. Paused to prevent runaway costs.".into(),
        ));
    }

    Ok(())
}

/// Post-LLM recorder: atomically increments the session's spend for the
/// current calendar minute using `INCRBYFLOAT`, then sets a 120-second
/// expiry so keys self-clean.
///
/// Cost model (MVP): flat $0.002 per 1 000 tokens.
///
/// All Redis errors are logged but never propagated — telemetry must
/// never break the response path.
#[instrument(skip(state), fields(session_id = %session_id, tokens))]
pub async fn record_session_spend(
    state: &Arc<AppState>,
    session_id: &str,
    tokens: u32,
) {
    let cost = (tokens as f64 / 1000.0) * 0.002;
    if cost == 0.0 {
        return;
    }

    let minute = current_epoch_minute();
    let key = format!("session:{}:burn:{}", session_id, minute);
    let mut conn = state.redis_conn.clone();

    // Atomic float increment + TTL in a single pipeline round-trip.
    let res: Result<(), _> = redis::pipe()
        .atomic()
        .cmd("INCRBYFLOAT").arg(&key).arg(cost)
        .expire(&key, BURN_KEY_TTL_SECS)
        .query_async(&mut conn)
        .await;

    if let Err(e) = res {
        error!(error = %e, "Burn rate: Redis INCRBYFLOAT/EXPIRE failed — spend not recorded");
    }
}
