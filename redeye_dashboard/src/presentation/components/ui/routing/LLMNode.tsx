import { memo, useState } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { motion, AnimatePresence } from 'framer-motion';
import { AlertTriangle, CheckCircle2, Clock, TrendingUp, Database, Activity } from 'lucide-react';
import type { LLMNodeData, ModelTier, NodeStatus } from './types';

// ─── Tier Badge ───────────────────────────────────────────────────────────────

const tierConfig: Record<ModelTier, { label: string; className: string }> = {
  primary: { label: 'Primary', className: 'bg-cyan-400/15 text-cyan-300 border-cyan-400/30' },
  secondary: { label: 'Fallback', className: 'bg-amber-400/15 text-amber-300 border-amber-400/30' },
  tertiary: { label: 'Reserve', className: 'bg-slate-500/20 text-slate-400 border-slate-500/30' },
};

// ─── Status glow config ───────────────────────────────────────────────────────

const statusGlowMap: Record<NodeStatus, string> = {
  active: '0 0 0 1px rgba(34,211,238,0.20), 0 16px 40px -10px rgba(34,211,238,0.5), 0 0 20px rgba(34,211,238,0.15), inset 0 1px 1px rgba(255,255,255,0.05)',
  degraded: '0 0 0 1px rgba(245,158,11,0.25), 0 16px 40px -10px rgba(245,158,11,0.5), 0 0 20px rgba(245,158,11,0.15), inset 0 1px 1px rgba(255,255,255,0.05)',
  error: '0 0 0 1px rgba(244,63,94,0.35), 0 16px 40px -10px rgba(244,63,94,0.5), 0 0 28px rgba(244,63,94,0.20), inset 0 1px 1px rgba(255,255,255,0.05)',
  offline: '0 0 0 1px rgba(100,116,139,0.15), 0 8px 30px -10px rgba(100,116,139,0.2)',
  standby: '0 0 0 1px rgba(148,163,184,0.10), 0 8px 30px -10px rgba(148,163,184,0.2)',
};

const statusBorderMap: Record<NodeStatus, string> = {
  active: 'rgba(34,211,238,0.25)',
  degraded: 'rgba(245,158,11,0.30)',
  error: 'rgba(244,63,94,0.40)',
  offline: 'rgba(100,116,139,0.20)',
  standby: 'rgba(148,163,184,0.12)',
};

const statusDotMap: Record<NodeStatus, string> = {
  active: 'bg-cyan-400',
  degraded: 'bg-amber-400',
  error: 'bg-rose-500',
  offline: 'bg-slate-500',
  standby: 'bg-slate-600',
};

// ─── Metric Row ───────────────────────────────────────────────────────────────

const MetricRow = ({
  icon: Icon,
  label,
  value,
  unit,
  color,
}: {
  icon: React.ElementType;
  label: string;
  value: string | number;
  unit?: string;
  color?: string;
}) => (
  <div className="flex items-center justify-between gap-3">
    <div className="flex items-center gap-1.5">
      <Icon size={10} className={color ?? 'text-[var(--text-subtle)]'} />
      <span className="text-[var(--text-muted)] text-[9px] uppercase tracking-widest font-geist font-medium">{label}</span>
    </div>
    <span
      className="text-white/85 text-[11px] font-semibold tabular-nums font-jetbrains"
    >
      {value}
      {unit && <span className="text-[var(--text-subtle)] ml-0.5 text-[9px]">{unit}</span>}
    </span>
  </div>
);

// ─── Status Icon ──────────────────────────────────────────────────────────────

const StatusIcon = ({ status }: { status: NodeStatus }) => {
  if (status === 'active')
    return <CheckCircle2 size={11} className="text-cyan-400" />;
  if (status === 'error')
    return <AlertTriangle size={11} className="text-rose-400" />;
  if (status === 'degraded')
    return <AlertTriangle size={11} className="text-amber-400" />;
  return <Clock size={11} className="text-slate-500" />;
};

// ─── Magnitude Formatter ───────────────────────────────────────────────────────
function fmtMag(raw: string | number | undefined | null): string {
  if (raw === undefined || raw === null) return '—';
  const n = typeof raw === 'string' ? parseFloat(raw) : raw;
  if (isNaN(n)) return '—';
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toFixed(n % 1 === 0 ? 0 : 1);
}

// ─── LLM Node ─────────────────────────────────────────────────────────────────

export const LLMNode = memo(({ data }: NodeProps) => {
  const d = data as LLMNodeData;
  const [isHovered, setIsHovered] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);

  const tierCfg = tierConfig[d.tier];
  const isError = d.status === 'error';
  const isDegraded = d.status === 'degraded';
  const isStandby = d.status === 'standby' || d.status === 'offline';

  // Flicker animation for error nodes
  const errorFlickerAnim = isError
    ? {
      opacity: [1, 0.85, 1, 0.9, 1],
      boxShadow: [
        statusGlowMap.error,
        '0 0 0 1px rgba(244,63,94,0.15), 0 4px 15px -5px rgba(244,63,94,0.2), 0 0 12px rgba(244,63,94,0.08)',
        statusGlowMap.error,
      ],
    }
    : {};

  return (
    <motion.div
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={() => setIsExpanded(!isExpanded)}
      animate={
        isError
          ? errorFlickerAnim
          : {
            opacity: isStandby ? 0.55 : 1,
            scale: isExpanded ? 1.05 : 1,
          }
      }
      transition={
        isError
          ? { duration: 2.5, repeat: Infinity, ease: 'easeInOut' }
          : { duration: 0.3, type: 'spring', stiffness: 300, damping: 20 }
      }
      style={{
        width: 220,
        background: isError
          ? 'linear-gradient(135deg, rgba(244,63,94,0.06) 0%, rgba(5,5,5,0.95) 100%)'
          : isDegraded
            ? 'linear-gradient(135deg, rgba(245,158,11,0.05) 0%, rgba(5,5,5,0.95) 100%)'
            : 'linear-gradient(135deg, rgba(255,255,255,0.025) 0%, rgba(5,5,5,0.92) 100%)',
        border: `1px solid ${statusBorderMap[d.status]}`,
        borderRadius: '14px',
        boxShadow: statusGlowMap[d.status],
        backdropFilter: 'blur(20px)',
        WebkitBackdropFilter: 'blur(20px)',
        overflow: 'hidden',
        cursor: 'pointer',
        zIndex: isExpanded ? 50 : 1,
      }}
    >
      {/* ── Top accent line ────────────────────────────────────────────────── */}
      <div
        className="absolute top-0 left-0 right-0 h-[1px]"
        style={{
          background: `linear-gradient(90deg, transparent, ${d.providerColor}55, transparent)`,
        }}
      />

      {/* ── Main content ───────────────────────────────────────────────────── */}
      <div className="p-3.5">
        {/* Header row */}
        <div className="flex items-start justify-between gap-2 mb-2.5">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-1.5 mb-0.5">
              {/* Status dot with pulse for active */}
              <div className="relative flex items-center justify-center">
                {d.status === 'active' && (
                  <motion.div
                    className="absolute w-2.5 h-2.5 rounded-full bg-cyan-400/40"
                    animate={{ scale: [1, 1.8], opacity: [0.4, 0] }}
                    transition={{ duration: 1.8, repeat: Infinity }}
                  />
                )}
                <div className={`w-1.5 h-1.5 rounded-full ${statusDotMap[d.status]}`} />
              </div>

              <p className="text-[var(--on-surface)] font-semibold text-[12px] leading-tight truncate">
                {d.label}
              </p>
            </div>

            {/* Provider */}
            <div className="flex items-center gap-1.5">
              <div
                className="w-1.5 h-1.5 rounded-sm"
                style={{ background: d.providerColor }}
              />
              <span className="text-white/35 text-[9px] font-medium tracking-widest font-geist uppercase">
                {d.provider}
              </span>
            </div>
          </div>

          {/* Right: tier badge + status icon */}
          <div className="flex flex-col items-end gap-1 flex-shrink-0">
            <span
              className={`inline-flex items-center gap-0.5 border text-[8px] font-semibold uppercase tracking-widest px-1.5 py-0.5 rounded-full font-geist ${tierCfg.className}`}
            >
              {tierCfg.label}
            </span>
            <StatusIcon status={d.status} />
          </div>
        </div>

        {/* ── Collapsed metrics (always visible) ─────────────────────────── */}
        <div className="flex items-center gap-2">
          <div
            className="flex-1 bg-white/[0.03] rounded-lg px-2 py-1.5 flex flex-col items-center border border-white/[0.05]"
          >
            <span
              className="text-white/85 text-sm font-bold tabular-nums font-jetbrains leading-none"
            >
              {isError ? '—' : fmtMag(d.metrics.rpm)}
            </span>
            <span className="text-[var(--text-subtle)] text-[8px] uppercase tracking-widest font-geist mt-0.5">RPM</span>
          </div>
          <div
            className="flex-1 bg-white/[0.03] rounded-lg px-2 py-1.5 flex flex-col items-center border border-white/[0.05]"
          >
            <span
              className="text-white/85 text-sm font-bold tabular-nums font-jetbrains leading-none"
            >
              {isError ? '—' : `${Math.round(d.metrics.avgLatencyMs)}`}
            </span>
            <span className="text-[var(--text-subtle)] text-[8px] uppercase tracking-widest font-geist mt-0.5">ms</span>
          </div>
          <div
            className="flex-1 bg-white/[0.03] rounded-lg px-2 py-1.5 flex flex-col items-center border border-white/[0.05]"
          >
            <span
              className={`text-sm font-bold tabular-nums font-jetbrains leading-none ${isError ? 'text-rose-400/80' : 'text-white/85'
                }`}
            >
              {isError ? `${d.metrics.errorRate}%` : `${d.metrics.cacheHitPct}%`}
            </span>
            <span className="text-[var(--text-subtle)] text-[8px] uppercase tracking-widest font-geist mt-0.5">
              {isError ? 'ERR' : 'CACHE'}
            </span>
          </div>
        </div>

        {/* ── Expanded hover/click panel ─────────────────────────────────────────── */}
        <AnimatePresence>
          {(isHovered || isExpanded) && (
            <motion.div
              initial={{ opacity: 0, height: 0, marginTop: 0 }}
              animate={{ opacity: 1, height: 'auto', marginTop: 10 }}
              exit={{ opacity: 0, height: 0, marginTop: 0 }}
              transition={{ duration: 0.22, ease: [0.4, 0, 0.2, 1] }}
              className="overflow-hidden"
            >
              <div
                className="rounded-lg p-2.5 flex flex-col gap-1.5"
                style={{
                  background: 'rgba(255,255,255,0.02)',
                  border: '1px solid rgba(255,255,255,0.06)',
                }}
              >
                <MetricRow
                  icon={TrendingUp}
                  label="Uptime"
                  value={`${d.metrics.uptime.toFixed(1)}`}
                  unit="%"
                  color="text-emerald-400/60"
                />
                <MetricRow
                  icon={Activity}
                  label="TPS"
                  value={isError ? '0' : fmtMag(d.metrics.tps)}
                  color="text-cyan-400/60"
                />
                <MetricRow
                  icon={Database}
                  label="Cache Hit"
                  value={isError ? '—' : `${d.metrics.cacheHitPct}`}
                  unit="%"
                  color="text-violet-400/60"
                />
                <MetricRow
                  icon={AlertTriangle}
                  label="Error Rate"
                  value={`${d.metrics.errorRate.toFixed(1)}`}
                  unit="%"
                  color={d.metrics.errorRate > 1 ? 'text-rose-400/70' : 'text-[var(--text-subtle)]'}
                />
                {/* Model ID chip */}
                <div className="mt-1 pt-1.5 border-t border-white/[0.05]">
                  <span
                    className="text-[8px] text-white/25 font-jetbrains font-medium uppercase tracking-widest"
                  >
                    ID: {d.model}
                  </span>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* ── React Flow handle ─────────────────────────────────────────────── */}
      <Handle
        type="target"
        position={Position.Left}
        style={{
          background: isError ? 'rgba(244,63,94,0.6)' : 'rgba(34,211,238,0.5)',
          border: `1.5px solid ${isError ? 'rgba(244,63,94,0.9)' : 'rgba(34,211,238,0.8)'}`,
          width: 8,
          height: 8,
          left: 2,
        }}
      />
    </motion.div>
  );
});

LLMNode.displayName = 'LLMNode';
