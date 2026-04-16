// Dashboard View — SecurityView
// Security Command Center: stat cards, blocked-threats chart, live threat feed.
// Theme: slate-950/indigo-500.

import { ShieldAlert, Flame, DollarSign, Loader2, AlertCircle } from 'lucide-react';
import { StatCard } from '../components/ui/StatCard';
import useSWR from 'swr';
import {
  BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend,
} from 'recharts';

// ── Types ────────────────────────────────────────────────────────────────────

interface Alert {
  timestamp: string;
  session_id: string;
  reason: string;
  severity: 'Critical' | 'High' | 'Medium';
}

interface SecurityData {
  total_blocked_loops: number;
  total_burn_rate_blocks: number;
  estimated_savings_usd: number;
  daily_blocks: { date: string; loops: number; burn_rate: number }[];
  recent_alerts: Alert[];
}

// ── Helpers ──────────────────────────────────────────────────────────────────

const fetcher = async (url: string) => {
  // Authentication handled via HttpOnly cookies (credentials: 'include')
  const res = await fetch(url, { 
    credentials: 'include',
    headers: { 'Content-Type': 'application/json' }
  });
  if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
  return res.json();
};

const SEVERITY_STYLES: Record<string, string> = {
  Critical: 'bg-rose-500/15 text-rose-400 border-rose-500/30',
  High:     'bg-amber-500/15 text-amber-400 border-amber-500/30',
  Medium:   'bg-indigo-500/15 text-indigo-400 border-indigo-500/30',
};

function formatTime(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: true });
}

// ── Component ────────────────────────────────────────────────────────────────

export function SecurityView() {
  const { data, error, isLoading } = useSWR<SecurityData>(
    'http://localhost:8080/v1/admin/security/alerts',
    fetcher,
    {
      refreshInterval: 5000,
      errorRetryCount: 3,
      onErrorRetry: (error, _key, _config, revalidate, { retryCount }) => {
        const msg: string = error?.message ?? '';
        if (msg.includes('401') || msg.includes('403')) return;
        if (retryCount >= 3) return;
        setTimeout(() => revalidate({ retryCount }), 5000);
      },
    },
  );

  return (
    <div className="space-y-6 animate-in fade-in duration-500">
      {/* Header */}
      <header className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl sm:text-3xl lg:text-4xl font-extrabold tracking-tight bg-gradient-to-r from-rose-600 to-indigo-600 dark:from-rose-400 dark:via-amber-300 dark:to-indigo-400 bg-clip-text text-transparent pb-1">
            Security Command Center
          </h1>
          <p className="text-xs sm:text-sm text-slate-500 dark:text-slate-400 mt-1">
            AI Agent threat detection &amp; cost abuse prevention
          </p>
        </div>
        <div className="flex items-center space-x-2 glass-panel bg-white/80 dark:bg-slate-900/50 border border-slate-200/60 dark:border-slate-700/50 shadow-sm backdrop-blur-md dark:shadow-none px-3 py-1.5 sm:px-4 sm:py-2 rounded-full self-start sm:self-auto w-fit transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
          {isLoading && !data ? (
            <Loader2 className="w-4 h-4 text-indigo-500 dark:text-indigo-400 animate-spin" />
          ) : (
            <div className={`w-2 h-2 sm:w-3 sm:h-3 rounded-full ${error ? 'bg-rose-500' : 'bg-emerald-500 neon-dot'}`} />
          )}
          <span className="text-xs sm:text-sm font-medium text-slate-600 dark:text-slate-300">
            {isLoading && !data ? 'Loading…' : error ? 'Offline' : 'Monitoring Active'}
          </span>
        </div>
      </header>

      {/* Stat Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 sm:gap-4">
        <StatCard
          title="Agent Loops Blocked"
          value={isLoading && !data ? '…' : data?.total_blocked_loops ?? 0}
          icon={ShieldAlert}
          accentClass="text-rose-400 ring-1 ring-rose-400/20"
          subtitle="Recursive loop circuit-breaker"
        />
        <StatCard
          title="Runaway Spenders Blocked"
          value={isLoading && !data ? '…' : data?.total_burn_rate_blocks ?? 0}
          icon={Flame}
          accentClass="text-amber-400 ring-1 ring-amber-400/20"
          subtitle="Burn rate velocity limiter"
        />
        <div className="sm:col-span-2 lg:col-span-1">
          <StatCard
            title="Estimated $ Saved"
            value={isLoading && !data ? '…' : `$${(data?.estimated_savings_usd ?? 0).toFixed(2)}`}
            icon={DollarSign}
            accentClass="text-emerald-400 ring-1 ring-emerald-400/20"
            subtitle="Cost abuse prevention"
          />
        </div>
      </div>

      {/* Error banner */}
      {error && !data && (
        <div className="glass-panel bg-rose-50 dark:bg-rose-500/5 border border-rose-200 dark:border-rose-500/20 p-4 flex items-center gap-3 shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
          <AlertCircle className="w-5 h-5 text-rose-500 flex-shrink-0" />
          <p className="text-sm text-rose-700 dark:text-slate-300">Failed to connect to security endpoint. Showing stale or zeroed data.</p>
        </div>
      )}

      {/* Chart + Table */}
      <div className="grid grid-cols-1 lg:grid-cols-5 gap-4 sm:gap-6">
        {/* Bar Chart — 2/5 width */}
        <div className="glass-panel bg-white/80 dark:bg-slate-900/40 border border-slate-200/60 dark:border-slate-700/50 p-4 sm:p-6 lg:col-span-2 shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
          <h2 className="text-lg sm:text-xl font-bold text-slate-900 dark:text-slate-100 mb-6 flex items-center gap-2">
            Blocked Threats / Day
            <span className="text-[10px] text-amber-600 dark:text-amber-400 bg-amber-100 dark:bg-amber-500/10 px-1.5 py-0.5 rounded font-mono uppercase tracking-tighter">7-day</span>
          </h2>
          <div className="h-[280px] w-full min-h-[280px]">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={data?.daily_blocks ?? []} barGap={4}>
                <CartesianGrid strokeDasharray="3 3" stroke="#e2e8f0" vertical={false} />
                <XAxis dataKey="date" stroke="#64748b" fontSize={10} tickLine={false} axisLine={false} />
                <YAxis stroke="#64748b" fontSize={10} tickLine={false} axisLine={false} />
                <Tooltip
                  contentStyle={{ backgroundColor: '#0f172a', borderColor: '#1e293b', borderRadius: '8px', fontSize: '12px' }}
                />
                <Legend iconType="circle" wrapperStyle={{ fontSize: '10px', paddingTop: '12px' }} />
                <Bar dataKey="loops" name="Loop Blocks" fill="#e11d48" radius={[4, 4, 0, 0]} animationDuration={1200} />
                <Bar dataKey="burn_rate" name="Burn Rate Blocks" fill="#d97706" radius={[4, 4, 0, 0]} animationDuration={1200} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* Live Threat Feed — 3/5 width */}
        <div className="glass-panel bg-white/80 dark:bg-slate-900/40 border border-slate-200/60 dark:border-slate-700/50 p-4 sm:p-6 lg:col-span-3 flex flex-col overflow-hidden shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
          <h2 className="text-lg sm:text-xl font-bold text-slate-900 dark:text-slate-100 mb-4 flex items-center justify-between">
            Live Threat Feed
            <span className="text-[10px] text-rose-600 dark:text-rose-400 bg-rose-100 dark:bg-rose-500/10 px-1.5 py-0.5 rounded font-mono uppercase tracking-widest animate-pulse">Live</span>
          </h2>
          <div className="overflow-x-auto w-full flex-1 custom-scrollbar">
            <table className="w-full text-sm text-left">
              <thead>
                <tr className="border-b border-slate-200 dark:border-slate-800/60 text-xs text-slate-500 uppercase tracking-wider">
                  <th className="py-2.5 px-3 font-medium">Time</th>
                  <th className="py-2.5 px-3 font-medium">Session</th>
                  <th className="py-2.5 px-3 font-medium">Threat</th>
                  <th className="py-2.5 px-3 font-medium">Severity</th>
                </tr>
              </thead>
              <tbody>
                {(data?.recent_alerts ?? []).map((alert, i) => (
                  <tr
                    key={`${alert.session_id}-${i}`}
                    className="border-b border-slate-100 dark:border-slate-800/30 hover:bg-slate-50 dark:hover:bg-slate-800/20 transition-colors"
                  >
                    <td className="py-2.5 px-3 font-mono text-xs text-slate-500 dark:text-slate-400">{formatTime(alert.timestamp)}</td>
                    <td className="py-2.5 px-3 font-mono text-xs text-indigo-600 dark:text-indigo-300">{alert.session_id}</td>
                    <td className="py-2.5 px-3 text-slate-700 dark:text-slate-200">{alert.reason}</td>
                    <td className="py-2.5 px-3">
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase tracking-wide border ${
                          SEVERITY_STYLES[alert.severity] ?? SEVERITY_STYLES.Medium
                        }`}
                      >
                        {alert.severity}
                      </span>
                    </td>
                  </tr>
                ))}
                {(!data || data.recent_alerts.length === 0) && (
                  <tr>
                    <td colSpan={4} className="py-8 text-center text-slate-500 text-sm">
                      No recent alerts
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}
