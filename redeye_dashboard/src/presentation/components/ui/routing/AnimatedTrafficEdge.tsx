import { memo, useId } from 'react';
import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  type EdgeProps,
} from '@xyflow/react';
import type { AnimatedEdgeData } from './types';

// ─── CSS keyframe injection (runs once) ───────────────────────────────────────

const STYLE_ID = 'animated-traffic-edge-styles';

if (typeof document !== 'undefined' && !document.getElementById(STYLE_ID)) {
  const style = document.createElement('style');
  style.id = STYLE_ID;
  style.textContent = `
    @keyframes dash-flow {
      to { stroke-dashoffset: -40; }
    }
    @keyframes dash-flow-fast {
      to { stroke-dashoffset: -24; }
    }
    @keyframes edge-broken-pulse {
      0%, 100% { opacity: 1; }
      40%       { opacity: 0.3; }
      60%       { opacity: 0.7; }
    }
    @keyframes edge-label-fade-in {
      from { opacity: 0; transform: translateY(4px); }
      to   { opacity: 1; transform: translateY(0); }
    }
  `;
  document.head.appendChild(style);
}

// ─── Edge config by status ────────────────────────────────────────────────────

const edgeConfig = {
  flowing: {
    stroke: 'rgba(34,211,238,0.55)',
    glowStroke: 'rgba(34,211,238,0.18)',
    dashStroke: '#22d3ee',
    dashArray: '6 10',
    dashAnimation: 'dash-flow 1.4s linear infinite',
    strokeWidth: 1.5,
    glowWidth: 6,
    opacity: 1,
    bodyAnimation: '',
  },
  fallback: {
    stroke: 'rgba(251,191,36,0.70)',
    glowStroke: 'rgba(251,191,36,0.30)',
    dashStroke: '#fbbf24',
    dashArray: '5 6',
    dashAnimation: 'dash-flow-fast 0.7s linear infinite',
    strokeWidth: 2,
    glowWidth: 8,
    opacity: 1,
    bodyAnimation: 'edge-broken-pulse 1.2s ease-in-out infinite',
  },
  broken: {
    stroke: 'rgba(244,63,94,0.65)',
    glowStroke: 'rgba(244,63,94,0.20)',
    dashStroke: '#f43f5e',
    dashArray: '4 8',
    dashAnimation: '',
    strokeWidth: 1.5,
    glowWidth: 6,
    opacity: 0.9,
    bodyAnimation: 'edge-broken-pulse 1.8s ease-in-out infinite',
  },
  powered_down: {
    stroke: 'rgba(255,255,255,0.05)',
    glowStroke: 'transparent',
    dashStroke: 'transparent',
    dashArray: 'none',
    dashAnimation: '',
    strokeWidth: 1,
    glowWidth: 0,
    opacity: 0.1,
    bodyAnimation: '',
  },
  idle: {
    stroke: 'rgba(148,163,184,0.15)',
    glowStroke: 'transparent',
    dashStroke: 'transparent',
    dashArray: '4 10',
    dashAnimation: '',
    strokeWidth: 1,
    glowWidth: 0,
    opacity: 0.3,
    bodyAnimation: '',
  },
};

// ─── AnimatedTrafficEdge ─────────────────────────────────────────────────────

export const AnimatedTrafficEdge = memo(
  ({
    id,
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    data,
  }: EdgeProps) => {
    const uid = useId();
    const edgeData = (data ?? { status: 'idle', tps: 0, isActive: false }) as AnimatedEdgeData;
    const cfg = edgeConfig[edgeData.status] ?? edgeConfig.idle;

    // Modulate animation speed based on TPS: higher TPS = faster flow
    // Range: ~2.0s (slow) to ~0.4s (very fast)
    const tps = edgeData.tps;
    const duration = tps > 0 ? Math.max(0.4, 2.0 - (tps / 5000)) : 1.4;
    const dynamicAnimation = cfg.dashAnimation
      ? `${cfg.dashAnimation.split(' ')[0]} ${duration.toFixed(2)}s linear infinite`
      : 'none';

    const [edgePath, labelX, labelY] = getBezierPath({
      sourceX,
      sourceY,
      sourcePosition,
      targetX,
      targetY,
      targetPosition,
    });

    const filterId = `glow-${uid}-${id}`;

    return (
      <>
        <defs>
          <filter id={filterId} x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur stdDeviation="3" result="blur" />
            <feMerge>
              <feMergeNode in="blur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>

        {/* ── Glow halo (wide, soft) ─────────────────────────────────────── */}
        {edgeData.status !== 'idle' && (
          <BaseEdge
            id={`${id}-glow`}
            path={edgePath}
            style={{
              stroke: cfg.glowStroke,
              strokeWidth: cfg.glowWidth,
              fill: 'none',
              animation: cfg.bodyAnimation,
              opacity: cfg.opacity,
              filter: `url(#${filterId})`,
            }}
          />
        )}

        {/* ── Crisp base line ───────────────────────────────────────────── */}
        <BaseEdge
          id={`${id}-base`}
          path={edgePath}
          style={{
            stroke: cfg.stroke,
            strokeWidth: cfg.strokeWidth,
            fill: 'none',
            animation: cfg.bodyAnimation,
            opacity: cfg.opacity,
          }}
        />

        {/* ── Animated data-packet dashes ───────────────────────────────── */}
        {edgeData.isActive && edgeData.status !== 'idle' && edgeData.status !== 'broken' && (
          <path
            d={edgePath}
            fill="none"
            stroke={cfg.dashStroke}
            strokeWidth={cfg.strokeWidth + 0.5}
            strokeDasharray={cfg.dashArray}
            strokeLinecap="round"
            style={{
              animation: dynamicAnimation,
              opacity: 0.9,
              filter: `url(#${filterId})`,
            }}
          />
        )}

        {/* ── TPS edge label ────────────────────────────────────────────── */}
        {edgeData.tps > 0 && (
          <EdgeLabelRenderer>
            <div
              style={{
                position: 'absolute',
                transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
                pointerEvents: 'none',
                animation: 'edge-label-fade-in 0.4s ease forwards',
              }}
              className="nodrag nopan"
            >
              <div
                style={{
                  background: 'rgba(5,5,5,0.88)',
                  border: `1px solid ${edgeData.status === 'broken'
                      ? 'rgba(244,63,94,0.40)'
                      : edgeData.status === 'fallback'
                        ? 'rgba(251,191,36,0.40)'
                        : 'rgba(34,211,238,0.25)'
                    }`,
                  borderRadius: '6px',
                  padding: '2px 7px',
                  backdropFilter: 'blur(12px)',
                  boxShadow: '0 2px 12px rgba(0,0,0,0.5)',
                }}
              >
                <span
                  style={{
                    fontFamily: "'JetBrains Mono', ui-monospace, monospace",
                    fontSize: '9px',
                    fontWeight: 600,
                    letterSpacing: '0.05em',
                    color:
                      edgeData.status === 'broken'
                        ? 'rgba(244,63,94,0.90)'
                        : edgeData.status === 'fallback'
                          ? 'rgba(251,191,36,0.90)'
                          : 'rgba(34,211,238,0.85)',
                  }}
                >
                  {edgeData.tps.toLocaleString()} TPS
                </span>
              </div>
            </div>
          </EdgeLabelRenderer>
        )}
      </>
    );
  }
);

AnimatedTrafficEdge.displayName = 'AnimatedTrafficEdge';
