// Dashboard View — ComplianceView
// DPDP Compliance Command Center: Real-time PII redaction telemetry,
// geo-block enforcement stats, and regional detection heatmap.
// Theme: "Cool Revival" (Midnight Obsidian + Neon Cyan/Teal/Amber)

import { ShieldCheck, Fingerprint, MapPin, Eye, Globe, AlertTriangle, TrendingUp } from 'lucide-react';
import useSWR from 'swr';
import { motion } from 'framer-motion';
import { StatCard } from '../components/ui/StatCard';
import { Badge } from '../components/ui/Badge';
import {
  COMPLIANCE_METRICS_URL,
  fetchComplianceMetrics,
  type ComplianceMetrics,
} from '../../data/services/metricsService';

// ── Framer Motion Variants ──────────────────────────────────────────────────

const containerVariants = {
  hidden: {},
  show: { transition: { staggerChildren: 0.08 } },
} as const;

const fadeUpVariant = {
  hidden: { opacity: 0, y: 20 },
  show: {
    opacity: 1,
    y: 0,
    transition: { duration: 0.45, ease: [0.25, 0.1, 0.25, 1] as [number, number, number, number] },
  },
};

// ── Region flag emojis ──────────────────────────────────────────────────────

const REGION_FLAGS: Record<string, string> = {
  IN: '🇮🇳',
  US: '🇺🇸',
  EU: '🇪🇺',
  GLOBAL: '🌐',
};

const REGION_LABELS: Record<string, string> = {
  IN: 'India (DPDP)',
  US: 'United States',
  EU: 'European Union',
  GLOBAL: 'Global',
};

// ── Helper ──────────────────────────────────────────────────────────────────

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

function getPercentage(value: number, total: number): number {
  if (total === 0) return 0;
  return Math.round((value / total) * 100);
}

// ── Component ───────────────────────────────────────────────────────────────

export function ComplianceView() {
  const { data: metrics, error, isLoading } = useSWR<ComplianceMetrics>(
    COMPLIANCE_METRICS_URL,
    fetchComplianceMetrics,
    { refreshInterval: 10_000, errorRetryCount: 3 }
  );

  // ── Loading Skeleton ────────────────────────────────────────────────────

  if (isLoading && !metrics) {
    return (
      <div className="space-y-6 animate-pulse">
        <header>
          <div className="w-32 h-3 bg-slate-800/60 rounded mb-3" />
          <div className="w-72 h-8 bg-slate-800/60 rounded mb-2" />
          <div className="w-96 h-4 bg-slate-800/40 rounded" />
        </header>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          {[...Array(3)].map((_, i) => (
            <div key={i} className="h-28 glass-panel bg-slate-900/40" />
          ))}
        </div>
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          <div className="h-64 glass-panel bg-slate-900/40 lg:col-span-2" />
          <div className="h-64 glass-panel bg-slate-900/40" />
        </div>
      </div>
    );
  }

  const totalScanned = metrics?.total_scanned ?? 0;
  const dpdpBlocks = metrics?.dpdp_blocks ?? 0;
  const piiRedactions = metrics?.pii_redactions ?? 0;
  const regionBreakdown = metrics?.region_breakdown ?? [];

  // Compute the protection rate (scanned - blocks - redactions ≈ clean pass-throughs)
  const cleanThrough = Math.max(0, totalScanned - dpdpBlocks - piiRedactions);
  const protectionRate = totalScanned > 0 ? ((cleanThrough / totalScanned) * 100).toFixed(1) : '100.0';

  // Max count for the region bar widths
  const maxRegionCount = Math.max(1, ...regionBreakdown.map((r) => r.count));

  return (
    <motion.div
      variants={containerVariants}
      initial="hidden"
      animate="show"
      className="space-y-6"
    >
      {/* ── Header ────────────────────────────────────────────────────── */}
      <motion.header
        variants={fadeUpVariant}
        className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4"
      >
        <div>
          <p className="text-xs uppercase tracking-[0.25em] text-slate-500 mb-1 font-medium">
            Compliance Engine
          </p>
          <h1 className="text-2xl sm:text-3xl lg:text-4xl font-extrabold tracking-tight bg-gradient-to-r from-amber-500 to-orange-500 dark:from-amber-400 dark:to-orange-400 bg-clip-text text-transparent pb-1">
            DPDP Security Center
          </h1>
          <p className="text-xs sm:text-sm text-slate-500 dark:text-slate-400 mt-1">
            Real-time PII protection, geo-routing enforcement, and data residency compliance.
          </p>
        </div>

        {/* Live Badge */}
        <div className="flex items-center gap-3">
          {error && (
            <div className="flex items-center gap-1.5 text-amber-400 text-xs font-medium bg-amber-500/10 px-3 py-1.5 rounded-full border border-amber-500/20">
              <AlertTriangle className="w-3.5 h-3.5" />
              <span>Mock Data</span>
            </div>
          )}
          <div className="flex items-center space-x-2 glass-panel px-3 py-1.5 sm:px-4 sm:py-2 rounded-full w-fit">
            <div className={`w-2 h-2 sm:w-3 sm:h-3 rounded-full ${error ? 'bg-amber-500' : 'bg-emerald-500 neon-dot'}`} />
            <span className="text-xs sm:text-sm font-medium text-slate-300">
              {error ? 'Offline — Fallback Active' : 'DPDP Shield Active'}
            </span>
          </div>
        </div>
      </motion.header>

      {/* ── Stat Cards ────────────────────────────────────────────────── */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 sm:gap-4">
        <StatCard
          index={0}
          title="Total Prompts Inspected"
          value={formatNumber(totalScanned)}
          icon={Eye}
          accentClass="text-cyan-400 ring-1 ring-cyan-400/20"
          subtitle={`${protectionRate}% clean pass-through`}
        />
        <StatCard
          index={1}
          title="DPDP Geo-Blocks (India)"
          value={formatNumber(dpdpBlocks)}
          icon={MapPin}
          accentClass="text-rose-400 ring-1 ring-rose-400/20"
          subtitle={`${getPercentage(dpdpBlocks, totalScanned)}% of total traffic`}
        />
        <StatCard
          index={2}
          title="Aadhaar / PAN Redactions"
          value={formatNumber(piiRedactions)}
          icon={Fingerprint}
          accentClass="text-amber-400 ring-1 ring-amber-400/20"
          subtitle={`${getPercentage(piiRedactions, totalScanned)}% detection rate`}
        />
      </div>

      {/* ── Main Content ──────────────────────────────────────────────── */}
      <motion.div
        variants={fadeUpVariant}
        className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-6"
      >
        {/* Region Breakdown — Bar Chart */}
        <div className="glass-panel p-5 sm:p-6 lg:col-span-2">
          <div className="flex items-center justify-between mb-5">
            <div>
              <h2 className="text-lg sm:text-xl font-bold text-slate-100 flex items-center gap-2">
                <Globe className="w-5 h-5 text-cyan-400" />
                Regional Detection Heatmap
              </h2>
              <p className="text-xs text-slate-500 mt-1">
                Distribution of compliance events by detected region.
              </p>
            </div>
            <Badge variant="success">Live</Badge>
          </div>

          <div className="space-y-4">
            {regionBreakdown.length === 0 ? (
              <div className="text-center py-10">
                <Globe className="w-8 h-8 text-slate-700 mx-auto mb-3" />
                <p className="text-sm text-slate-500">No regional data yet</p>
              </div>
            ) : (
              regionBreakdown.map((region, idx) => {
                const pct = getPercentage(region.count, totalScanned);
                const barWidth = Math.max(4, (region.count / maxRegionCount) * 100);
                const isIndia = region.region === 'IN';

                return (
                  <motion.div
                    key={region.region}
                    initial={{ opacity: 0, x: -20 }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={{ delay: idx * 0.1, duration: 0.35 }}
                    className={`group relative rounded-xl p-4 border transition-all duration-300 hover:-translate-y-0.5 ${
                      isIndia
                        ? 'bg-amber-500/5 border-amber-500/20 hover:border-amber-500/40 hover:shadow-[0_0_20px_rgba(245,158,11,0.08)]'
                        : 'bg-slate-900/30 border-slate-800/60 hover:border-cyan-500/25 hover:shadow-[0_0_20px_rgba(34,211,238,0.06)]'
                    }`}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-3">
                        <span className="text-xl">{REGION_FLAGS[region.region] ?? '🌐'}</span>
                        <div>
                          <p className="text-sm font-semibold text-slate-200">
                            {REGION_LABELS[region.region] ?? region.region}
                          </p>
                          <p className="text-[10px] text-slate-500 font-mono uppercase tracking-wider">
                            {region.region}
                            {isIndia && (
                              <span className="ml-2 text-amber-400">● DPDP ENFORCED</span>
                            )}
                          </p>
                        </div>
                      </div>
                      <div className="text-right">
                        <p className="text-lg font-bold text-slate-100 tabular-nums">
                          {region.count.toLocaleString()}
                        </p>
                        <p className="text-[10px] text-slate-500">{pct}% of total</p>
                      </div>
                    </div>
                    {/* Progress bar */}
                    <div className="h-1.5 bg-slate-800/60 rounded-full overflow-hidden">
                      <motion.div
                        initial={{ width: 0 }}
                        animate={{ width: `${barWidth}%` }}
                        transition={{ delay: 0.3 + idx * 0.1, duration: 0.6, ease: 'easeOut' }}
                        className={`h-full rounded-full ${
                          isIndia
                            ? 'bg-gradient-to-r from-amber-500 to-orange-500'
                            : 'bg-gradient-to-r from-cyan-500 to-teal-500'
                        }`}
                      />
                    </div>
                  </motion.div>
                );
              })
            )}
          </div>
        </div>

        {/* Right Column — DPDP Policy Status + Protection Summary */}
        <div className="space-y-4 sm:space-y-6">
          {/* DPDP Policy Card */}
          <motion.div variants={fadeUpVariant} className="glass-panel p-5">
            <h2 className="text-sm font-bold text-slate-100 flex items-center gap-2 mb-4">
              <ShieldCheck className="w-4 h-4 text-amber-400" />
              DPDP Policy Status
            </h2>
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-xs text-slate-400">Framework</span>
                <Badge variant="success">DPDP 2023</Badge>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs text-slate-400">Region Lock</span>
                <Badge variant="success">IN Enforced</Badge>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs text-slate-400">Fail Mode</span>
                <Badge variant="danger">Fail-Closed</Badge>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs text-slate-400">PII Engine</span>
                <Badge variant="success">Two-Tier Active</Badge>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs text-slate-400">US-Only Block</span>
                <Badge variant="success">o1-preview, o1-mini</Badge>
              </div>
            </div>
          </motion.div>

          {/* Detection Summary */}
          <motion.div variants={fadeUpVariant} className="glass-panel p-5">
            <h2 className="text-sm font-bold text-slate-100 flex items-center gap-2 mb-4">
              <TrendingUp className="w-4 h-4 text-cyan-400" />
              Protection Summary
            </h2>
            <div className="space-y-3">
              {[
                { label: 'Aadhaar Numbers', color: 'text-amber-400 bg-amber-500/10' },
                { label: 'PAN Cards', color: 'text-amber-400 bg-amber-500/10' },
                { label: 'SSN (US)', color: 'text-sky-400 bg-sky-500/10' },
                { label: 'Credit Cards', color: 'text-rose-400 bg-rose-500/10' },
                { label: 'Email Addresses', color: 'text-violet-400 bg-violet-500/10' },
              ].map((entity) => (
                <div key={entity.label} className="flex items-center gap-3">
                  <div className={`w-2 h-2 rounded-full ${entity.color.split(' ')[0]}`} />
                  <span className="text-xs text-slate-400 flex-1">{entity.label}</span>
                  <span
                    className={`text-[10px] font-semibold px-2 py-0.5 rounded-full ${entity.color}`}
                  >
                    Protected
                  </span>
                </div>
              ))}
            </div>

            {/* Protection rate indicator */}
            <div className="mt-5 pt-4 border-t border-slate-800/60">
              <div className="flex items-center justify-between mb-2">
                <span className="text-xs text-slate-500">Overall Protection Rate</span>
                <span className="text-sm font-bold text-emerald-400">{protectionRate}%</span>
              </div>
              <div className="h-2 bg-slate-800/60 rounded-full overflow-hidden">
                <motion.div
                  initial={{ width: 0 }}
                  animate={{ width: `${protectionRate}%` }}
                  transition={{ delay: 0.5, duration: 0.8, ease: 'easeOut' }}
                  className="h-full bg-gradient-to-r from-emerald-500 to-teal-400 rounded-full"
                />
              </div>
            </div>
          </motion.div>
        </div>
      </motion.div>

      {/* ── Active Protections Strip ──────────────────────────────────── */}
      <motion.div
        variants={fadeUpVariant}
        className="glass-panel p-4 sm:p-5"
      >
        <div className="flex flex-wrap items-center gap-3">
          <span className="text-xs font-semibold text-slate-500 uppercase tracking-wider mr-2">
            Active Shields:
          </span>
          {[
            { label: 'Aho-Corasick PII Scan', variant: 'success' as const },
            { label: 'Presidio Deep Analysis', variant: 'success' as const },
            { label: 'DPDP Region Lock', variant: 'success' as const },
            { label: 'US-Only Model Block', variant: 'danger' as const },
            { label: 'Fail-Closed Enforcement', variant: 'danger' as const },
          ].map((shield) => (
            <Badge key={shield.label} variant={shield.variant}>
              {shield.label}
            </Badge>
          ))}
        </div>
      </motion.div>
    </motion.div>
  );
}
