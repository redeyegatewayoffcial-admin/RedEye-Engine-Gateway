// Dashboard View — CacheView
// Semantic Cache: hit/miss ratios + insights panel.

import { Database } from 'lucide-react';
import { mockCacheStats } from '../../data/repositories/mockData';

export function CacheView() {
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
                {(mockCacheStats.hitRatio * 100).toFixed(1)}%
              </p>
            </div>
            <div>
              <p className="text-xs font-medium text-slate-400 mb-1">Miss Ratio</p>
              <p className="text-3xl font-semibold text-rose-400">
                {(mockCacheStats.missRatio * 100).toFixed(1)}%
              </p>
            </div>
            <div>
              <p className="text-xs font-medium text-slate-400 mb-1">Total Lookups</p>
              <p className="text-3xl font-semibold text-slate-100">
                {mockCacheStats.totalLookups.toLocaleString()}
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
