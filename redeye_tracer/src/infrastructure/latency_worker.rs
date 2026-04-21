//! infrastructure/latency_worker.rs — Background EMA P95 latency cron for LLM provider ranking.
//!
//! Queries ClickHouse every `interval_secs` seconds for P95 latency per executed_provider,
//! applies Exponential Moving Average (EMA) smoothing, and pushes a sorted ranking to Redis.
//! The gateway reads this ZSET at O(1) during request routing.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tracing::{debug, info, warn};

use crate::infrastructure::clickhouse_repo::ClickHouseRepo;

/// Redis key for the latency-sorted provider ranking.
const REDIS_KEY: &str = "redeye:latency:rankings";

/// TTL in seconds for the Redis key (auto-expires if the worker dies).
const REDIS_TTL_SECS: u64 = 30;

/// Default EMA smoothing factor (α). Higher = more weight on recent observations.
const DEFAULT_ALPHA: f64 = 0.3;

/// Default polling interval in seconds.
const DEFAULT_INTERVAL_SECS: u64 = 10;

/// Background worker that maintains EMA-smoothed P95 latency rankings in Redis.
pub struct LatencyWorker {
    repo: Arc<ClickHouseRepo>,
    redis_conn: redis::aio::MultiplexedConnection,
    interval_secs: u64,
    alpha: f64,
    ema_state: HashMap<String, f64>,
}

impl LatencyWorker {
    /// Creates a new `LatencyWorker`.
    pub fn new(
        repo: Arc<ClickHouseRepo>,
        redis_conn: redis::aio::MultiplexedConnection,
    ) -> Self {
        let interval_secs = std::env::var("LATENCY_WORKER_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_INTERVAL_SECS);

        Self {
            repo,
            redis_conn,
            interval_secs,
            alpha: DEFAULT_ALPHA,
            ema_state: HashMap::new(),
        }
    }

    /// Runs the worker loop forever. Designed to be spawned via `tokio::spawn`.
    pub async fn run(mut self) {
        info!(
            interval_secs = self.interval_secs,
            alpha = self.alpha,
            "Starting latency ranking worker"
        );

        let mut interval = tokio::time::interval(
            std::time::Duration::from_secs(self.interval_secs),
        );
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Skip the first immediate tick so we don't query before data accumulates.
        interval.tick().await;

        let mut consecutive_failures = 0;

        loop {
            interval.tick().await;

            if let Err(e) = self.tick().await {
                consecutive_failures += 1;
                
                // Log on the 1st failure, and every ~60 seconds thereafter (if interval is 10s)
                if consecutive_failures == 1 || consecutive_failures % 6 == 0 {
                    warn!(
                        error = %e,
                        consecutive_failures,
                        "Latency worker tick failed (will retry next interval, suppressing redundant logs)"
                    );
                }
            } else {
                if consecutive_failures > 0 {
                    info!("Latency worker recovered after {} failures", consecutive_failures);
                }
                consecutive_failures = 0;
            }
        }
    }

    /// Single tick: query → EMA → push to Redis.
    async fn tick(&mut self) -> Result<(), String> {
        let raw_latencies = self.query_p95_latencies().await?;

        if raw_latencies.is_empty() {
            debug!("No latency data from ClickHouse — skipping ZSET update");
            return Ok(());
        }

        // Apply EMA smoothing to each provider.
        for (provider, raw_p95) in &raw_latencies {
            let ema = compute_ema(
                self.ema_state.get(provider).copied(),
                *raw_p95,
                self.alpha,
            );
            self.ema_state.insert(provider.clone(), ema);
        }

        // Build the ZADD payload: (score, member) pairs.
        let entries: Vec<(f64, &str)> = self
            .ema_state
            .iter()
            .map(|(provider, ema)| (*ema, provider.as_str()))
            .collect();

        if entries.is_empty() {
            return Ok(());
        }

        // Atomic replace: DEL + ZADD + EXPIRE in a pipeline.
        let mut pipe = redis::pipe();
        pipe.atomic()
            .del(REDIS_KEY)
            .ignore();

        // Add entries one by one via the pipeline.
        for (score, member) in &entries {
            pipe.zadd(REDIS_KEY, *member, *score).ignore();
        }
        pipe.expire(REDIS_KEY, REDIS_TTL_SECS as i64).ignore();

        pipe.query_async::<()>(&mut self.redis_conn)
            .await
            .map_err(|e| format!("Redis pipeline failed: {}", e))?;

        info!(
            providers = entries.len(),
            "Updated latency rankings in Redis"
        );

        for (score, member) in &entries {
            debug!(provider = member, ema_p95_ms = score, "Latency ranking entry");
        }

        Ok(())
    }

    /// Queries ClickHouse for P95 latency per `executed_provider` over the last
    /// `interval_secs` seconds — exactly matching the polling cadence so each tick
    /// reads only *new* rows with zero overlap.
    ///
    /// Bug 7 Fix: The original query used a hardcoded `INTERVAL 5 MINUTE` window
    /// against a 10-second polling loop. This caused 96% of the data to be queried
    /// repeatedly on every tick, creating massive CPU spikes in ClickHouse under load.
    /// By scoping the window to `interval_secs`, each tick is a non-overlapping slice.
    async fn query_p95_latencies(&self) -> Result<HashMap<String, f64>, String> {
        // Use the worker's own polling interval as the query window for zero-overlap reads.
        let interval = self.interval_secs;
        let query = format!(
            r#"
            SELECT
                executed_provider,
                quantile(0.95)(latency_ms) AS p95
            FROM RedEye_telemetry.request_logs
            WHERE created_at >= now() - INTERVAL {interval} SECOND
              AND status = 200
              AND executed_provider != ''
            GROUP BY executed_provider
            FORMAT JSON
            "#
        );

        let resp = self
            .repo
            .raw_query(&query)
            .await?;

        let mut result = HashMap::new();

        if let Some(rows) = resp["data"].as_array() {
            for row in rows {
                let provider = match row["executed_provider"].as_str() {
                    Some(p) if !p.is_empty() => p.to_string(),
                    _ => continue,
                };

                let p95: f64 = match &row["p95"] {
                    Value::String(s) => s.parse().unwrap_or(0.0),
                    Value::Number(n) => n.as_f64().unwrap_or(0.0),
                    _ => continue,
                };

                if p95 > 0.0 {
                    result.insert(provider, p95);
                }
            }
        }

        Ok(result)
    }
}

/// Computes the Exponential Moving Average.
///
/// - If `previous` is `None` (cold start), the raw value becomes the initial EMA.
/// - Otherwise: `ema = α * raw + (1 - α) * previous`
pub fn compute_ema(previous: Option<f64>, raw: f64, alpha: f64) -> f64 {
    match previous {
        None => raw,
        Some(prev) => alpha * raw + (1.0 - alpha) * prev,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_cold_start() {
        // First observation becomes the EMA value.
        let ema = compute_ema(None, 200.0, 0.3);
        assert!((ema - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ema_smoothing() {
        // Second observation: ema = 0.3 * 100 + 0.7 * 200 = 30 + 140 = 170
        let ema = compute_ema(Some(200.0), 100.0, 0.3);
        assert!((ema - 170.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ema_converges_toward_stable_value() {
        // Simulate 10 observations of the same value — EMA should converge.
        let mut ema = compute_ema(None, 500.0, 0.3);
        for _ in 0..20 {
            ema = compute_ema(Some(ema), 100.0, 0.3);
        }
        // After 20 iterations at a constant 100, should be very close to 100.
        assert!((ema - 100.0).abs() < 1.0, "EMA did not converge: {}", ema);
    }

    #[test]
    fn test_ema_spike_reaction() {
        // Start at 100, sudden spike to 1000.
        let ema1 = compute_ema(Some(100.0), 1000.0, 0.3);
        // Should be 0.3*1000 + 0.7*100 = 300 + 70 = 370
        assert!((ema1 - 370.0).abs() < f64::EPSILON);

        // Next tick back to normal (100):
        let ema2 = compute_ema(Some(ema1), 100.0, 0.3);
        // 0.3*100 + 0.7*370 = 30 + 259 = 289
        assert!((ema2 - 289.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ema_alpha_boundaries() {
        // α = 0: ignore raw, keep previous
        let ema = compute_ema(Some(200.0), 100.0, 0.0);
        assert!((ema - 200.0).abs() < f64::EPSILON);

        // α = 1: ignore previous, use raw
        let ema = compute_ema(Some(200.0), 100.0, 1.0);
        assert!((ema - 100.0).abs() < f64::EPSILON);
    }
}
