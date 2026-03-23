// Data Service — Metrics API calls to RedEye Gateway
// Fetches real-time token usage and estimated cost from GET /v1/admin/metrics/usage.

const GATEWAY_URL = 'http://localhost:8080';

/**
 * Shape of the /v1/admin/metrics/usage response.
 *
 * @field total_tokens   - Aggregate token consumption for the tenant (u64 from Rust).
 * @field estimated_cost - Cost in USD at $0.002 per 1,000 tokens (4 d.p. precision).
 */
export interface UsageMetrics {
  total_tokens: number;
  estimated_cost: number;
}

/**
 * SWR-compatible fetcher for /v1/admin/metrics/usage.
 *
 * Reads the Bearer token from localStorage (set on login by authService).
 * Returns graceful defaults `{ total_tokens: 0, estimated_cost: 0 }` rather
 * than throwing when ClickHouse is empty — the backend already handles that.
 *
 * Complexity: O(1) — single HTTP round-trip, fixed-size payload.
 */
export async function fetchUsageMetrics(url: string): Promise<UsageMetrics> {
  const token = localStorage.getItem('re_token');
  if (!token) {
    throw new Error('No authentication token found — please log in.');
  }

  const res = await fetch(url, {
    headers: { Authorization: `Bearer ${token}` },
  });

  if (!res.ok) {
    const body = await res.text().catch(() => res.statusText);
    throw new Error(`Usage metrics fetch failed (${res.status}): ${body}`);
  }

  return res.json() as Promise<UsageMetrics>;
}

/**
 * Shape of a single row from /v1/admin/billing/breakdown.
 */
export interface BillingBreakdown {
  date: string;
  model: string;
  total_tokens: number;
  estimated_cost: number;
}

/** Fully qualified URL used as the SWR cache key. */
export const USAGE_METRICS_URL = `${GATEWAY_URL}/v1/admin/metrics/usage`;
export const BILLING_BREAKDOWN_URL = `${GATEWAY_URL}/v1/admin/billing/breakdown`;

/**
 * SWR-compatible fetcher for /v1/admin/billing/breakdown.
 */
export async function fetchBillingBreakdown(url: string): Promise<BillingBreakdown[]> {
  const token = localStorage.getItem('re_token');
  if (!token) {
    throw new Error('No authentication token found — please log in.');
  }

  const res = await fetch(url, {
    headers: { Authorization: `Bearer ${token}` },
  });

  if (!res.ok) {
    const body = await res.text().catch(() => res.statusText);
    throw new Error(`Billing breakdown fetch failed (${res.status}): ${body}`);
  }

  return res.json() as Promise<BillingBreakdown[]>;
}
