// Shared mock data — moved from App.tsx monolith
// All views import from here; no stale inline constants.

export const mockLatencyData = [
  { bucket: '0-50ms', count: 420 },
  { bucket: '50-100ms', count: 210 },
  { bucket: '100-200ms', count: 80 },
  { bucket: '200-500ms', count: 30 },
  { bucket: '500ms+', count: 10 },
];

export const mockModelData = [
  { name: 'GPT-4o', value: 65 },
  { name: 'Gemini-2.5', value: 25 },
  { name: 'Claude-3', value: 10 },
];

export const CHART_COLORS = ['#6366f1', '#818cf8', '#a5b4fc'];

export const mockAuditLogs = [
  { id: 'req-982', tenant: 'acme-corp', model: 'gpt-4o', status: 200, latency: '42ms', time: 'Just now' },
  { id: 'req-981', tenant: 'globex-inc', model: 'gemini-2.5', status: 429, latency: '12ms', time: '2s ago' },
  { id: 'req-980', tenant: 'acme-corp', model: 'gpt-4o', status: 200, latency: '85ms', time: '5s ago' },
  { id: 'req-979', tenant: 'stark-ind', model: 'claude-3.5', status: 200, latency: '110ms', time: '12s ago' },
  { id: 'req-978', tenant: 'globex-inc', model: 'gpt-4o', status: 429, latency: '8ms', time: '15s ago' },
];

export const mockRedactedEntities = 1287;

export const mockResidencyRoutes = [
  { region: 'us-east', endpoint: 'https://gateway.us-east.redeye', isolation: 'Strict' },
  { region: 'eu-central', endpoint: 'https://gateway.eu-central.redeye', isolation: 'Strict' },
  { region: 'ap-south', endpoint: 'https://gateway.ap-south.redeye', isolation: 'Relaxed' },
];

export const mockTraces = [
  { traceId: 'trace-9012', tenantId: 'acme-corp', policy: 'Allowed', latency: '48ms' },
  { traceId: 'trace-9011', tenantId: 'globex-inc', policy: 'Blocked', latency: '15ms' },
  { traceId: 'trace-9010', tenantId: 'stark-ind', policy: 'Allowed', latency: '92ms' },
];

export const mockCacheStats = {
  hitRatio: 0.82,
  missRatio: 0.18,
  totalLookups: 42319,
};

export const mockApiKeys = [
  { id: 'key-1', name: 'Production Backend', maskedKey: 're_live_••••••••••••••••••••••••••••x9a2', createdAt: '2026-03-01T10:00:00Z', status: 'Active' },
  { id: 'key-2', name: 'Staging Environment', maskedKey: 're_live_••••••••••••••••••••••••••••b8b1', createdAt: '2026-03-10T14:30:00Z', status: 'Active' },
  { id: 'key-3', name: 'Developer Test Key', maskedKey: 're_live_••••••••••••••••••••••••••••f4c7', createdAt: '2026-02-15T09:15:00Z', status: 'Revoked' },
];
