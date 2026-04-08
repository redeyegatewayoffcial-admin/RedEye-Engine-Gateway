// Data Service — Metrics API calls to RedEye Gateway
// Fetches real-time token usage and estimated cost from GET /v1/admin/metrics/usage.
// NOTE: All authenticated requests use credentials: 'include' to send HttpOnly cookies automatically.

import { parseApiError } from '../utils/apiErrors';

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
 * Generic fetcher that standardizes authentication via HttpOnly cookies,
 * prevents CORS / 500 errors, and handles HTTP exceptions cleanly.
 * Uses credentials: 'include' to automatically send HttpOnly cookies.
 * Returns standardized errors for consistent error handling.
 */
async function fetchMetrics<T>(url: string): Promise<T> {
  const res = await fetch(url, {
    credentials: 'include', // Sends HttpOnly auth cookies automatically
    headers: {
      'Content-Type': 'application/json',
    },
  });

  if (!res.ok) {
    const error = await parseApiError(res);
    throw error;
  }
  return res.json() as Promise<T>;
}

/**
 * SWR-compatible fetcher for /v1/admin/metrics/usage.
 *
 * Authentication is handled automatically via HttpOnly cookies (credentials: 'include').
 * Returns graceful defaults `{ total_tokens: 0, estimated_cost: 0 }` rather
 * than throwing when ClickHouse is empty — the backend already handles that.
 *
 * Complexity: O(1) — single HTTP round-trip, fixed-size payload.
 */
export async function fetchUsageMetrics(url: string): Promise<UsageMetrics> {
  return fetchMetrics<UsageMetrics>(url);
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
  return fetchMetrics<BillingBreakdown[]>(url);
}

/**
 * Shape of the /v1/admin/metrics/cache response.
 */
export interface CacheMetrics {
  hit_ratio: number;
  miss_ratio: number;
  total_lookups: number;
}

export const CACHE_METRICS_URL = `${GATEWAY_URL}/v1/admin/metrics/cache`;

export async function fetchCacheMetrics(url: string): Promise<CacheMetrics> {
  return fetchMetrics<CacheMetrics>(url);
}

/**
 * Shape of the /v1/admin/metrics/compliance response.
 */
export interface ResidencyRoute {
  region: string;
  endpoint: string;
  isolation: 'Strict' | 'Relaxed';
}

export interface ComplianceMetrics {
  redacted_count: number;
  residency_routes: ResidencyRoute[];
}

export const COMPLIANCE_METRICS_URL = `${GATEWAY_URL}/v1/admin/metrics/compliance`;

export async function fetchComplianceMetrics(url: string): Promise<ComplianceMetrics> {
  return fetchMetrics<ComplianceMetrics>(url);
}
