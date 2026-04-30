import { parseApiError } from '../utils/apiErrors';
import type { LlmModel, ProviderKey } from '../../domain/entities/Config';

// ── Service base URLs ─────────────────────────────────────────────────────────
// provider-keys (secrets)  → redeye_auth  :8084
// llm_models / routing     → redeye_config :8085

const AUTH_BASE   = 'http://localhost:8084/v1/auth';
const CONFIG_BASE = 'http://localhost:8085/v1/config';

// ── Shared fetch helper ───────────────────────────────────────────────────────

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(url, {
    ...options,
    credentials: 'include',
    headers: {
      'Content-Type': 'application/json',
      'x-csrf-token': '1',
      ...options?.headers,
    },
  });

  if (!res.ok) {
    const error = await parseApiError(res);
    throw error;
  }

  // Handle 204 No Content
  if (res.status === 204) {
    return undefined as unknown as T;
  }

  return res.json() as Promise<T>;
}

// ── configService ─────────────────────────────────────────────────────────────

export const configService = {
  // ── Provider keys (owned by redeye_auth) ───────────────────────────────────
  // Auth stores the encrypted secret; the list endpoint returns metadata only.

  /** List all provider keys for the current user's tenant. */
  async getProviderKeys(): Promise<ProviderKey[]> {
    return fetchJson<ProviderKey[]>(`${AUTH_BASE}/provider-keys`);
  },

  /** Add a new provider key. Calls Auth only (secret storage). */
  async addProviderKey(payload: {
    provider_name: string;
    api_key: string;
    key_alias: string;
  }): Promise<ProviderKey> {
    return fetchJson<ProviderKey>(`${AUTH_BASE}/provider-keys`, {
      method: 'POST',
      body: JSON.stringify(payload),
    });
  },

  // ── LLM models (owned by redeye_config) ────────────────────────────────────

  /**
   * List all LLM models registered for the tenant.
   * The tenantId is read from the JWT on the server; no path param needed here
   * because the config service scopes it via the auth middleware.
   */
  async getModels(tenantId: string): Promise<LlmModel[]> {
    return fetchJson<LlmModel[]>(`${CONFIG_BASE}/${tenantId}/models`);
  },

  // ── Routing mesh (owned by redeye_config) ───────────────────────────────────

  /**
   * Register or update a model→key routing entry and trigger a Redis publish.
   * This is the Phase 2 call after `addProviderKey` succeeds.
   */
  async upsertRoutingMesh(
    tenantId: string,
    payload: {
      model_name: string;
      base_url: string;
      schema_format: string;
      key_alias: string;
      priority: number;
      weight: number;
    }
  ): Promise<{ model_name: string; message: string }> {
    return fetchJson(`${CONFIG_BASE}/${tenantId}/routing-mesh`, {
      method: 'POST',
      body: JSON.stringify(payload),
    });
  },

  /**
   * Update routing priority / weight for a specific model's keys.
   * Publishes a fresh routing mesh snapshot to Redis.
   */
  async updateRoutingPolicy(
    tenantId: string,
    payload: {
      model_name: string;
      strategy: 'manual_priority' | 'auto_weighted';
      keys: { key_alias: string; priority: number; weight: number }[];
    }
  ): Promise<void> {
    // We update each key individually by calling the routing-mesh endpoint once
    // per key with the new priority/weight, then let the backend republish.
    for (const k of payload.keys) {
      await fetchJson(`${CONFIG_BASE}/${tenantId}/routing-mesh`, {
        method: 'POST',
        body: JSON.stringify({
          model_name:    payload.model_name,
          schema_format: 'openai',       // will be preserved from DB on update
          key_alias:     k.key_alias,
          base_url:      '',             // kept blank — server does NOT overwrite base_url if empty
          priority:      k.priority,
          weight:        k.weight,
        }),
      });
    }
  },
};
