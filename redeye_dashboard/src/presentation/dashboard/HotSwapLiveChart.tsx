import { useEffect, useState } from 'react';
import {
  Area,
  AreaChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
  CartesianGrid,
} from 'recharts';
import { motion, AnimatePresence } from 'framer-motion';
import { Zap, ShieldAlert } from 'lucide-react';

interface HotSwapMetric {
  time: string;
  openai_success: number;
  openai_error: number;
  anthropic_fallback: number;
}

const fetcher = async (url: string) => {
  const res = await fetch(url, { 
    credentials: 'include',
    headers: { 'Content-Type': 'application/json', 'x-csrf-token': '1' }
  });
  if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
  return res.json();
};

export const HotSwapLiveChart = () => {
  const [data, setData] = useState<HotSwapMetric[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [armed, setArmed] = useState(false);
  const [firing, setFiring] = useState(false);

  const HOT_SWAPS_URL = 'http://localhost:8080/v1/admin/metrics/hot-swaps';

  useEffect(() => {
    let intervalId: ReturnType<typeof setInterval>;

    const fetchAndSchedule = async () => {
      try {
        const json = await fetcher(HOT_SWAPS_URL);
        setData(json);
        setError(null);
      } catch (err: unknown) {
        const message: string = err instanceof Error ? err.message : 'An unknown error occurred.';
        setError(message);
        if (message.includes('401') || message.includes('Unauthorized')) {
          clearInterval(intervalId);
        }
      } finally {
        setLoading(false);
      }
    };

    fetchAndSchedule();
    intervalId = setInterval(fetchAndSchedule, 2000);
    return () => clearInterval(intervalId);
  }, []);

  const handleTriggerChaos = async () => {
    if (!armed) return;
    setFiring(true);
    try {
      await fetch('http://localhost:8080/v1/admin/toggle-chaos', {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json', 'x-csrf-token': '1' }
      });
    } catch (e) {
      console.error('Failed to trigger chaos', e);
    } finally {
      setTimeout(() => setFiring(false), 600);
    }
  };

  return (
    <div
      className="w-full relative overflow-hidden rounded-2xl"
      style={{
        background: 'var(--glass-bg)',
        border: '1px solid var(--glass-border)',
        backdropFilter: 'blur(40px)',
        WebkitBackdropFilter: 'blur(40px)',
        boxShadow: armed
          ? '0 0 50px -10px rgba(245, 158, 11, 0.25), 0 20px 60px -12px rgba(0,0,0,0.5)'
          : 'var(--glass-shadow)',
        transition: 'box-shadow 0.4s cubic-bezier(0.4,0,0.2,1)',
      }}
    >
      {/* ── Tactical warning stripes — behind chart only ─────── */}
      <div
        className="absolute top-0 right-0 w-64 h-full pointer-events-none z-0 opacity-[0.04] dark:opacity-[0.06]"
        style={{
          background: 'repeating-linear-gradient(45deg, transparent, transparent 12px, #f59e0b 12px, #f59e0b 24px)',
        }}
      />

      <div className="relative z-10 flex flex-col h-full p-8">
        {/* ── Header ── */}
        <div className="flex items-start justify-between mb-6 gap-4">
          <div className="flex-1">
            <h3 className="text-sm font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--canvas-text)' }}>
              Zero-Downtime Hot-Swaps
            </h3>
            <p className="text-xs font-mono mt-1" style={{ color: 'var(--canvas-text)', opacity: 0.4 }}>
              Real-time multi-LLM routing telemetry · 2s interval
            </p>
          </div>

          {/* Status */}
          <div className="flex items-center gap-2 flex-shrink-0">
            {error && <span className="text-[10px] uppercase tracking-widest text-rose-500 font-bold">Offline</span>}
            {loading && !error && <span className="text-[10px] uppercase tracking-widest font-bold animate-pulse" style={{ color: 'var(--canvas-text)', opacity: 0.4 }}>Connecting...</span>}
            {!error && !loading && (
              <div className="flex items-center gap-2">
                <span className="relative flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
                </span>
                <span className="text-[10px] uppercase tracking-widest font-bold text-emerald-500">Live</span>
              </div>
            )}
          </div>
        </div>

        {/* ── Chart ── */}
        <div className="w-full h-64 flex flex-col flex-shrink-0">
          {data.length === 0 && !loading && !error && (
            <div className="h-full flex items-center justify-center text-xs font-mono" style={{ color: 'var(--canvas-text)', opacity: 0.3 }}>
              No telemetry data available for the last hour.
            </div>
          )}
          <div className="relative flex-1 w-full h-full min-h-0">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={data} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
                <defs>
                  <linearGradient id="gradSuccess" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#22d3ee" stopOpacity={0.5} />
                    <stop offset="95%" stopColor="#22d3ee" stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="gradFallback" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#10b981" stopOpacity={0.5} />
                    <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="gradError" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#ef4444" stopOpacity={0.7} />
                    <stop offset="95%" stopColor="#ef4444" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid stroke="#ffffff" strokeOpacity={0.03} vertical={false} />
                <XAxis dataKey="time" stroke="#64748b" fontSize={10} tickLine={false} axisLine={false} minTickGap={20} />
                <YAxis stroke="#64748b" fontSize={10} tickLine={false} axisLine={false} tickFormatter={v => `${v}`} />
                <Tooltip
                  contentStyle={{ backgroundColor: 'rgba(5,5,5,0.9)', border: '1px solid rgba(255,255,255,0.08)', borderRadius: '10px', color: '#f8fafc', boxShadow: '0 10px 40px rgba(0,0,0,0.5)', backdropFilter: 'blur(12px)' }}
                  itemStyle={{ fontFamily: 'monospace', fontSize: '12px' }}
                  labelStyle={{ color: '#64748b', fontSize: '10px', marginBottom: '4px', fontWeight: 600 }}
                />
                <Area type="monotone" dataKey="openai_success"     name="OpenAI (Normal)"      stroke="#22d3ee" strokeWidth={2} fillOpacity={1} fill="url(#gradSuccess)"  isAnimationActive={false} />
                <Area type="monotone" dataKey="anthropic_fallback" name="Anthropic (Hot-Swap)"  stroke="#10b981" strokeWidth={2.5} fillOpacity={1} fill="url(#gradFallback)" isAnimationActive={false} />
                <Area type="monotone" dataKey="openai_error"       name="OpenAI (Failed)"       stroke="#ef4444" strokeWidth={2} fillOpacity={1} fill="url(#gradError)"    isAnimationActive={false} />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* ── Legend + Tactical Panel ── */}
        <div className="mt-6 flex flex-wrap items-end gap-6 justify-between">
          {/* Legend */}
          <div className="flex flex-wrap gap-3 text-[10px] font-medium">
            {[
              { color: '#22d3ee', label: 'OpenAI · Normal' },
              { color: '#10b981', label: 'Anthropic · Hot-Swapped' },
              { color: '#ef4444', label: 'OpenAI · Failed' },
            ].map(({ color, label }) => (
              <div
                key={label}
                className="flex items-center gap-2 px-3 py-1.5 rounded-full"
                style={{ background: 'var(--glass-bg)', border: '1px solid var(--glass-border)' }}
              >
                <div className="w-2 h-2 rounded-full flex-shrink-0" style={{ backgroundColor: color, boxShadow: `0 0 6px ${color}88` }} />
                <span className="uppercase tracking-widest" style={{ color: 'var(--canvas-text)', opacity: 0.6 }}>{label}</span>
              </div>
            ))}
          </div>

          {/* ── Tactical Command Block ── */}
          <div
            className="flex items-center gap-4 rounded-xl p-4 relative overflow-hidden"
            style={{
              background: armed
                ? 'rgba(245, 158, 11, 0.08)'
                : 'var(--glass-bg)',
              border: `1px solid ${armed ? 'rgba(245, 158, 11, 0.3)' : 'var(--glass-border)'}`,
              transition: 'all 0.3s cubic-bezier(0.4,0,0.2,1)',
            }}
          >
            {/* Stripes overlay (always present, more visible when armed) */}
            <div
              className="absolute inset-0 pointer-events-none"
              style={{
                background: 'repeating-linear-gradient(45deg, transparent, transparent 8px, rgba(245, 158, 11, 1) 8px, rgba(245, 158, 11, 1) 16px)',
                opacity: armed ? 0.08 : 0.02,
                transition: 'opacity 0.3s',
              }}
            />

            {/* Armed Toggle */}
            <div className="flex items-center gap-2 relative z-10">
              <ShieldAlert className="w-3.5 h-3.5 text-amber-500 flex-shrink-0" />
              <span className="text-[9px] uppercase tracking-[0.2em] font-bold text-amber-500">Arm</span>
              <button
                onClick={() => setArmed(a => !a)}
                className="relative w-10 h-5 rounded-full transition-all duration-300 focus:outline-none"
                style={{
                  background: armed ? 'rgba(245,158,11,0.8)' : 'rgba(100,116,139,0.3)',
                  boxShadow: armed ? '0 0 12px rgba(245,158,11,0.5)' : 'none',
                }}
                aria-label="Arm chaos switch"
              >
                <motion.div
                  className="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white"
                  animate={{ x: armed ? 20 : 0 }}
                  transition={{ type: 'spring', stiffness: 500, damping: 30 }}
                  style={{ boxShadow: '0 1px 4px rgba(0,0,0,0.3)' }}
                />
              </button>
            </div>

            {/* Armed Indicator */}
            <AnimatePresence>
              {armed && (
                <motion.div
                  initial={{ opacity: 0, scale: 0.8 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.8 }}
                  className="flex items-center gap-1.5 relative z-10"
                >
                  <div className="w-2 h-2 rounded-full bg-amber-400 animate-spatial-glow" style={{ boxShadow: '0 0 8px rgba(245,158,11,0.8)' }} />
                  <span className="text-[9px] uppercase tracking-[0.2em] font-bold text-amber-400">Armed</span>
                </motion.div>
              )}
            </AnimatePresence>

            {/* Fire Button */}
            <button
              onClick={handleTriggerChaos}
              disabled={!armed}
              className={`relative z-10 flex items-center gap-2 px-4 py-2 rounded-lg font-bold uppercase tracking-[0.12em] text-[10px] transition-all duration-200 focus:outline-none ${
                armed
                  ? 'active:translate-y-0.5 active:scale-[0.98]'
                  : 'cursor-not-allowed'
              }`}
              style={{
                background: armed ? '#f59e0b' : 'rgba(100,116,139,0.15)',
                color: armed ? '#050505' : 'rgba(100,116,139,0.5)',
                boxShadow: armed
                  ? 'inset 0 1px 0 rgba(255,255,255,0.35), inset 0 -2px 0 rgba(0,0,0,0.2), 0 8px 16px rgba(245,158,11,0.35)'
                  : 'none',
                transition: 'all 0.2s cubic-bezier(0.4,0,0.2,1)',
              }}
            >
              <Zap className={`w-3.5 h-3.5 ${armed ? 'fill-current' : ''}`} />
              {firing ? 'Firing…' : 'Trigger Outage'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default HotSwapLiveChart;
