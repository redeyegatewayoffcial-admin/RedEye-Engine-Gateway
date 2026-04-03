// Dashboard View — DashboardView
// Renders live traffic charts, stat cards, model distribution, latency histogram,
// and live request audit log. Theme: Cool Revival (Midnight Obsidian + Neon Cyan/Teal).
// Upgraded with framer-motion stagger entrance animations + product tour anchors.

import { Activity, Zap, ShieldAlert, Cpu, DollarSign, Loader2, AlertCircle } from 'lucide-react';
import { StatCard } from '../components/ui/StatCard';
import { TutorialOverlay } from '../components/TutorialOverlay';
import useSWR from 'swr';
import { motion } from 'framer-motion';
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
  PieChart, Pie, Cell, BarChart, Bar, Legend
} from 'recharts';
import { fetchUsageMetrics, USAGE_METRICS_URL, type UsageMetrics } from '../../data/services/metricsService';
import { HotSwapLiveChart } from './HotSwapLiveChart';

interface Metrics {
  total_requests: string;
  avg_latency_ms: number;
  total_tokens: string;
  rate_limited_requests: string;
  traffic_series: { timestamp: string; requests: number }[];
  model_distribution: { name: string; value: number }[];
  latency_buckets: { bucket: string; count: number }[];
}

const CHART_COLORS = ['#22d3ee', '#2dd4bf', '#818cf8', '#ec4899', '#f59e0b'];

const fetcher = async (url: string) => {
  // Authentication handled via HttpOnly cookies (credentials: 'include')
  const res = await fetch(url, { 
    credentials: 'include',
    headers: { 'Content-Type': 'application/json' }
  });
  if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
  return res.json();
};

/** Formats a token count with locale-aware thousands separators (e.g. 1,234,567). */
const formatTokens = (n: number): string => n.toLocaleString('en-US');

/** Formats a cost to USD string (e.g. $0.0025). */
const formatCost = (n: number): string => `$${n.toFixed(4)}`;

// Framer-motion variants
const containerVariants = {
  hidden: {},
  show: { transition: { staggerChildren: 0.08 } },
} as const;

const fadeUpVariant = {
  hidden: { opacity: 0, y: 20 },
  show: { opacity: 1, y: 0, transition: { duration: 0.45, ease: [0.25, 0.1, 0.25, 1] as [number, number, number, number] } },
};

export function DashboardView() {
  const { data: metrics, error, isLoading } = useSWR<Metrics>(
    'http://localhost:8080/v1/admin/metrics',
    fetcher,
    { refreshInterval: 3000, errorRetryCount: 3 }
  );

  const {
    data: usageMetrics,
    isLoading: isUsageLoading,
  } = useSWR<UsageMetrics>(
    USAGE_METRICS_URL,
    fetchUsageMetrics,
    { refreshInterval: 30_000, errorRetryCount: 3 }
  );

  return (
    <>
      {/* Product Tour — renders nothing visually, fires driver.js once */}
      <TutorialOverlay />

      <motion.div
        variants={containerVariants}
        initial="hidden"
        animate="show"
        className="space-y-6"
      >
        {/* ── Header ─────────────────────────────────────────────── */}
        <motion.header
          variants={fadeUpVariant}
          className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4"
        >
          <div>
            <h1 className="text-2xl sm:text-3xl lg:text-4xl font-extrabold tracking-tight bg-gradient-to-r from-cyan-600 to-teal-500 dark:from-cyan-400 dark:to-teal-300 bg-clip-text text-transparent pb-1">
              RedEye Gateway
            </h1>
            <p className="text-xs sm:text-sm text-slate-500 dark:text-slate-400 mt-1">
              Enterprise Telemetry &amp; Security Command Center
            </p>
          </div>

          {/* Live Sync Badge */}
          <div className="flex items-center space-x-2 glass-panel bg-white/80 dark:bg-slate-900/50 border border-slate-200/60 dark:border-slate-800/80 px-3 py-1.5 sm:px-4 sm:py-2 rounded-full self-start sm:self-auto w-fit shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
            {isLoading && !metrics ? (
              <Loader2 className="w-4 h-4 text-cyan-600 dark:text-cyan-400 animate-spin" />
            ) : (
              <div className={`w-2 h-2 sm:w-3 sm:h-3 rounded-full ${error ? 'bg-rose-500' : 'bg-emerald-500 neon-dot'}`} />
            )}
            <span className="text-xs sm:text-sm font-medium text-slate-600 dark:text-slate-300">
              {isLoading && !metrics ? 'Connecting...' : error ? 'System Offline' : 'Live Sync Active'}
            </span>
          </div>
        </motion.header>

        {/* ── Stat Cards ─────────────────────────────────────────── */}
        <div id="tour-stat-cards" className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5 gap-3 sm:gap-4">
          <StatCard index={0} title="Total Traffic" value={isLoading && !metrics ? '...' : metrics?.total_requests ?? '0'} icon={Activity} accentClass="text-cyan-400 ring-1 ring-cyan-400/20" />
          <StatCard index={1} title="Avg Latency" value={isLoading && !metrics ? '...' : `${Math.round(metrics?.avg_latency_ms ?? 0)} ms`} icon={Zap} accentClass="text-violet-400 ring-1 ring-violet-400/20" />
          <StatCard
            index={2}
            title="Tokens Processed"
            value={(isUsageLoading && !usageMetrics) ? '...' : formatTokens(usageMetrics?.total_tokens ?? 0)}
            icon={Cpu}
            accentClass="text-sky-400 ring-1 ring-sky-400/20"
          />
          <StatCard index={3} title="Threats Blocked" value={isLoading && !metrics ? '...' : metrics?.rate_limited_requests ?? '0'} icon={ShieldAlert} accentClass="text-rose-400 ring-1 ring-rose-400/20" />
          <div className="sm:col-span-2 lg:col-span-1 xl:col-span-1">
            <StatCard
              index={4}
              title="Est. Token Cost"
              value={(isUsageLoading && !usageMetrics) ? '...' : formatCost(usageMetrics?.estimated_cost ?? 0)}
              icon={DollarSign}
              accentClass="text-emerald-400 ring-1 ring-emerald-400/20"
              subtitle="Live · $0.002 / 1K tokens"
            />
          </div>
        </div>

        {/* Error Banner */}
        {error && !metrics && (
          <motion.div variants={fadeUpVariant} className="glass-panel bg-rose-50 dark:bg-rose-500/5 border border-rose-200 dark:border-rose-500/20 p-4 flex items-center gap-3 shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
            <AlertCircle className="w-5 h-5 text-rose-500 flex-shrink-0" />
            <p className="text-sm text-rose-700 dark:text-slate-300">Connection to backend metrics failed. Showing stale or zeroed data.</p>
          </motion.div>
        )}

        {/* ── Hot-Swap Chart & Controls ───────────────────────────────────────── */}
        <motion.div variants={fadeUpVariant} className="grid grid-cols-1 lg:grid-cols-4 gap-4 sm:gap-6 mt-6">
          <div className="lg:col-span-3">
            <HotSwapLiveChart />
          </div>
          <div className="lg:col-span-1 glass-panel bg-white/80 dark:bg-slate-900/40 border border-slate-200/60 dark:border-slate-800/80 p-4 sm:p-6 flex flex-col justify-between rounded-xl shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
            <div>
              <h2 className="text-lg sm:text-xl font-bold text-slate-900 dark:text-slate-100 mb-2 flex items-center gap-2">
                <AlertCircle className="w-5 h-5 text-orange-500" />
                Chaos Engineering
              </h2>
              <p className="text-sm text-slate-600 dark:text-slate-400 mb-6">
                Test zero-downtime routing. This will simulate a 503 Service Unavailable error from the primary OpenAI provider.
              </p>
            </div>
            
            <button
              onClick={async () => {
                try {
                  // Authentication handled via HttpOnly cookies
                  await fetch('http://localhost:8080/v1/admin/toggle-chaos', {
                    method: 'POST',
                    credentials: 'include', // Sends HttpOnly cookies automatically
                    headers: { 'Content-Type': 'application/json' }
                  });
                } catch (e) {
                  console.error('Failed to trigger chaos', e);
                }
              }}
              className="w-full py-3 px-4 bg-orange-100 dark:bg-orange-500/10 hover:bg-orange-200 dark:hover:bg-orange-500/20 text-orange-600 dark:text-orange-400 border border-orange-300 dark:border-orange-500/50 hover:border-orange-400 dark:hover:border-orange-500 rounded-lg font-bold transition-all duration-200 active:scale-95 ease-in-out focus:outline-none focus:ring-2 focus:ring-orange-500/50 focus:ring-offset-1 dark:focus:ring-offset-slate-900 flex items-center justify-center gap-2"
            >
              <Zap className="w-4 h-4 fill-current" />
              Simulate OpenAI Outage
            </button>
          </div>
        </motion.div>

        {/* ── Charts Row 1 ───────────────────────────────────────── */}
        <motion.div variants={fadeUpVariant} className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6 mt-6">
          {/* Traffic Chart */}
          <div id="tour-traffic-chart" className="glass-panel bg-white/80 dark:bg-slate-900/40 border border-slate-200/60 dark:border-slate-800/80 p-4 sm:p-6 lg:col-span-2 shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
            <h2 className="text-lg sm:text-xl font-bold text-slate-900 dark:text-slate-100 mb-6 flex items-center gap-2">
              Live Traffic Overview
              <span className="text-[10px] text-cyan-600 dark:text-cyan-400 bg-cyan-100 dark:bg-cyan-500/10 px-1.5 py-0.5 rounded font-mono uppercase tracking-tighter">Real-time</span>
            </h2>
            <div className="h-[250px] w-full min-h-[250px]">
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={metrics?.traffic_series && metrics.traffic_series.length > 0 ? metrics.traffic_series : []}>
                  <defs>
                    <linearGradient id="colorRequests" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#22d3ee" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="#22d3ee" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke="#e2e8f0" vertical={false} />
                  <XAxis
                    dataKey="timestamp"
                    stroke="#64748b"
                    fontSize={10}
                    tickLine={false}
                    axisLine={false}
                    tickFormatter={(val) => val.split('T')[1]?.substring(0, 5) ?? val}
                  />
                  <YAxis stroke="#64748b" fontSize={10} tickLine={false} axisLine={false} />
                  <Tooltip
                    contentStyle={{ backgroundColor: '#0f172a', borderColor: 'rgba(34,211,238,0.2)', borderRadius: '10px', fontSize: '12px' }}
                    itemStyle={{ color: '#22d3ee' }}
                  />
                  <Area
                    type="monotone"
                    dataKey="requests"
                    stroke="#22d3ee"
                    fillOpacity={1}
                    fill="url(#colorRequests)"
                    strokeWidth={2}
                    animationDuration={1500}
                  />
                </AreaChart>
              </ResponsiveContainer>
            </div>
          </div>

          {/* Model Distribution */}
          <div className="glass-panel bg-white/80 dark:bg-slate-900/40 border border-slate-200/60 dark:border-slate-800/80 p-4 sm:p-6 lg:col-span-1 flex flex-col shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
            <h2 className="text-lg sm:text-xl font-bold text-slate-900 dark:text-slate-100 mb-6">Model Distribution</h2>
            <div className="h-[250px] w-full flex-1 min-h-[250px]">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={metrics?.model_distribution && metrics.model_distribution.length > 0 ? metrics.model_distribution : []}
                    cx="50%"
                    cy="50%"
                    innerRadius={60}
                    outerRadius={80}
                    paddingAngle={5}
                    dataKey="value"
                    animationDuration={1000}
                  >
                    {(metrics?.model_distribution || []).map((_, index) => (
                      <Cell key={`cell-${index}`} fill={CHART_COLORS[index % CHART_COLORS.length]} stroke="none" />
                    ))}
                  </Pie>
                  <Tooltip
                    contentStyle={{ backgroundColor: '#0f172a', borderColor: 'rgba(34,211,238,0.2)', borderRadius: '10px', fontSize: '12px' }}
                  />
                  <Legend iconType="circle" wrapperStyle={{ fontSize: '10px', paddingTop: '20px' }} />
                </PieChart>
              </ResponsiveContainer>
            </div>
          </div>
        </motion.div>

        {/* ── Charts Row 2 ───────────────────────────────────────── */}
        <motion.div variants={fadeUpVariant} className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6">
          {/* Latency Histogram */}
          <div className="glass-panel bg-white/80 dark:bg-slate-900/40 border border-slate-200/60 dark:border-slate-800/80 p-4 sm:p-6 lg:col-span-1 shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
            <h2 className="text-lg sm:text-xl font-bold text-slate-900 dark:text-slate-100 mb-6">Latency Histogram</h2>
            <div className="h-[250px] w-full min-h-[250px]">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={metrics?.latency_buckets && metrics.latency_buckets.length > 0 ? metrics.latency_buckets : []}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#e2e8f0" vertical={false} />
                  <XAxis dataKey="bucket" stroke="#64748b" fontSize={10} tickLine={false} axisLine={false} />
                  <YAxis stroke="#64748b" fontSize={10} tickLine={false} axisLine={false} />
                  <Tooltip
                    contentStyle={{ backgroundColor: '#0f172a', borderColor: 'rgba(45,212,191,0.2)', borderRadius: '10px', fontSize: '12px' }}
                  />
                  <Bar
                    dataKey="count"
                    fill="#2dd4bf"
                    radius={[4, 4, 0, 0]}
                    animationDuration={1200}
                  />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </div>

          {/* Audit Log */}
          <div className="glass-panel bg-white/80 dark:bg-slate-900/40 border border-slate-200/60 dark:border-slate-800/80 p-4 sm:p-6 lg:col-span-2 flex flex-col overflow-hidden shadow-sm backdrop-blur-md dark:shadow-none transition-all duration-300 hover:shadow-md hover:-translate-y-0.5">
            <h2 className="text-lg sm:text-xl font-bold text-slate-900 dark:text-slate-100 mb-4 flex items-center justify-between">
              Live Request Audit Log
              <span className="text-[10px] text-emerald-600 dark:text-emerald-400 bg-emerald-100 dark:bg-emerald-500/10 px-1.5 py-0.5 rounded font-mono uppercase tracking-widest animate-pulse">Live</span>
            </h2>
            <div className="overflow-x-auto w-full pb-2 custom-scrollbar flex-1 min-h-[190px] flex flex-col justify-center border border-slate-200 dark:border-slate-800/50 rounded-xl bg-slate-50 dark:bg-slate-950/20">
              <div className="text-center py-8">
                <Activity className="w-8 h-8 text-slate-400 dark:text-slate-700 mx-auto mb-3 animate-pulse" />
                <span className="text-slate-500 text-sm font-medium">Tracing active spans...</span>
                <p className="text-[10px] text-slate-400 dark:text-slate-600 mt-1 uppercase tracking-[0.2em]">Audit log data buffered at gateway</p>
              </div>
            </div>
          </div>
        </motion.div>
      </motion.div>
    </>
  );
}
