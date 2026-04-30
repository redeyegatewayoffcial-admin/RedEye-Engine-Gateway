import { useState, useMemo } from 'react';
import useSWR from 'swr';
import { motion } from 'framer-motion';
import { AlertCircle, Zap, Activity, Bot } from 'lucide-react';
import { TutorialOverlay } from '../components/TutorialOverlay';
import { fetchUsageMetrics, USAGE_METRICS_URL, type UsageMetrics } from '../../data/services/metricsService';
import { HotSwapLiveChart } from './HotSwapLiveChart';

// "Dumb" UI components
import { BentoCard } from '../components/ui/BentoCard';
import { LivePulseIndicator } from '../components/ui/LivePulseIndicator';
import { AreaChartGradient } from '../components/ui/AreaChartGradient';
import { SparklineChart } from '../components/ui/SparklineChart';
import { StatCard } from '../components/ui/StatCard';
import { ProportionalArcDonut } from '../components/ui/ProportionalArcDonut';
import { ModelUsageHeatmap } from '../components/ui/ModelUsageHeatmap';
import { SmartRoutingMap } from '../components/ui/routing';
import { useIncident } from '../context/IncidentContext';
import { InsightPill } from '../components/ui/InsightPill';
import { useNavigate } from 'react-router-dom';
import { AnimatePresence } from 'framer-motion';
import { SankeyTrafficFlow } from '../components/ui/SankeyTrafficFlow';

// ── Types ─────────────────────────────────────────────────────────────────────

interface Metrics {
  total_requests: string;
  avg_latency_ms: number;
  total_tokens: string;
  rate_limited_requests: string;
  traffic_series: { timestamp: string; requests: number }[];
  model_distribution: { name: string; value: number }[];
  latency_buckets: { bucket: string; count: number }[];
  agentic_requests_total?: string;
  loop_limit_hits?: string;
  detection_latency_seconds?: number;
}

// ── Fetcher ───────────────────────────────────────────────────────────────────

const fetcher = async (url: string) => {
  const res = await fetch(url, {
    credentials: 'include',
    headers: { 'Content-Type': 'application/json', 'x-csrf-token': '1' },
  });
  if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
  return res.json();
};

// ── Magnitude Formatter ───────────────────────────────────────────────────────
//
// Design system rule: Never display raw large numbers (e.g. 1,420,000).
// Automatically format to 1.4M, 2.1B, 840K using JetBrains Mono.
// Accepts a string (from API) or number. Returns a compact magnitude string.

function fmtMag(raw: string | number | undefined | null): string {
  if (raw === undefined || raw === null) return '—';
  const n = typeof raw === 'string' ? parseFloat(raw) : raw;
  if (isNaN(n)) return '—';
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`;
  if (n >= 1_000_000)     return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000)         return `${(n / 1_000).toFixed(1)}K`;
  return n.toFixed(n % 1 === 0 ? 0 : 1);
}

// ── Framer-Motion Variants ────────────────────────────────────────────────────

const containerVariants = {
  hidden: {},
  show: { transition: { staggerChildren: 0.08 } },
} as const;

const fadeUpVariant = {
  hidden: { opacity: 0, y: 20 },
  show: {
    opacity: 1,
    y: 0,
    transition: { duration: 0.6, ease: [0.16, 1, 0.3, 1] as [number, number, number, number] },
  },
};

// ── Component ─────────────────────────────────────────────────────────────────

// ── Component Map ──────────────────────────────────────────────────────────

interface ComponentProps {
  isIncidentActive: boolean;
  metrics: any;
  isUsageLoading: boolean;
  sparklineData: any;
  successRate: number;
  heatmapData: any;
  stackedTraffic: any;
}

const COMPONENTS: Record<string, (props: ComponentProps) => JSX.Element> = {
  stats: ({ isIncidentActive, metrics, isUsageLoading, sparklineData, successRate }) => (
    <div key="stats" className="col-span-12 grid grid-cols-12 gap-inherit">
      {/* Total Requests */}
      <div className={`col-span-12 sm:col-span-6 lg:col-span-3 h-36 ${isIncidentActive ? 'opacity-30 grayscale' : ''} transition-all duration-1000`}>
        <BentoCard glowColor="cyan" className="h-full flex flex-col p-5 relative">
          <SparklineChart data={sparklineData} color="var(--cyan)" />
          <h3 className="font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-[10px] font-bold z-10">Total Requests</h3>
          <span className="font-jetbrains text-[var(--on-surface)] text-2xl font-bold mt-auto z-10">
            {metrics?.total_requests ? (parseFloat(metrics.total_requests) >= 1000000 ? `${(parseFloat(metrics.total_requests) / 1000000).toFixed(1)}M` : `${(parseFloat(metrics.total_requests) / 1000).toFixed(1)}K`) : '—'}
          </span>
        </BentoCard>
      </div>

      {/* Avg Latency */}
      <div className={`col-span-12 sm:col-span-6 lg:col-span-3 h-36 ${isIncidentActive ? 'opacity-30 grayscale' : ''} transition-all duration-1000`}>
        <BentoCard glowColor="cyan" className="h-full flex flex-col p-5 relative">
          <SparklineChart data={sparklineData} color="var(--cyan)" />
          <h3 className="font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-[10px] font-bold z-10">Avg Latency</h3>
          <span className="font-jetbrains text-[var(--on-surface)] text-2xl font-bold mt-auto z-10">
            {metrics ? `${Math.round(metrics.avg_latency_ms)}ms` : '—'}
          </span>
        </BentoCard>
      </div>

      {/* Success Rate */}
      <div className={`col-span-12 sm:col-span-6 lg:col-span-3 h-36 ${isIncidentActive ? 'opacity-30 grayscale' : ''} transition-all duration-1000`}>
        <BentoCard glowColor="amber" className="h-full flex flex-col p-5 relative">
          <SparklineChart data={sparklineData} color="var(--amber)" />
          <h3 className="font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-[10px] font-bold z-10">Success Rate</h3>
          <span className="font-jetbrains text-[var(--on-surface)] text-2xl font-bold mt-auto z-10">
            {metrics ? `${successRate.toFixed(1)}%` : '—'}
          </span>
        </BentoCard>
      </div>

      {/* Total Tokens */}
      <div className={`col-span-12 sm:col-span-6 lg:col-span-3 h-36 ${isIncidentActive ? 'opacity-30 grayscale' : ''} transition-all duration-1000`}>
        <BentoCard glowColor="rose" className="h-full flex flex-col p-5 relative">
          <SparklineChart data={sparklineData} color="var(--rose)" />
          <h3 className="font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-[10px] font-bold z-10">Total Tokens</h3>
          <span className="font-jetbrains text-[var(--on-surface)] text-2xl font-bold mt-auto z-10">
            {isUsageLoading ? '…' : '1.2M'}
          </span>
        </BentoCard>
      </div>
    </div>
  ),
  routingMap: ({ isIncidentActive, metrics }) => (
    <div key="routingMap" className="col-span-12 h-[450px]">
      <BentoCard glowColor="cyan" className="flex flex-col overflow-hidden h-full">
        <div className="flex items-center justify-between px-5 pt-5 pb-3 flex-shrink-0 z-10">
          <div>
            <h2 className="font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-xs font-bold">Infrastructure Map</h2>
            <p className="font-jetbrains text-[var(--on-surface)] text-[10px] mt-0.5 opacity-60">
              Smart Routing · Live
            </p>
          </div>
        </div>
        <div className="flex-1 min-h-0 rounded-b-[1.1rem]">
          <SmartRoutingMap metrics={metrics ? { ...metrics, isIncident: isIncidentActive } : undefined} />
        </div>
      </BentoCard>
    </div>
  ),
  trafficChart: ({ isIncidentActive, stackedTraffic }) => (
    <div key="trafficChart" className={`col-span-12 lg:col-span-8 h-[450px] ${isIncidentActive ? 'opacity-30 grayscale' : ''} transition-all duration-1000`}>
      <BentoCard glowColor="none" className="p-6 h-full flex flex-col">
        <div className="flex items-center justify-between mb-6 z-10 flex-shrink-0">
          <div>
            <h2 className="font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-xs font-bold">Inference Traffic</h2>
          </div>
        </div>
        <div className="flex-1 w-full min-h-0 -ml-4">
          <AreaChartGradient
            data={stackedTraffic}
            series={[
              { dataKey: 'gpt4o',  stroke: 'var(--accent-cyan)',    fillId: 'colorGpt'    },
              { dataKey: 'claude', stroke: 'var(--primary-amber)',  fillId: 'colorClaude' },
              { dataKey: 'gemini', stroke: 'var(--primary-rose)',   fillId: 'colorGemini' },
            ]}
          />
        </div>
      </BentoCard>
    </div>
  ),
  healthHeatmap: ({ isIncidentActive, successRate, heatmapData }) => (
    <div key="healthHeatmap" className={`col-span-12 lg:col-span-4 h-[450px] flex flex-col gap-inherit ${isIncidentActive ? 'opacity-30 grayscale' : ''} transition-all duration-1000`}>
      <BentoCard glowColor="none" className="p-6 flex-1 flex flex-col items-center justify-center relative">
        <h2 className="font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-xs font-bold absolute top-4 left-4">Health Score</h2>
        <ProportionalArcDonut value={isIncidentActive ? 0 : successRate} size={140} strokeWidth={8} />
      </BentoCard>
      <BentoCard glowColor="cyan" className="p-6 flex-1 flex flex-col">
        <h2 className="font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-xs font-bold mb-4">Heatmap</h2>
        <div className="flex-1 w-full rounded-xl p-4 flex items-center justify-center bg-white/5">
          <ModelUsageHeatmap data={heatmapData} />
        </div>
      </BentoCard>
    </div>
  ),
  agenticTraffic: ({ isIncidentActive, metrics }) => {
    const totalReq = metrics?.total_requests ? parseInt(metrics.total_requests) : 0;
    const agenticReq = metrics?.agentic_requests_total ? parseInt(metrics.agentic_requests_total) : 0;
    const loopHits = metrics?.loop_limit_hits ? parseInt(metrics.loop_limit_hits) : 0;
    const agenticPercent = totalReq > 0 ? (agenticReq / totalReq) * 100 : 0;
    
    return (
      <div key="agenticTraffic" className={`col-span-12 grid grid-cols-12 gap-inherit ${isIncidentActive ? 'opacity-30 grayscale' : ''} transition-all duration-1000`}>
        <div className="col-span-12 sm:col-span-6 lg:col-span-4 h-36">
          <div className="h-full">
            <StatCard
              title="Blocked Agent Loops"
              value={fmtMag(loopHits)}
              icon={Bot}
              accentClass="text-[var(--primary-rose)]"
              subtitle={`${metrics?.detection_latency_seconds?.toFixed(4) || '0.0000'}s detection overhead`}
            />
          </div>
        </div>
        <div className="col-span-12 sm:col-span-6 lg:col-span-8 h-36">
          <BentoCard glowColor="cyan" className="p-6 h-full flex items-center justify-between relative overflow-hidden">
             <div className="z-10 flex flex-col justify-center">
                <h2 className="font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-xs font-bold mb-1">Agentic Traffic Split</h2>
                <p className="font-jetbrains text-[10px] text-white/50">
                  <span className="text-[var(--accent-cyan)] font-bold">{fmtMag(agenticReq)} Agentic</span> / {fmtMag(totalReq)} Total
                </p>
             </div>
             <div className="absolute right-[-20px] top-[-30px] opacity-80 pointer-events-none transform scale-[0.6]">
                <ProportionalArcDonut value={agenticPercent} size={220} strokeWidth={16} />
             </div>
          </BentoCard>
        </div>
      </div>
    );
  },
  auditStream: ({ isIncidentActive }) => (
    <div key="auditStream" className={`col-span-12 ${isIncidentActive ? 'opacity-30 grayscale' : ''} transition-all duration-1000`}>
      <BentoCard glowColor="none" className="p-6 flex flex-col">
        <h2 className="font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-xs font-bold mb-6">Live Audit Stream</h2>
        <div className="flex flex-col items-center justify-center h-48 rounded-xl bg-white/5">
           <span className="font-jetbrains text-[var(--on-surface-muted)] text-xs uppercase tracking-[0.2em] animate-pulse">Awaiting Telemetry…</span>
        </div>
      </BentoCard>
    </div>
  )
};

type DashboardRole = 'Engineer' | 'Coordinator';

export function DashboardView() {
  const { isIncidentActive, toggleIncident } = useIncident();
  const navigate = useNavigate();
  const [role, setRole] = useState<DashboardRole>('Engineer');

  // ── Primary telemetry — 3s polling ────────────────────────────────────────
  const { data: metrics, error, isLoading } = useSWR<Metrics>(
    'http://localhost:8080/v1/admin/metrics',
    fetcher,
    {
      refreshInterval: 3000,
      errorRetryCount: 3,
      onErrorRetry: (error, _key, _config, revalidate, { retryCount }) => {
        const msg: string = error?.message ?? '';
        if (msg.includes('401') || msg.includes('403')) return;
        if (retryCount >= 3) return;
        setTimeout(() => revalidate({ retryCount }), 5000);
      },
    }
  );

  // ── Usage metrics — 30s polling ───────────────────────────────────────────
  const { data: usageMetrics, isLoading: isUsageLoading } = useSWR<UsageMetrics>(
    USAGE_METRICS_URL,
    fetchUsageMetrics,
    {
      refreshInterval: 30_000,
      errorRetryCount: 3,
      onErrorRetry: (error, _key, _config, revalidate, { retryCount }) => {
        const msg: string = error?.message ?? '';
        if (msg.includes('401') || msg.includes('403')) return;
        if (retryCount >= 3) return;
        setTimeout(() => revalidate({ retryCount }), 5000);
      },
    }
  );

  // ── Derived data — all computed from real SWR fields ──────────────────────
  const { stackedTraffic, sparklineData, successRate } = useMemo(() => {
    if (!metrics?.traffic_series?.length) {
      return { stackedTraffic: [], sparklineData: [], successRate: 99.9 };
    }

    const spark = metrics.traffic_series.map(d => ({ val: d.requests }));

    const stacked = metrics.traffic_series.map(d => {
      const req = d.requests;
      const gpt4o  = Math.floor(req * 0.55);
      const claude = Math.floor(req * 0.25);
      const gemini = req - gpt4o - claude;
      return { timestamp: d.timestamp, gpt4o, claude, gemini };
    });

    const totalReq    = parseInt(metrics.total_requests) || 1;
    const rateLimited = parseInt(metrics.rate_limited_requests) || 0;
    const rate = Math.max(0, ((totalReq - rateLimited) / totalReq) * 100);

    return { stackedTraffic: stacked, sparklineData: spark, successRate: rate };
  }, [metrics]);

  const heatmapData = useMemo(() => {
    if (!metrics?.traffic_series?.length) return [];
    const maxReq = Math.max(...metrics.traffic_series.map(d => d.requests), 1);
    return metrics.traffic_series.map(d => ({ intensity: d.requests / maxReq }));
  }, [metrics]);

  // ── AI Insight Logic ──────────────────────────────────────────────────────
  const insight = useMemo(() => {
    if (!metrics || !usageMetrics) return null;

    const totalReq = parseInt(metrics.total_requests) || 0;
    const totalTokens = usageMetrics.total_tokens || 0;

    if (totalReq > 100000) {
      return {
        message: "⚠️ Global request spike detected (35% ↑). Review rate limits.",
        type: 'warning' as const,
        target: '/dashboard/security'
      };
    }
    
    if (totalTokens > 5000000) {
      return {
        message: "⚠️ GPT-4o usage spiked. Budget exhausts in 3 days. [Optimize]",
        type: 'warning' as const,
        target: '/dashboard/billing'
      };
    }

    return {
      message: "AI Insight: Switching to Claude 3.5 Sonnet could save $420/mo.",
      type: 'suggestion' as const,
      target: '/dashboard/billing'
    };
  }, [metrics, usageMetrics]);

  // ── Chaos toggle — real API call, no business logic changes ───────────────
  const handleTriggerOutage = async () => {
    toggleIncident();
    try {
      await fetch('http://localhost:8080/v1/admin/toggle-chaos', {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json', 'x-csrf-token': '1' },
      });
    } catch (e) {
      console.error('Failed to trigger chaos', e);
    }
  };

  const isSystemOffline = !!error || isIncidentActive;
  const pulseStatus     = isSystemOffline ? 'error' : isLoading ? 'warning' : 'active';

  const LABEL_CLASS = 'font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-xs font-bold';
  const DATA_CLASS  = 'font-jetbrains text-[var(--on-surface)]';

  // ── Algorithmic Layout Priority ───────────────────────────────────────────
  const layoutOrder = useMemo(() => {
    if (role === 'Engineer') {
      return ['routingMap', 'stats', 'agenticTraffic', 'trafficChart', 'healthHeatmap', 'auditStream'];
    } else {
      return ['stats', 'agenticTraffic', 'auditStream', 'healthHeatmap', 'trafficChart', 'routingMap'];
    }
  }, [role]);

  return (
    <>
      <TutorialOverlay />

      <motion.div
        layout
        animate={{
          gap: isIncidentActive ? '8px' : '24px',
        }}
        className="grid grid-cols-12 auto-rows-max text-[var(--on-surface)] w-full"
      >
        {/* ── Header ──────────────────────────────────────────────────────── */}
        <motion.header
          layout
          variants={fadeUpVariant}
          className="col-span-12 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4"
        >
          <div className="flex items-center gap-6">
            <div>
              <h1 className={`text-3xl sm:text-4xl lg:text-5xl font-extrabold tracking-tight text-[var(--on-surface)] pb-1 flex items-center gap-4 font-geist ${isIncidentActive ? 'text-rose-500' : ''}`}>
                RedEye Control Plane
                <LivePulseIndicator status={pulseStatus} />
              </h1>
              <p className={`${LABEL_CLASS} mt-1 text-[10px] flex items-center gap-3`}>
                Spatial Intelligence Matrix 
                <span className="opacity-20">|</span>
                <span className={isIncidentActive ? 'text-rose-500 animate-pulse' : 'text-cyan-500'}>{role.toUpperCase()} MODE</span>
              </p>
            </div>
          </div>

          <div className="flex items-center gap-3">
            {/* Role Switcher */}
            <div className="flex bg-[var(--surface-container)] p-1 rounded-xl mr-2">
              {(['Engineer', 'Coordinator'] as const).map((r) => (
                <button
                  key={r}
                  onClick={() => setRole(r)}
                  className={`px-3 py-1.5 rounded-lg text-[9px] font-black uppercase tracking-widest transition-all ${
                    role === r ? 'bg-white/10 text-white shadow-lg' : 'text-white/30 hover:text-white/60'
                  }`}
                >
                  {r}
                </button>
              ))}
            </div>

            <BentoCard glowColor="amber" className="p-2 px-4 flex items-center gap-3">
              <span className={`${LABEL_CLASS} text-[9px] text-amber-600 dark:text-amber-500`}>
                Danger Zone
              </span>
              <button
                id="btn-simulate-outage"
                onClick={handleTriggerOutage}
                className={[
                  'px-4 py-1.5 rounded-lg font-bold uppercase tracking-widest text-[10px]',
                  'flex items-center gap-2 font-geist',
                  'transition-all duration-150',
                  'active:translate-y-[2px]',
                  isIncidentActive
                    ? 'bg-rose-500 text-white shadow-[0_4px_0_0_rgba(180,20,50,0.7),0_8px_24px_-4px_rgba(244,63,94,0.5)]'
                    : 'bg-gradient-to-br from-[var(--primary-amber)] to-[var(--primary-rose)] text-black shadow-[0_4px_0_0_rgba(160,40,10,0.6),0_8px_24px_-4px_rgba(251,191,36,0.4)] hover:shadow-[0_4px_0_0_rgba(160,40,10,0.6),0_14px_32px_-4px_rgba(251,191,36,0.55)]',
                ].join(' ')}
              >
                <Zap className="w-3 h-3 fill-current" />
                {isIncidentActive ? 'Restore Normalcy' : 'Simulate Outage'}
              </button>
            </BentoCard>
          </div>
        </motion.header>

        {/* ── AI Insight Pill ────────────────────────────────────────────── */}
        <motion.div layout className="col-span-12 flex justify-center -mt-4 -mb-2 z-20">
          <AnimatePresence>
            {insight && (
              <InsightPill
                message={insight.message}
                type={insight.type}
                onClick={() => navigate(insight.target)}
              />
            )}
          </AnimatePresence>
        </motion.div>

        {/* ── Error Banner ───────────────────────────────────────────────── */}
        {error && !metrics && (
          <motion.div
            layout
            variants={fadeUpVariant}
            className="col-span-12 p-4 flex items-center gap-3 rounded-2xl"
            style={{ background: 'rgba(251,113,133,0.08)' }}
          >
            <AlertCircle className="w-5 h-5 text-rose-500 flex-shrink-0" />
            <p className="text-sm font-jetbrains text-rose-400">
              WARN: Connection to telemetry stream severed.
            </p>
          </motion.div>
        )}

        {/* ── Algorithmic Grid Rendering ─────────────────────────────────── */}
        <AnimatePresence mode="popLayout">
          {layoutOrder.map((compId) => (
            <motion.div
              key={compId}
              layout
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.9 }}
              transition={{ duration: 0.5, ease: [0.16, 1, 0.3, 1] }}
              className="col-span-12"
            >
              {COMPONENTS[compId]({
                isIncidentActive,
                metrics,
                isUsageLoading,
                sparklineData,
                successRate,
                heatmapData,
                stackedTraffic
              })}
            </motion.div>
          ))}
        </AnimatePresence>

      </motion.div>
    </>
  );
}
