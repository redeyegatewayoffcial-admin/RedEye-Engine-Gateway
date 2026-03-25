// Dashboard View — CacheView
// Semantic Cache: hit/miss ratios + insights panel.

import { Database } from 'lucide-react';
import useSWR from 'swr';
import { CACHE_METRICS_URL, fetchCacheMetrics } from '../../data/services/metricsService';

export function CacheView() {
  const { data: cacheStats, error, isLoading } = useSWR(CACHE_METRICS_URL, fetchCacheMetrics, {
    refreshInterval: 10000,
  });

  if (isLoading) {
    return (
      <div className="space-y-6 animate-pulse">
        <header>
          <div className="w-32 h-4 bg-slate-800 rounded mb-2"></div>
          <div className="w-64 h-8 bg-slate-800 rounded"></div>
        </header>
        <div className="h-48 bg-slate-900/40 border border-slate-800/80 rounded"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6 bg-rose-950/20 border border-rose-900/50 rounded-lg text-rose-400">
        <p className="font-semibold text-sm">Failed to load Cache Metrics</p>
        <p className="text-xs mt-1">{error.message}</p>
      </div>
    );
  }

  const stats = cacheStats || { hit_ratio: 0, miss_ratio: 0, total_lookups: 0 };

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between gap-4">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">Performance</p>
          <h1 className="text-2xl sm:text-3xl font-bold text-slate-50">Semantic Cache</h1>
          <p className="text-sm text-slate-400 mt-1">
            Measure cache efficiency for embeddings and model responses across tenants.
          </p>
        </div>
        <div className="hidden sm:flex items-center gap-2 text-slate-400 text-xs font-medium">
          <Database className="w-4 h-4 text-emerald-400" />
          <span>Cache @8081</span>
        </div>
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6">
        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5 lg:col-span-2">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h2 className="text-sm font-semibold text-slate-100">Hit / Miss Overview</h2>
              <p className="text-xs text-slate-500">Recent semantic lookups across all services.</p>
            </div>
          </div>

          <div className="grid grid-cols-3 gap-4">
            <div>
              <p className="text-xs font-medium text-slate-400 mb-1">Hit Ratio</p>
              <p className="text-3xl font-semibold text-emerald-400">
                {(stats.hit_ratio * 100).toFixed(1)}%
              </p>
            </div>
            <div>
              <p className="text-xs font-medium text-slate-400 mb-1">Miss Ratio</p>
              <p className="text-3xl font-semibold text-rose-400">
                {(stats.miss_ratio * 100).toFixed(1)}%
              </p>
            </div>
            <div>
              <p className="text-xs font-medium text-slate-400 mb-1">Total Lookups</p>
              <p className="text-3xl font-semibold text-slate-100">
                {stats.total_lookups.toLocaleString()}
              </p>
            </div>
          </div>

          <p className="text-xs text-slate-500 mt-4">
            High cache hit ratios reduce both end-to-end latency and downstream token spend for LLM calls.
          </p>
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-5 lg:col-span-1">
          <h2 className="text-sm font-semibold text-slate-100 mb-2">Cache Insights</h2>
          <ul className="space-y-2 text-xs text-slate-400">
            <li>• Embedding similarity thresholds tuned for low drift.</li>
            <li>• Multi-tenant partitioning prevents cross-tenant cache bleed.</li>
            <li>• Cache warming applied for high-traffic tenants.</li>
          </ul>
        </div>
      </div>
    </div>
  );
}
