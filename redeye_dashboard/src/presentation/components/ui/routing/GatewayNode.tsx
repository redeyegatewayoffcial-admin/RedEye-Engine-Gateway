import { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { motion } from 'framer-motion';
import { Activity, GitBranch, Zap, Clock } from 'lucide-react';
import type { GatewayNodeData } from './types';

// ─── Pulse Ring ───────────────────────────────────────────────────────────────

const PulseRing = ({ delay = 0 }: { delay?: number }) => (
  <motion.div
    className="absolute inset-0 rounded-full border border-cyan-400/40"
    animate={{ scale: [1, 1.6], opacity: [0.6, 0] }}
    transition={{ duration: 2.4, repeat: Infinity, ease: 'easeOut', delay }}
  />
);

// ─── Stat Pill ────────────────────────────────────────────────────────────────

const StatPill = ({
  icon: Icon,
  label,
  value,
  accent,
}: {
  icon: React.ElementType;
  label: string;
  value: string;
  accent?: string;
}) => (
  <div className="flex items-center gap-1.5 bg-white/[0.04] border border-white/[0.07] rounded-lg px-2.5 py-1.5">
    <Icon size={11} className={accent ?? 'text-[var(--text-muted)]'} />
    <span className="text-[var(--text-muted)] text-[9px] font-medium uppercase tracking-widest">{label}</span>
    <span
      className="text-[var(--on-surface)] text-[11px] font-semibold tabular-nums leading-none"
      style={{ fontFamily: "'JetBrains Mono', ui-monospace, monospace" }}
    >
      {value}
    </span>
  </div>
);

// ─── Gateway Node ─────────────────────────────────────────────────────────────

export const GatewayNode = memo(({ data }: NodeProps) => {
  const d = data as GatewayNodeData;
  const hasFallback = d.fallbacksActive > 0;

  return (
    <div className="relative flex items-center justify-center" style={{ width: 220, height: 220 }}>
      {/* ── Ambient pulse rings ─────────────────────────────────────────────── */}
      <PulseRing delay={0} />
      <PulseRing delay={0.8} />
      <PulseRing delay={1.6} />

      {/* ── Outer glow disc ─────────────────────────────────────────────────── */}
      <div
        className="absolute inset-0 rounded-full"
        style={{
          background:
            'radial-gradient(circle, rgba(34,211,238,0.08) 0%, rgba(34,211,238,0.03) 50%, transparent 70%)',
        }}
      />

      {/* ── Main disc ───────────────────────────────────────────────────────── */}
      <motion.div
        className="relative flex flex-col items-center justify-center rounded-full"
        style={{
          width: 180,
          height: 180,
          background: 'radial-gradient(circle at 35% 30%, rgba(34,211,238,0.10) 0%, #050505 60%)',
          border: '1.5px solid rgba(34,211,238,0.35)',
          boxShadow: hasFallback
            ? '0 0 0 1px rgba(34,211,238,0.15), 0 16px 40px -10px rgba(34,211,238,0.5), 0 0 60px rgba(34,211,238,0.10), 0 0 16px rgba(245,158,11,0.18) inset'
            : '0 0 0 1px rgba(34,211,238,0.15), 0 16px 40px -10px rgba(34,211,238,0.5), 0 0 60px rgba(34,211,238,0.10)',
          backdropFilter: 'blur(40px)',
          WebkitBackdropFilter: 'blur(40px)',
        }}
        animate={
          hasFallback
            ? {
                boxShadow: [
                  '0 0 0 1px rgba(34,211,238,0.15), 0 16px 40px -10px rgba(34,211,238,0.5), 0 0 60px rgba(34,211,238,0.10), 0 0 16px rgba(245,158,11,0.12) inset',
                  '0 0 0 1px rgba(34,211,238,0.25), 0 24px 60px -8px rgba(34,211,238,0.6), 0 0 80px rgba(34,211,238,0.15), 0 0 24px rgba(245,158,11,0.22) inset',
                  '0 0 0 1px rgba(34,211,238,0.15), 0 16px 40px -10px rgba(34,211,238,0.5), 0 0 60px rgba(34,211,238,0.10), 0 0 16px rgba(245,158,11,0.12) inset',
                ],
              }
            : {
                boxShadow: [
                  '0 0 0 1px rgba(34,211,238,0.15), 0 16px 40px -10px rgba(34,211,238,0.4), 0 0 50px rgba(34,211,238,0.08)',
                  '0 0 0 1px rgba(34,211,238,0.25), 0 24px 60px -8px rgba(34,211,238,0.5), 0 0 72px rgba(34,211,238,0.14)',
                  '0 0 0 1px rgba(34,211,238,0.15), 0 16px 40px -10px rgba(34,211,238,0.4), 0 0 50px rgba(34,211,238,0.08)',
                ],
              }
        }
        transition={{ duration: 3, repeat: Infinity, ease: 'easeInOut' }}
      >
        {/* ── Inner accent ring ─────────────────────────────────────────────── */}
        <div
          className="absolute inset-4 rounded-full"
          style={{
            border: '1px solid rgba(34,211,238,0.12)',
            background: 'transparent',
          }}
        />

        {/* ── RedEye mark ───────────────────────────────────────────────────── */}
        <div className="relative z-10 flex flex-col items-center gap-1">
          <div className="flex items-center justify-center w-8 h-8 rounded-full bg-cyan-400/10 border border-cyan-400/30 mb-1">
            <Zap size={16} className="text-cyan-400" />
          </div>
          <span
            className="text-[var(--on-surface)] font-bold tracking-[-0.02em] text-sm leading-none"
          >
            RedEye
          </span>
          <span
            className="text-cyan-400/80 font-semibold text-[10px] tracking-[0.2em] uppercase leading-none"
          >
            Gateway
          </span>

          {/* ── Uptime indicator ────────────────────────────────────────────── */}
          <div className="flex items-center gap-1 mt-2">
            <motion.div
              className="w-1.5 h-1.5 rounded-full bg-emerald-400"
              animate={{ opacity: [1, 0.3, 1] }}
              transition={{ duration: 1.8, repeat: Infinity }}
            />
            <span
              className="text-emerald-400/80 text-[9px] font-semibold tabular-nums"
              style={{ fontFamily: "'JetBrains Mono', ui-monospace, monospace" }}
            >
              {d.uptime.toFixed(2)}% UP
            </span>
          </div>
        </div>
      </motion.div>

      {/* ── Stats strip — sits below the disc ─────────────────────────────── */}
      <div
        className="absolute flex gap-1.5 flex-wrap justify-center"
        style={{ bottom: -8, left: '50%', transform: 'translateX(-50%)', width: 260 }}
      >
        <StatPill icon={Activity} label="RPM" value={d.totalRpm.toLocaleString()} accent="text-cyan-400" />
        <StatPill icon={GitBranch} label="Routes" value={String(d.activeRoutes)} />
        {hasFallback && (
          <StatPill icon={Clock} label="Fallbacks" value={String(d.fallbacksActive)} accent="text-amber-400" />
        )}
      </div>

      {/* ── React Flow handle ─────────────────────────────────────────────── */}
      <Handle
        type="source"
        position={Position.Right}
        style={{
          background: 'rgba(34,211,238,0.6)',
          border: '1.5px solid rgba(34,211,238,0.9)',
          width: 8,
          height: 8,
          right: 2,
        }}
      />
    </div>
  );
});

GatewayNode.displayName = 'GatewayNode';
