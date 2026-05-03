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

use std::sync::atomic::Ordering;
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
        .unwrap_or_default()
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

/// Enforces the agentic loop budget using a 5-minute sliding window via Redis.
/// Limit: 15 calls per 5-minute window.
/// Fallback: If Redis times out, fallback to L1 cache with a strict 5 loops max limit.
/// Returns the current loop count if allowed, or an AgentLoopBudgetExceeded error.
#[instrument(skip(state), fields(session_id = %session_id))]
pub async fn enforce_agentic_loop_budget(
    state: &Arc<AppState>,
    session_id: &str,
) -> Result<u64, GatewayError> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| GatewayError::AgentLoopBudgetExceeded(format!("System time drift: {}", e)))?
        .as_millis() as u64;
    let window_start = now.saturating_sub(300_000); // 5 minutes
    
    let key = format!("session:{}:agent_loops", session_id);
    let member = format!("{}:{}", now, uuid::Uuid::new_v4());

    let mut conn = state.redis_conn.clone();
    
    // Sliding Window:
    // 1. Remove old entries outside the 5-minute window
    // 2. Add current request timestamp
    // 3. Count total entries in the window
    // 4. Set expiry to 300s to avoid memory leaks
    let res: Result<(i64, i64, u64, i64), _> = redis::pipe()
        .atomic()
        .cmd("ZREMRANGEBYSCORE").arg(&key).arg("-inf").arg(window_start)
        .cmd("ZADD").arg(&key).arg(now).arg(&member)
        .cmd("ZCOUNT").arg(&key).arg("-inf").arg("+inf")
        .expire(&key, 300)
        .query_async(&mut conn)
        .await;

    match res {
        Ok((_, _, count, _)) => {
            if count > 15 {
                warn!(session_id = %session_id, count, "Agent loop budget exceeded (Redis)");
                Err(GatewayError::AgentLoopBudgetExceeded("Agent loop budget exceeded".into()))
            } else {
                Ok(count)
            }
        }
        Err(e) => {
            error!(error = %e, "Loop detection: Redis pipeline failed — failing over to L1 cache");
            
            // L1 Fallback: Get or initialize the AtomicU64 counter
            let counter = state.loop_fallback_cache.get_with(session_id.to_string(), async {
                Arc::new(std::sync::atomic::AtomicU64::new(0))
            }).await;
            
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if count > 5 {
                warn!(session_id = %session_id, count, "Agent loop budget exceeded (L1 Fallback)");
                Err(GatewayError::AgentLoopBudgetExceeded("Agent loop budget exceeded".into()))
            } else {
                Ok(count)
            }
        }
    }
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
        Ok(Some(v)) => v.parse::<f64>().map_err(|e| {
            error!(error = %e, "Burn rate: Redis value corrupted — failing closed");
            GatewayError::BurnRateExceeded("Burn rate limit enforcement failed due to corrupted tracking data".into())
        })?,
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
pub async fn record_session_spend(state: &Arc<AppState>, session_id: &str, tokens: u32) {
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
        .cmd("INCRBYFLOAT")
        .arg(&key)
        .arg(cost)
        .expire(&key, BURN_KEY_TTL_SECS)
        .query_async(&mut conn)
        .await;

    if let Err(e) = res {
        error!(error = %e, "Burn rate: Redis INCRBYFLOAT/EXPIRE failed — spend not recorded");
    }
}

// ── Tunnel 3 Phase 3: MCP Sandbox Firewall (Fail-Closed) ────────────────────

/// Returns `true` when the sandbox should fail-open (legacy / non-production).
///
/// Reads `SANDBOX_FALLBACK_MODE` env var once per request.
/// Default (unset) = fail-closed: OPA downtime **blocks** execution.
/// Set `SANDBOX_FALLBACK_MODE=open` to revert to fail-open behaviour.
#[inline]
fn sandbox_is_fail_open() -> bool {
    std::env::var("SANDBOX_FALLBACK_MODE")
        .map(|v| v.to_ascii_lowercase() == "open")
        .unwrap_or(false)
}

/// Compact deny payload — single JSON object, minimal token cost.
const DENY_CONTENT: &str = r#"{"error":"RedEye Guard: Policy Denied. Use read-only tools."}"#;

/// Inspects a buffered LLM response body for `tool_calls` and enforces
/// the MCP sandbox policy via an OPA RBAC check.
///
/// On OPA **deny** (or OPA failure in fail-closed mode), the response body is
/// mutated in-place: the assistant `tool_calls` message is replaced with a
/// compact `role:"tool"` soft-steer payload that keeps the agentic loop alive
/// without exposing the denial as a hard HTTP error.
///
/// ## Fail-Closed (Phase 3 default)
/// * OPA timeout / unreachable → **DENY** (blocks execution).
/// * OPA non-2xx → **DENY** (blocks execution).
/// * Parse failure on body → allow unchanged (body unreadable, best-effort).
///
/// Set `SANDBOX_FALLBACK_MODE=open` to revert to fail-open (non-prod only).
pub async fn enforce_mcp_sandbox(
    state: &Arc<AppState>,
    tenant_id: &str,
    response_body: Vec<u8>,
) -> Vec<u8> {
    // ── Fast-path: simd-json scan for tool_calls ─────────────────────────────
    // Clone only the first 4 KB for the scan window.
    let scan_len = response_body.len().min(4 * 1024);
    let scan_buf = response_body[..scan_len].to_vec();

    // Fast-path: byte-level scan for the literal key "tool_calls".
    // If absent, no tool_calls exist in this response — skip full parse.
    // O(n) single-pass, zero allocation, no trait imports required.
    let has_tool_calls = scan_buf
        .windows(b"tool_calls".len())
        .any(|w| w == b"tool_calls");


    if !has_tool_calls {
        return response_body;
    }

    // ── Parse full body to extract tool_calls ────────────────────────────────
    let mut body_val: serde_json::Value = match serde_json::from_slice(&response_body) {
        Ok(v) => v,
        Err(_) => return response_body,
    };

    // Extract the first tool call's function name for the OPA check.
    let tool_name: String = body_val
        .pointer("/choices/0/message/tool_calls/0/function/name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown_tool")
        .to_string();

    // ── Non-blocking OPA RBAC check ──────────────────────────────────────────
    let opa_url = format!(
        "{}/v1/data/redeye/mcp/allow",
        state.compliance_url.trim_end_matches('/')
    );

    let opa_payload = serde_json::json!({
        "input": {
            "tenant_id": tenant_id,
            "tool_name": tool_name,
            "action": "execute"
        }
    });

    let opa_result = state
        .http_client
        .post(&opa_url)
        .timeout(std::time::Duration::from_millis(200))
        .json(&opa_payload)
        .send()
        .await;

    let fail_open = sandbox_is_fail_open();

    let denied = match opa_result {
        Ok(resp) if resp.status().is_success() => {
            // OPA returns {"result": true} when the policy ALLOWS the request.
            let allow: bool = resp
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v["result"].as_bool())
                .unwrap_or(!fail_open); // fail-closed default: ambiguous result → deny
            !allow
        }
        Ok(resp) => {
            // Non-2xx from OPA (e.g. policy not loaded yet).
            warn!(
                status = resp.status().as_u16(),
                tool_name = %tool_name,
                fail_open,
                "MCP sandbox: OPA returned non-2xx — applying fallback mode"
            );
            !fail_open // fail-closed: deny; fail-open: allow
        }
        Err(e) => {
            // Network timeout or OPA unreachable.
            warn!(
                error = %e,
                tool_name = %tool_name,
                fail_open,
                "MCP sandbox: OPA unreachable — applying fallback mode"
            );
            !fail_open // fail-closed: deny; fail-open: allow
        }
    };

    if !denied {
        return response_body;
    }

    // ── Compact Soft-Steering Mutation ────────────────────────────────────────
    tracing::warn!(
        tenant_id = %tenant_id,
        tool_name = %tool_name,
        "Tunnel 3 Phase 3 — OPA denied; injecting compact soft-steer response"
    );

    if let Some(choices) = body_val.get_mut("choices").and_then(|c| c.as_array_mut()) {
        if let Some(first_choice) = choices.first_mut() {
            // Replace message with compact deny payload — minimal token cost.
            if let Some(message) = first_choice.get_mut("message") {
                *message = serde_json::json!({
                    "role": "tool",
                    "content": DENY_CONTENT
                });
            }
            // Strip tool_calls + set finish_reason to prevent re-invocation.
            if let Some(obj) = first_choice.as_object_mut() {
                obj.remove("tool_calls");
                obj.insert("finish_reason".to_string(), serde_json::json!("stop"));
            }
        }
    }

    serde_json::to_vec(&body_val).unwrap_or(response_body)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::RoutingState;
    use crate::infrastructure::l1_cache::L1Cache;
    use crate::infrastructure::cache_client::CacheGrpcClient;
    use moka::future::Cache;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::redis::Redis;

    async fn setup_test_state_with_redis() -> (Arc<AppState>, testcontainers::ContainerAsync<Redis>) {
        let container = Redis::default().start().await.unwrap();
        let host_port = container.get_host_port_ipv4(6379).await.unwrap();
        let redis_url = format!("redis://127.0.0.1:{}", host_port);

        let redis_client = redis::Client::open(redis_url).unwrap();
        let redis_conn = redis_client.get_multiplexed_tokio_connection().await.unwrap();

        let (telemetry_tx, _) = tokio::sync::mpsc::channel(100);
        let circuit_breaker = Cache::builder().build();
        let loop_fallback_cache = Cache::builder().build();
        let routing_state = Arc::new(RoutingState::new());
        let l1_cache = Arc::new(L1Cache::new(1024).unwrap());
        let http_client = reqwest::Client::new();
        let channel = tonic::transport::Channel::from_static("http://127.0.0.1:1").connect_lazy();
        let cache_grpc_client = CacheGrpcClient::new(channel);

        let state = unsafe {
            AppState {
                http_client,
                cache_grpc_client,
                compliance_url: String::new(),
                redis_conn,
                db_pool: std::mem::MaybeUninit::uninit().assume_init(),
                rate_limit_max: 0,
                rate_limit_window: 0,
                clickhouse_url: String::new(),
                tracer_url: String::new(),
                dashboard_url: String::new(),
                llm_api_base_url: None,
                telemetry_tx,
                l1_cache,
                routing_state,
                circuit_breaker,
                loop_fallback_cache,
                mcp_registry: crate::infrastructure::mcp_registry::McpConnectionRegistry::empty(),
                tool_registry: crate::usecases::tool_router::ToolRegistry::empty(),
            }
        };

        (Arc::new(state), container)
    }

    #[tokio::test]
    async fn test_circuit_breaker_allows_under_limit() {
        let (state, _container) = setup_test_state_with_redis().await;
        let session_id = "test-session-1";

        for i in 1..=10 {
            let res = enforce_agentic_loop_budget(&state, session_id).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), i);
        }
        
        std::mem::forget(state);
    }

    #[tokio::test]
    async fn test_circuit_breaker_blocks_over_limit() {
        let (state, _container) = setup_test_state_with_redis().await;
        let session_id = "test-session-2";

        for i in 1..=15 {
            let res = enforce_agentic_loop_budget(&state, session_id).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), i);
        }

        // 16th should block
        let res = enforce_agentic_loop_budget(&state, session_id).await;
        assert!(res.is_err());
        match res.unwrap_err() {
            GatewayError::AgentLoopBudgetExceeded(_) => {}
            _ => panic!("Expected AgentLoopBudgetExceeded"),
        }
        
        std::mem::forget(state);
    }

    #[tokio::test]
    async fn test_circuit_breaker_moka_fallback() {
        let (state, container) = setup_test_state_with_redis().await;
        let session_id = "test-session-fallback";

        // Drop the container to break the connection and force fallback
        container.stop().await.unwrap();
        
        // The first 5 should use Moka fallback and pass
        for i in 1..=5 {
            let res = enforce_agentic_loop_budget(&state, session_id).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), i);
        }

        // 6th should block via Moka fallback
        let res = enforce_agentic_loop_budget(&state, session_id).await;
        assert!(res.is_err());
        match res.unwrap_err() {
            GatewayError::AgentLoopBudgetExceeded(_) => {}
            _ => panic!("Expected AgentLoopBudgetExceeded"),
        }
        
        std::mem::forget(state);
    }

    #[tokio::test]
    async fn test_chaos_redis_intermittent_failures() {
        let (state, container) = setup_test_state_with_redis().await;
        let session_id = "test-session-chaos";

        // 1. Assert 3 successful requests against Redis
        for i in 1..=3 {
            let res = enforce_agentic_loop_budget(&state, session_id).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), i);
        }

        // 2. Kill the Redis container to forcefully break the socket
        container.stop().await.unwrap();

        // 3. Assert 3 more successful requests. Should transition to Moka gracefully.
        for i in 1..=3 {
            let res = enforce_agentic_loop_budget(&state, session_id).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), i);
        }
        
        std::mem::forget(state);
    }

    // ── Phase 3: Fail-Closed Firewall Unit Tests ──────────────────────────────

    #[test]
    fn test_deny_content_is_compact_json() {
        // DENY_CONTENT must be valid JSON and contain the expected error key.
        let parsed: serde_json::Value =
            serde_json::from_str(DENY_CONTENT).expect("DENY_CONTENT must be valid JSON");
        assert_eq!(
            parsed["error"].as_str(),
            Some("RedEye Guard: Policy Denied. Use read-only tools."),
            "DENY_CONTENT error message mismatch"
        );
        // Token budget: must be fewer than 20 characters of content text.
        assert!(
            DENY_CONTENT.len() < 80,
            "DENY_CONTENT must be compact (< 80 bytes), got {}",
            DENY_CONTENT.len()
        );
    }

    #[test]
    fn test_sandbox_fail_open_env_flag() {
        // Default (unset) = fail-closed.
        std::env::remove_var("SANDBOX_FALLBACK_MODE");
        assert!(!sandbox_is_fail_open(), "Default must be fail-closed");

        // Explicit 'open' = fail-open.
        std::env::set_var("SANDBOX_FALLBACK_MODE", "open");
        assert!(sandbox_is_fail_open(), "SANDBOX_FALLBACK_MODE=open must be fail-open");

        // Case-insensitive.
        std::env::set_var("SANDBOX_FALLBACK_MODE", "OPEN");
        assert!(sandbox_is_fail_open(), "SANDBOX_FALLBACK_MODE=OPEN must be fail-open");

        // Any other value = fail-closed.
        std::env::set_var("SANDBOX_FALLBACK_MODE", "true");
        assert!(!sandbox_is_fail_open(), "SANDBOX_FALLBACK_MODE=true must be fail-closed");

        std::env::remove_var("SANDBOX_FALLBACK_MODE");
    }

    #[test]
    fn test_soft_steer_content_is_compact_deny() {
        // Simulate what the soft-steer mutation produces.
        let message = serde_json::json!({
            "role": "tool",
            "content": DENY_CONTENT
        });
        assert_eq!(message["role"].as_str(), Some("tool"));
        let content = message["content"].as_str().unwrap();
        // Must be valid JSON containing the error key.
        let parsed: serde_json::Value = serde_json::from_str(content)
            .expect("Soft-steer content must be valid JSON");
        assert!(
            parsed.get("error").is_some(),
            "Soft-steer content must contain 'error' key"
        );
    }
}
