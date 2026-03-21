// Dashboard View — DashboardView
// Renders live traffic charts, stat cards, model distribution, latency histogram,
// and live request audit log. Mirrors original renderDashboard() from App.tsx.
// Theme: slate-950/indigo-500.

import { Activity, Zap, ShieldAlert, Cpu, DollarSign, Loader2 } from 'lucide-react';
import { StatCard } from '../components/ui/StatCard';
import useSWR from 'swr';

interface Metrics {
  total_requests: string;
  avg_latency_ms: number;
  total_tokens: string;
  rate_limited_requests: string;
}

const fetcher = async (url: string) => {
  const token = localStorage.getItem('re_token');
  if (!token) throw new Error("No authentication token found");
  const res = await fetch(url, { headers: { 'Authorization': `Bearer ${token}` } });
  if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
  return res.json();
};



export function DashboardView() {
  const { data: metrics, error, isLoading } = useSWR<Metrics>(
    'http://localhost:8080/v1/admin/metrics',
    fetcher,
    { refreshInterval: 3000, errorRetryCount: 3 }
  );

  const calculateSavedCost = () => {
    if (!metrics) return '0.00';
    return (parseInt(metrics.rate_limited_requests || '0') * 0.005).toFixed(2);
  };

  return (
    <div className="space-y-6 animate-in fade-in duration-500">
      {/* Header */}
      <header className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl sm:text-3xl lg:text-4xl font-extrabold tracking-tight bg-gradient-to-r from-indigo-400 to-slate-200 bg-clip-text text-transparent pb-1">
            RedEye Gateway
          </h1>
          <p className="text-xs sm:text-sm text-slate-400 mt-1">Enterprise Telemetry &amp; Security Command Center</p>
        </div>
        <div className="flex items-center space-x-2 glass-panel px-3 py-1.5 sm:px-4 sm:py-2 rounded-full self-start sm:self-auto w-fit">
          {isLoading && !metrics ? (
            <Loader2 className="w-4 h-4 text-indigo-400 animate-spin" />
          ) : (
            <div className={`w-2 h-2 sm:w-3 sm:h-3 rounded-full ${error ? 'bg-rose-500' : 'bg-emerald-500 neon-dot'}`} />
          )}
          <span className="text-xs sm:text-sm font-medium text-slate-300">
            {isLoading && !metrics ? 'Connecting...' : error ? 'System Offline' : 'Live Sync Active'}
          </span>
        </div>
      </header>

      {/* Stat Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5 gap-3 sm:gap-4">
        <StatCard title="Total Traffic" value={isLoading ? '...' : metrics?.total_requests ?? '0'} icon={Activity} accentClass="text-indigo-400 ring-1 ring-indigo-400/20" />
        <StatCard title="Avg Latency" value={isLoading ? '...' : `${Math.round(metrics?.avg_latency_ms ?? 0)} ms`} icon={Zap} accentClass="text-violet-400 ring-1 ring-violet-400/20" />
        <StatCard title="Tokens Processed" value={isLoading ? '...' : metrics?.total_tokens ?? '0'} icon={Cpu} accentClass="text-sky-400 ring-1 ring-sky-400/20" />
        <StatCard title="Threats Blocked" value={isLoading ? '...' : metrics?.rate_limited_requests ?? '0'} icon={ShieldAlert} accentClass="text-rose-400 ring-1 ring-rose-400/20" />
        <div className="sm:col-span-2 lg:col-span-1 xl:col-span-1">
          <StatCard
            title="API Cost Saved"
            value={`$${calculateSavedCost()}`}
            icon={DollarSign}
            accentClass="text-emerald-400 ring-1 ring-emerald-400/20"
            subtitle="Prevented by Gateway"
          />
        </div>
      </div>

      {/* Charts Row 1 */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6">
        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-4 sm:p-6 lg:col-span-2">
          <h2 className="text-lg sm:text-xl font-bold text-slate-100 mb-2 sm:mb-4">Live Traffic Overview</h2>
          <div className="h-[200px] sm:h-[250px] w-full flex items-center justify-center text-slate-500 text-sm border-2 border-dashed border-slate-800 rounded-lg">
            Time series endpoint not yet implemented
          </div>
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-4 sm:p-6 lg:col-span-1">
          <h2 className="text-lg sm:text-xl font-bold text-slate-100 mb-2 sm:mb-4">Model Distribution</h2>
          <div className="h-[200px] sm:h-[250px] w-full flex items-center justify-center text-slate-500 text-sm border-2 border-dashed border-slate-800 rounded-lg">
            Distribution endpoint not yet implemented
          </div>
        </div>
      </div>

      {/* Charts Row 2 */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6">
        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-4 sm:p-6 lg:col-span-1">
          <h2 className="text-lg sm:text-xl font-bold text-slate-100 mb-2 sm:mb-4">Latency Histogram</h2>
          <div className="h-[200px] sm:h-[250px] w-full flex items-center justify-center text-slate-500 text-sm border-2 border-dashed border-slate-800 rounded-lg">
            Histogram endpoint not yet implemented
          </div>
        </div>

        <div className="glass-panel bg-slate-900/40 border border-slate-800/80 p-4 sm:p-6 lg:col-span-2 flex flex-col overflow-hidden">
          <h2 className="text-lg sm:text-xl font-bold text-slate-100 mb-2 sm:mb-4">Live Request Audit Log</h2>
          <div className="overflow-x-auto w-full pb-2 custom-scrollbar flex-1 min-h-[150px] flex items-center justify-center border-2 border-dashed border-slate-800 rounded-lg">
             <span className="text-slate-500 text-sm">Audit log streaming not yet implemented</span>
          </div>
        </div>
      </div>
    </div>
  );
}
