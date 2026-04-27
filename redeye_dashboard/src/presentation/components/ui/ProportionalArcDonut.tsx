import React, { useEffect, useState } from 'react';
import { motion, useSpring, useTransform, animate } from 'framer-motion';

// ── Types ─────────────────────────────────────────────────────────────────────

interface ProportionalArcDonutProps {
  value: number;      // 0–100
  size?: number;
  strokeWidth?: number;
  className?: string;
}

// ── Colour resolver ───────────────────────────────────────────────────────────

interface ArcColour {
  stroke: string;           // CSS var reference for SVG stroke
  shadowRgb: string;        // raw RGB for feDropShadow
}

function resolveColour(value: number): ArcColour {
  if (value < 50) {
    return {
      stroke:    'var(--primary-rose)',
      shadowRgb: '251, 113, 133',
    };
  }
  if (value < 90) {
    return {
      stroke:    'var(--primary-amber)',
      shadowRgb: '251, 191, 36',
    };
  }
  return {
    stroke:    'var(--accent-cyan)',
    shadowRgb: '34, 211, 238',
  };
}

// ── Component ─────────────────────────────────────────────────────────────────

export function ProportionalArcDonut({
  value,
  size = 160,
  strokeWidth = 10,
  className = '',
}: ProportionalArcDonutProps) {
  const clamped = Math.min(Math.max(value, 0), 100);
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const dashOffset = circumference - (clamped / 100) * circumference;

  const { stroke, shadowRgb } = resolveColour(clamped);
  const uid = `glow-${size}-${Math.floor(value)}`;
  const maskId = `segment-mask-${size}`;

  // ── Animated Number Logic ──────────────────────────────────────────────────
  const springValue = useSpring(0, { stiffness: 40, damping: 20 });
  const displayValue = useTransform(springValue, (v) => v.toFixed(1));

  useEffect(() => {
    springValue.set(clamped);
  }, [clamped, springValue]);

  return (
    <div
      className={`relative flex items-center justify-center ${className}`}
      style={{ width: size, height: size }}
    >
      <svg
        width={size}
        height={size}
        className="transform -rotate-90"
        aria-hidden="true"
        viewBox={`0 0 ${size} ${size}`}
      >
        <defs>
          {/* Holographic Glow Filter */}
          <filter id={uid} x="-50%" y="-50%" width="200%" height="200%">
            <feDropShadow
              dx="0"
              dy="0"
              stdDeviation="4"
              floodColor={`rgb(${shadowRgb})`}
              floodOpacity="0.8"
            />
            <feDropShadow
              dx="0"
              dy="0"
              stdDeviation="1.5"
              floodColor="#ffffff"
              floodOpacity="0.4"
            />
          </filter>

          {/* Segmented Mask (The tactical blocks) */}
          <mask id={maskId}>
            <circle
              cx={size / 2}
              cy={size / 2}
              r={radius}
              stroke="white"
              strokeWidth={strokeWidth}
              fill="none"
              strokeDasharray="4 6"
              strokeLinecap="butt"
            />
          </mask>
        </defs>

        {/* Segmented Background Track */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          strokeWidth={strokeWidth}
          fill="none"
          stroke="rgba(255, 255, 255, 0.05)"
          strokeDasharray="4 6"
          strokeLinecap="butt"
        />

        {/* Active Segmented Arc with Holographic Glow */}
        <motion.circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          strokeWidth={strokeWidth}
          fill="none"
          stroke={stroke}
          strokeLinecap="butt"
          style={{ 
            filter: `url(#${uid})`,
          }}
          strokeDasharray={circumference}
          initial={{ strokeDashoffset: circumference }}
          animate={{ strokeDashoffset: dashOffset }}
          transition={{ duration: 1.5, ease: [0.16, 1, 0.3, 1] }}
          mask={`url(#${maskId})`}
        />
      </svg>

      {/* ── HUD Center Label ── */}
      <div className="absolute flex flex-col items-center justify-center select-none">
        <div className="flex items-baseline gap-0.5">
          <motion.span
            className="font-jetbrains text-3xl font-extrabold tabular-nums tracking-tighter"
            style={{ color: 'var(--on-surface)' }}
          >
            {displayValue}
          </motion.span>
          <span
            className="text-xs font-jetbrains font-bold opacity-30 uppercase tracking-widest"
            style={{ color: 'var(--on-surface)' }}
          >
            %
          </span>
        </div>
        <div className="h-px w-8 bg-white/10 mt-1" />
        <span className="text-[8px] font-geist uppercase tracking-[0.2em] mt-1 opacity-20 font-bold">
          Telemetry
        </span>
      </div>
    </div>
  );
}

