// ─── Routing Map Types ────────────────────────────────────────────────────────

export type NodeStatus = 'active' | 'degraded' | 'error' | 'offline' | 'standby';
export type EdgeStatus = 'flowing' | 'broken' | 'fallback' | 'idle';
export type ModelTier = 'primary' | 'secondary' | 'tertiary';

export interface LLMNodeMetrics {
  rpm: number;          // Requests per minute
  avgLatencyMs: number; // Average latency in milliseconds
  cacheHitPct: number;  // Cache hit percentage (0–100)
  tps: number;          // Tokens per second (for edge tooltip)
  errorRate: number;    // Error rate percentage (0–100)
  uptime: number;       // Uptime percentage (0–100)
}

export interface LLMNodeData {
  label: string;         // Display name, e.g., "GPT-4o"
  provider: string;      // e.g., "OpenAI"
  model: string;         // e.g., "gpt-4o"
  tier: ModelTier;
  status: NodeStatus;
  metrics: LLMNodeMetrics;
  providerColor: string; // Accent color for the provider badge
}

export interface GatewayNodeData {
  label: string;
  totalRpm: number;
  activeRoutes: number;
  fallbacksActive: number;
  uptime: number;
}

export interface AnimatedEdgeData {
  status: EdgeStatus;
  tps: number;
  isActive: boolean;
}

// ─── Mock Data ────────────────────────────────────────────────────────────────

export const MOCK_LLM_NODES: Array<{ id: string; data: LLMNodeData; position: { x: number; y: number } }> = [
  {
    id: 'gpt4o',
    position: { x: 520, y: 20 },
    data: {
      label: 'GPT-4o',
      provider: 'OpenAI',
      model: 'gpt-4o',
      tier: 'primary',
      status: 'error',
      providerColor: '#10a37f',
      metrics: { rpm: 0, avgLatencyMs: 0, cacheHitPct: 0, tps: 0, errorRate: 100, uptime: 0 },
    },
  },
  {
    id: 'claude35',
    position: { x: 520, y: 140 },
    data: {
      label: 'Claude 3.5 Sonnet',
      provider: 'Anthropic',
      model: 'claude-3-5-sonnet',
      tier: 'secondary',
      status: 'active',
      providerColor: '#d97706',
      metrics: { rpm: 1420, avgLatencyMs: 310, cacheHitPct: 44, tps: 2340, errorRate: 0.1, uptime: 99.9 },
    },
  },
  {
    id: 'gemini15',
    position: { x: 520, y: 260 },
    data: {
      label: 'Gemini 1.5 Pro',
      provider: 'Google',
      model: 'gemini-1.5-pro',
      tier: 'primary',
      status: 'active',
      providerColor: '#4285F4',
      metrics: { rpm: 890, avgLatencyMs: 420, cacheHitPct: 31, tps: 1750, errorRate: 0.3, uptime: 99.7 },
    },
  },
  {
    id: 'gpt4turbo',
    position: { x: 520, y: 380 },
    data: {
      label: 'GPT-4 Turbo',
      provider: 'OpenAI',
      model: 'gpt-4-turbo',
      tier: 'secondary',
      status: 'standby',
      providerColor: '#10a37f',
      metrics: { rpm: 120, avgLatencyMs: 680, cacheHitPct: 18, tps: 340, errorRate: 0.5, uptime: 99.5 },
    },
  },
  {
    id: 'claude3opus',
    position: { x: 520, y: 500 },
    data: {
      label: 'Claude 3 Opus',
      provider: 'Anthropic',
      model: 'claude-3-opus',
      tier: 'tertiary',
      status: 'standby',
      providerColor: '#d97706',
      metrics: { rpm: 45, avgLatencyMs: 1100, cacheHitPct: 12, tps: 180, errorRate: 0.8, uptime: 98.9 },
    },
  },
  {
    id: 'mistral',
    position: { x: 520, y: 620 },
    data: {
      label: 'Mistral Large',
      provider: 'Mistral AI',
      model: 'mistral-large',
      tier: 'primary',
      status: 'active',
      providerColor: '#FF6B35',
      metrics: { rpm: 670, avgLatencyMs: 295, cacheHitPct: 52, tps: 1420, errorRate: 0.2, uptime: 99.8 },
    },
  },
  {
    id: 'llama3',
    position: { x: 520, y: 740 },
    data: {
      label: 'Llama 3.1 70B',
      provider: 'Meta',
      model: 'llama-3.1-70b',
      tier: 'secondary',
      status: 'active',
      providerColor: '#0668E1',
      metrics: { rpm: 540, avgLatencyMs: 380, cacheHitPct: 38, tps: 1180, errorRate: 0.4, uptime: 99.6 },
    },
  },
  {
    id: 'cohere',
    position: { x: 520, y: 860 },
    data: {
      label: 'Command R+',
      provider: 'Cohere',
      model: 'command-r-plus',
      tier: 'tertiary',
      status: 'degraded',
      providerColor: '#8B5CF6',
      metrics: { rpm: 210, avgLatencyMs: 740, cacheHitPct: 22, tps: 490, errorRate: 3.2, uptime: 96.8 },
    },
  },
  {
    id: 'geminiflash',
    position: { x: 820, y: 80 },
    data: {
      label: 'Gemini 1.5 Flash',
      provider: 'Google',
      model: 'gemini-1.5-flash',
      tier: 'primary',
      status: 'active',
      providerColor: '#4285F4',
      metrics: { rpm: 1800, avgLatencyMs: 180, cacheHitPct: 61, tps: 4200, errorRate: 0.1, uptime: 99.9 },
    },
  },
  {
    id: 'gpt35turbo',
    position: { x: 820, y: 200 },
    data: {
      label: 'GPT-3.5 Turbo',
      provider: 'OpenAI',
      model: 'gpt-3.5-turbo',
      tier: 'secondary',
      status: 'active',
      providerColor: '#10a37f',
      metrics: { rpm: 3200, avgLatencyMs: 120, cacheHitPct: 72, tps: 6100, errorRate: 0.1, uptime: 99.9 },
    },
  },
  {
    id: 'mixtral',
    position: { x: 820, y: 320 },
    data: {
      label: 'Mixtral 8x7B',
      provider: 'Mistral AI',
      model: 'mixtral-8x7b',
      tier: 'tertiary',
      status: 'active',
      providerColor: '#FF6B35',
      metrics: { rpm: 420, avgLatencyMs: 310, cacheHitPct: 45, tps: 920, errorRate: 0.6, uptime: 99.4 },
    },
  },
  {
    id: 'deepseek',
    position: { x: 820, y: 440 },
    data: {
      label: 'DeepSeek V3',
      provider: 'DeepSeek',
      model: 'deepseek-v3',
      tier: 'primary',
      status: 'active',
      providerColor: '#06b6d4',
      metrics: { rpm: 760, avgLatencyMs: 260, cacheHitPct: 49, tps: 1890, errorRate: 0.3, uptime: 99.7 },
    },
  },
  {
    id: 'qwen',
    position: { x: 820, y: 560 },
    data: {
      label: 'Qwen2.5 72B',
      provider: 'Alibaba',
      model: 'qwen2.5-72b',
      tier: 'secondary',
      status: 'standby',
      providerColor: '#FF6900',
      metrics: { rpm: 90, avgLatencyMs: 520, cacheHitPct: 15, tps: 210, errorRate: 1.0, uptime: 98.5 },
    },
  },
  {
    id: 'perplexity',
    position: { x: 820, y: 680 },
    data: {
      label: 'Sonar Large',
      provider: 'Perplexity',
      model: 'sonar-large',
      tier: 'tertiary',
      status: 'active',
      providerColor: '#20B2AA',
      metrics: { rpm: 330, avgLatencyMs: 410, cacheHitPct: 28, tps: 720, errorRate: 0.7, uptime: 99.3 },
    },
  },
  {
    id: 'o1mini',
    position: { x: 820, y: 800 },
    data: {
      label: 'o1-mini',
      provider: 'OpenAI',
      model: 'o1-mini',
      tier: 'secondary',
      status: 'degraded',
      providerColor: '#10a37f',
      metrics: { rpm: 88, avgLatencyMs: 2200, cacheHitPct: 8, tps: 145, errorRate: 4.1, uptime: 95.9 },
    },
  },
];

export const MOCK_GATEWAY_DATA: GatewayNodeData = {
  label: 'RedEye Gateway',
  totalRpm: 12480,
  activeRoutes: 14,
  fallbacksActive: 1,
  uptime: 99.94,
};
export const MODEL_METADATA_MAP: Record<string, Partial<LLMNodeData>> = {
  'gpt-4o': { label: 'GPT-4o', provider: 'OpenAI', tier: 'primary', providerColor: '#10a37f' },
  'claude-3-5-sonnet': { label: 'Claude 3.5 Sonnet', provider: 'Anthropic', tier: 'secondary', providerColor: '#d97706' },
  'gemini-1.5-pro': { label: 'Gemini 1.5 Pro', provider: 'Google', tier: 'primary', providerColor: '#4285F4' },
  'gpt-4-turbo': { label: 'GPT-4 Turbo', provider: 'OpenAI', tier: 'secondary', providerColor: '#10a37f' },
  'claude-3-opus': { label: 'Claude 3 Opus', provider: 'Anthropic', tier: 'tertiary', providerColor: '#d97706' },
  'mistral-large': { label: 'Mistral Large', provider: 'Mistral AI', tier: 'primary', providerColor: '#FF6B35' },
  'llama-3.1-70b': { label: 'Llama 3.1 70B', provider: 'Meta', tier: 'secondary', providerColor: '#0668E1' },
  'command-r-plus': { label: 'Command R+', provider: 'Cohere', tier: 'tertiary', providerColor: '#8B5CF6' },
  'gemini-1.5-flash': { label: 'Gemini 1.5 Flash', provider: 'Google', tier: 'primary', providerColor: '#4285F4' },
  'gpt-3.5-turbo': { label: 'GPT-3.5 Turbo', provider: 'OpenAI', tier: 'secondary', providerColor: '#10a37f' },
};
