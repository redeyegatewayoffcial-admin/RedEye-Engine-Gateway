import React, { useMemo } from 'react';
import { 
  ScatterChart, 
  Scatter, 
  XAxis, 
  YAxis, 
  ZAxis,
  CartesianGrid, 
  Tooltip, 
  ResponsiveContainer,
  Cell
} from 'recharts';

// ── Types ─────────────────────────────────────────────────────────────────────

interface TracePoint {
  timestamp: string | number;
  latency: number;
  id?: string;
  [key: string]: any;
}

interface LatencyBandChartProps {
  data: TracePoint[];
  height?: number | string;
  className?: string;
  onTraceClick?: (id: string) => void;
}

// ── Custom Dot Component ──────────────────────────────────────────────────────

const LatencyDot = (props: any) => {
  const { cx, cy, payload } = props;
  const latency = payload.latency;
  
  let fill = 'var(--accent-cyan)';
  let opacity = 0.3;
  let r = 2;
  let filter = 'none';
  let cursor = 'default';

  if (latency >= 1500) {
    fill = 'var(--primary-rose)';
    opacity = 1;
    r = 4;
    filter = 'url(#criticalGlow)';
    cursor = 'pointer';
  } else if (latency >= 500) {
    fill = 'var(--primary-amber)';
    opacity = 0.7;
    r = 3;
  }

  return (
    <circle
      cx={cx}
      cy={cy}
      r={r}
      fill={fill}
      fillOpacity={opacity}
      style={{ filter, cursor, transition: 'all 0.2s ease' }}
      className={latency >= 1500 ? 'hover:scale-125' : ''}
    />
  );
};

// ── Component ─────────────────────────────────────────────────────────────────

export function LatencyBandChart({ 
  data, 
  height = '100%', 
  className = '',
  onTraceClick 
}: LatencyBandChartProps) {
  
  // Format Y-Axis: 100ms, 1s, 2s...
  const formatYAxis = (val: number) => {
    if (val >= 1000) return `${(val / 1000).toFixed(1)}s`;
    return `${val}ms`;
  };

  // Format X-Axis: Time
  const formatXAxis = (val: any) => {
    const d = new Date(val);
    return isNaN(d.getTime()) ? val : d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  const handleDotClick = (data: any) => {
    if (data.latency >= 1500 && onTraceClick && data.id) {
      onTraceClick(data.id);
    }
  };

  return (
    <div style={{ height }} className={`w-full relative ${className}`}>
      <ResponsiveContainer width="100%" height="100%">
        <ScatterChart
          margin={{ top: 20, right: 20, bottom: 20, left: 10 }}
        >
          <defs>
            {/* The "Sparks" Glow Filter */}
            <filter id="criticalGlow" x="-50%" y="-50%" width="200%" height="200%">
              <feDropShadow
                dx="0"
                dy="0"
                stdDeviation="4"
                floodColor="var(--primary-rose)"
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
          </defs>

          <CartesianGrid 
            stroke="rgba(255, 255, 255, 0.03)" 
            vertical={false} 
            strokeDasharray="3 3"
          />

          <XAxis 
            dataKey="timestamp" 
            type="category" // Changed to category for time-series-like buckets if needed
            stroke="transparent" 
            tick={{ fill: 'var(--on-surface-muted)', fontSize: 10, fontFamily: 'JetBrains Mono' }}
            tickFormatter={formatXAxis}
            axisLine={false}
            tickLine={false}
            minTickGap={30}
          />

          <YAxis 
            dataKey="latency" 
            name="Latency"
            unit="ms"
            stroke="transparent" 
            tick={{ fill: 'var(--on-surface-muted)', fontSize: 10, fontFamily: 'JetBrains Mono' }}
            tickFormatter={formatYAxis}
            axisLine={false}
            tickLine={false}
            width={60}
          />

          <ZAxis type="number" range={[0, 0]} /> {/* Hide default size metric */}

          <Tooltip
            cursor={{ strokeDasharray: '3 3', stroke: 'rgba(255,255,255,0.1)' }}
            content={({ active, payload }) => {
              if (active && payload && payload.length) {
                const point = payload[0].payload;
                return (
                  <div className="bg-[#0A0A0A]/90 backdrop-blur-xl border border-white/5 p-2 rounded shadow-2xl">
                    <div className="flex flex-col gap-1">
                      <span className="font-jetbrains text-[9px] text-white/40 uppercase tracking-widest">
                        Telemetry Event
                      </span>
                      <div className="h-px w-full bg-white/10" />
                      <div className="flex items-center gap-3">
                        <span className="font-jetbrains text-[11px] text-white font-bold">
                          {point.latency}ms
                        </span>
                        <span className="font-jetbrains text-[10px] text-white/40">
                          {formatXAxis(point.timestamp)}
                        </span>
                      </div>
                      {point.latency >= 1500 && (
                        <span className="font-jetbrains text-[9px] text-rose-400 font-bold mt-1">
                          CRITICAL SPIKE — CLICK TO DEBUG
                        </span>
                      )}
                    </div>
                  </div>
                );
              }
              return null;
            }}
          />

          <Scatter 
            name="Latency Traces" 
            data={data} 
            shape={<LatencyDot />}
            onClick={handleDotClick}
            isAnimationActive={true}
            animationDuration={1000}
          />
        </ScatterChart>
      </ResponsiveContainer>
      
      {/* Ghost Legend HUD */}
      <div className="absolute top-4 right-4 flex items-center gap-4 opacity-40 pointer-events-none">
        <div className="flex items-center gap-1.5">
          <div className="w-1.5 h-1.5 rounded-full bg-[var(--accent-cyan)]" />
          <span className="font-geist text-[8px] font-bold uppercase tracking-widest text-white">Nominal</span>
        </div>
        <div className="flex items-center gap-1.5">
          <div className="w-1.5 h-1.5 rounded-full bg-[var(--primary-amber)]" />
          <span className="font-geist text-[8px] font-bold uppercase tracking-widest text-white">Congestion</span>
        </div>
        <div className="flex items-center gap-1.5">
          <div className="w-1.5 h-1.5 rounded-full bg-[var(--primary-rose)] shadow-[0_0_8px_var(--primary-rose)]" />
          <span className="font-geist text-[8px] font-bold uppercase tracking-widest text-white">Critical</span>
        </div>
      </div>
    </div>
  );
}
